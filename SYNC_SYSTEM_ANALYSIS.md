# Trilium Notes Synchronization System - Comprehensive Analysis

## Executive Summary

Trilium Notes implements a sophisticated multi-instance synchronization system that allows multiple clients to maintain identical copies of a document database while handling conflicts gracefully. The system uses a timestamp-based conflict resolution strategy, sectored hash verification, and a pull-first architecture with dual-pass protocol.

---

## 1. COMPLETE SYNC FLOW

### 1.1 High-Level Sync Process

The sync process follows a carefully orchestrated sequence (sync.ts lines 51-72):

```
SYNC CYCLE:
1. login() → Authenticate with sync server
2. pushChanges() → Send local changes to server (first pass)
3. pullChanges() → Receive remote changes from server
4. pushChanges() → Send any new changes triggered by pulls (second pass)
5. syncFinished() → Notify server sync completed
6. checkContentHash() → Verify data consistency
   └─ If conflicts found, queue sectors and repeat from step 2
```

**Key Insight**: The dual-push architecture (push → pull → push) ensures that:
- Local changes are sent first (important for timestamp ordering)
- Pulled changes are immediately available for re-pushing if newer
- Prevents race conditions where pulling creates new unsynced changes

### 1.2 Sync Initiation

**Automatic**: Every 60 seconds (sync.ts line 450)
```typescript
setInterval(cls.wrap(sync), 60000);  // Every 60 seconds
setTimeout(cls.wrap(sync), 5000);     // Initial sync after 5 seconds
```

**Manual**: Via API endpoint `/api/sync/now` (routes.ts line 228)

**Exclusive Access**: Uses mutex to prevent concurrent sync operations
```typescript
await syncMutexService.doExclusively(async () => {
    // Only one sync process runs at a time
});
```

---

## 2. ENTITY CHANGES TRACKING MECHANISM

### 2.1 Entity Changes Table Structure

**Location**: entity_changes table (schema.sql lines 1-12)

```sql
CREATE TABLE "entity_changes" (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    entityName      TEXT NOT NULL,           -- Type: notes, branches, attributes, etc.
    entityId        TEXT NOT NULL,           -- Primary key of the entity
    hash            TEXT NOT NULL,           -- Cryptographic hash of entity state
    isErased        INT NOT NULL,            -- Soft delete marker (1 = deleted)
    changeId        TEXT NOT NULL,           -- Unique change identifier (12-char random)
    componentId     TEXT NOT NULL,           -- Component that made the change
    instanceId      TEXT NOT NULL,           -- Source instance ID (12-char random)
    isSynced        INTEGER NOT NULL,        -- Sync flag (0 = local only, 1 = syncable)
    utcDateChanged  TEXT NOT NULL            -- ISO datetime of change
);

CREATE UNIQUE INDEX IDX_entityChanges_entityName_entityId 
    ON entity_changes(entityName, entityId);
CREATE INDEX IDX_entity_changes_changeId ON entity_changes(changeId);
```

### 2.2 Hash Calculation Mechanism

**Entity Hash Generation** (abstract_becca_entity.ts lines 63-76):

```typescript
generateHash(isDeleted?: boolean): string {
    const constructorData = this.constructor as ConstructorData<T>;
    let contentToHash = "";
    
    // Concatenate all "hashed properties" defined for the entity type
    for (const propertyName of constructorData.hashedProperties) {
        contentToHash += `|${this[propertyName]}`;
    }
    
    // Append deletion marker
    if (isDeleted) {
        contentToHash += "|deleted";
    }
    
    // Return first 10 characters of SHA-256 hash
    return utils.hash(contentToHash).substr(0, 10);
}
```

**Hashed Properties by Entity Type**:
- **notes**: noteId, title, isProtected, type, mime, blobId, isDeleted
- **branches**: branchId, noteId, parentNoteId, notePosition, isDeleted
- **attributes**: attributeId, noteId, type, name, value, isDeleted
- **revisions**: revisionId, noteId, title, isProtected, blobId
- **attachments**: attachmentId, ownerId, role, mime, title, isProtected, blobId, isDeleted

**Blob Hash Calculation** (abstract_becca_entity.ts lines 204-212, blob.ts):

```typescript
// For blobs (content), use encrypted prefix for protected content
const encryptedPrefixSuffix = "t$[nvQg7q)&_ENCRYPTED_?M:Bf&j3jr_";

// Concatenate prefix + content
const unencryptedContentForHashCalculation = 
    isProtected 
        ? Buffer.concat([Buffer.from(encryptedPrefixSuffix), content])
        : content;

// Use content hash as blob ID (content-addressable storage)
const newBlobId = utils.hashedBlobId(unencryptedContentForHashCalculation);
```

**Sector-Based Hash Verification** (content_hash.ts):

The system partitions entities into "sectors" based on the first character of entityId:

```typescript
function getEntityHashes() {
    // Get all entity changes grouped by entity type and sector (first char of entityId)
    const hashMap = {};
    
    for (const [entityName, entityId, hash, isErased] of hashRows) {
        const sector = entityId[0];  // First character is the sector
        
        // Concatenate: hash + isErased status
        entityHashMap[sector] += hash + isErased;
    }
    
    // Hash the concatenated string
    for (const key in entityHashMap) {
        entityHashMap[key] = hash(entityHashMap[key]);
    }
}
```

**Why Sectors?**: Allows localized sync recovery without full resync. If sector 'a' of notes differs, only that sector is re-queued.

### 2.3 Change Detection Algorithm

**When Changes Are Created** (entity_changes.ts lines 15-47):

```typescript
function putEntityChange(entityChange: EntityChange) {
    const ec = { ...entityChange };
    
    // If no changeId provided, generate unique one
    if (!ec.changeId) {
        ec.changeId = randomString(12);  // Unique per change
    }
    
    // Assign component and instance IDs
    ec.componentId = ec.componentId || cls.getComponentId() || "NA";
    ec.instanceId = ec.instanceId || instanceId;  // Current instance
    
    // Mark as synced/not synced
    ec.isSynced = ec.isSynced ? 1 : 0;
    ec.isErased = ec.isErased ? 1 : 0;
    
    // Insert into entity_changes table
    ec.id = sql.replace("entity_changes", ec);
    
    if (ec.id) {
        maxEntityChangeId = Math.max(maxEntityChangeId, ec.id);
    }
}
```

**Note Reordering Special Case** (entity_changes.ts lines 49-65):

```typescript
function putNoteReorderingEntityChange(parentNoteId: string, componentId?: string) {
    putEntityChange({
        entityName: "note_reordering",
        entityId: parentNoteId,
        hash: "N/A",  // Not hashed; positions are tracked separately
        isErased: false,
        utcDateChanged: dateUtils.utcNowDateTime(),
        isSynced: true,
        componentId
    });
    
    // Also emit event with actual position data
    eventService.emit(eventService.ENTITY_CHANGED, {
        entityName: "note_reordering",
        entity: sql.getMap(`SELECT branchId, notePosition FROM branches WHERE parentNoteId = ?`, 
            [parentNoteId])
    });
}
```

---

## 3. SYNC TABLES AND DATABASE SCHEMA

### 3.1 Core Sync-Related Tables

**entity_changes** (primary sync table)
- Immutable log of all changes
- Used for both local tracking and sync protocol
- Indexed for: (entityName, entityId), changeId

**options** (configuration)
- `lastSyncedPull`: Last entity_changes.id received from server
- `lastSyncedPush`: Last entity_changes.id sent to server
- `documentSecret`: HMAC key for sync authentication
- `documentId`: UUID identifying this document
- `syncServerHost`: URL of sync server
- `syncServerTimeout`: Request timeout (default 120000ms)
- `syncProxy`: Optional HTTP proxy
- `syncVersion`: Compatibility marker (appInfo.syncVersion)

**Other Entities with Sync Support**
- notes, branches, attributes, revisions, attachments, blobs, etapi_tokens
- All linked to entity_changes via (entityName, entityId) pair

### 3.2 Sync State Tracking

**Options Governing Sync State**:

```typescript
// From sync.ts
function getLastSyncedPull() {
    return parseInt(optionService.getOption("lastSyncedPull"));
}

function getLastSyncedPush() {
    const lastSyncedPush = parseInt(optionService.getOption("lastSyncedPush"));
    ws.setLastSyncedPush(lastSyncedPush);  // Notify frontend
    return lastSyncedPush;
}
```

**Sync Status Indicators**:
- `id > lastSyncedPull`: Changes not yet pulled from server
- `id > lastSyncedPush`: Changes not yet pushed to server
- `isSynced = 1`: Eligible for sync (not local-only options)
- `instanceId = current`: Changes from this instance
- `changeId`: Idempotency token (prevents duplicate application)

---

## 4. SYNC PROTOCOL ARCHITECTURE

### 4.1 Push Mechanism (Client → Server)

**Endpoint**: `PUT /api/sync/update`

**Flow** (sync.ts lines 203-255):

```typescript
async function pushChanges(syncContext: SyncContext) {
    let lastSyncedPush = getLastSyncedPush();
    
    while (true) {
        // Get unsynced changes in batches of 1000
        const entityChanges = sql.getRows<EntityChange>(
            "SELECT * FROM entity_changes WHERE isSynced = 1 AND id > ? LIMIT 1000",
            [lastSyncedPush]
        );
        
        if (entityChanges.length === 0) break;
        
        // Filter out changes from the server (prevent echo)
        const filteredEntityChanges = entityChanges.filter((entityChange) => {
            if (entityChange.instanceId === syncContext.instanceId) {
                // This came from the server, skip it
                lastSyncedPush = entityChange.id;
                return false;
            }
            return true;
        });
        
        // Convert to records with entity data
        const entityChangeRecords = getEntityChangeRecords(filteredEntityChanges);
        
        // Send request (with ~1MB size limit per request)
        await syncRequest(syncContext, "PUT", `/api/sync/update?logMarkerId=${logMarkerId}`, {
            entities: entityChangeRecords,
            instanceId
        });
        
        // Update lastSyncedPush
        lastSyncedPush = entityChangeRecords[entityChangeRecords.length - 1].entityChange.id;
        setLastSyncedPush(lastSyncedPush);
    }
}
```

**Request Format**:

```json
{
  "instanceId": "abc123def456",
  "entities": [
    {
      "entityChange": {
        "id": 1000,
        "entityName": "notes",
        "entityId": "note123",
        "hash": "abc1234567",
        "isErased": 0,
        "changeId": "change001",
        "componentId": "comp1",
        "instanceId": "instance1",
        "isSynced": 1,
        "utcDateChanged": "2024-11-11T10:30:00Z"
      },
      "entity": {
        "noteId": "note123",
        "title": "My Note",
        "type": "text",
        "mime": "text/html",
        "blobId": "blob456",
        "isDeleted": 0,
        ...
      }
    },
    ...
  ]
}
```

**Size Limits**: ~1MB per request (sync.ts line 394-396)
- Automatically paginated for larger payloads
- Paging handled via headers: pageCount, pageIndex, requestId

### 4.2 Pull Mechanism (Server → Client)

**Endpoint**: `GET /api/sync/changed?instanceId=X&lastEntityChangeId=Y`

**Flow** (sync.ts lines 156-201):

```typescript
async function pullChanges(syncContext: SyncContext) {
    while (true) {
        const lastSyncedPull = getLastSyncedPull();
        const changesUri = `/api/sync/changed?instanceId=${instanceId}&lastEntityChangeId=${lastSyncedPull}&logMarkerId=${logMarkerId}`;
        
        const resp = await syncRequest<ChangesResponse>(syncContext, "GET", changesUri);
        const { entityChanges, lastEntityChangeId, outstandingPullCount } = resp;
        
        // Apply changes atomically
        sql.transactional(() => {
            syncUpdateService.updateEntities(entityChanges, syncContext.instanceId);
            
            if (lastSyncedPull !== lastEntityChangeId) {
                setLastSyncedPull(lastEntityChangeId);
            }
        });
        
        // If no changes returned, we're up-to-date
        if (entityChanges.length === 0) break;
    }
}
```

**Response Format**:

```json
{
  "entityChanges": [
    {
      "entityChange": { ... },
      "entity": { ... }
    }
  ],
  "lastEntityChangeId": 2500,
  "outstandingPullCount": 150
}
```

**Server-Side Processing** (routes/api/sync.ts lines 142-197):

```typescript
function getChanged(req: Request) {
    let lastEntityChangeId = parseInt(req.query.lastEntityChangeId);
    const clientInstanceId = req.query.instanceId;
    let filteredEntityChanges = [];
    
    do {
        // Get changes in batches of 1000
        const entityChanges = sql.getRows<EntityChange>(
            `SELECT * FROM entity_changes 
             WHERE isSynced = 1 AND id > ? 
             ORDER BY id LIMIT 1000`,
            [lastEntityChangeId]
        );
        
        if (entityChanges.length === 0) break;
        
        // Filter: only return changes NOT from the requesting client
        filteredEntityChanges = entityChanges.filter(
            (ec) => ec.instanceId !== clientInstanceId
        );
        
        if (filteredEntityChanges.length === 0) {
            // All changes are from client, skip them
            lastEntityChangeId = entityChanges[entityChanges.length - 1].id;
        }
    } while (filteredEntityChanges.length === 0);
    
    // Convert to records with entity data
    const entityChangeRecords = syncService.getEntityChangeRecords(filteredEntityChanges);
    
    return {
        entityChanges: entityChangeRecords,
        lastEntityChangeId,
        outstandingPullCount: sql.getValue(
            `SELECT COUNT(id) FROM entity_changes 
             WHERE isSynced = 1 AND instanceId != ? AND id > ?`,
            [clientInstanceId, lastEntityChangeId]
        )
    };
}
```

### 4.3 Conflict Resolution Strategy

**Timestamp-Based Resolution** (sync_update.ts lines 74-113):

```typescript
function updateNormalEntity(remoteEC: EntityChange, remoteEntityRow: EntityRow, 
                           instanceId: string, updateContext: UpdateContext) {
    // Get local version of the same entity
    const localEC = sql.getRow<EntityChange>(
        `SELECT * FROM entity_changes WHERE entityName = ? AND entityId = ?`,
        [remoteEC.entityName, remoteEC.entityId]
    );
    
    // Check if remote is newer or equal
    const localECIsOlderOrSameAsRemote = 
        localEC && 
        localEC.utcDateChanged && 
        remoteEC.utcDateChanged && 
        localEC.utcDateChanged <= remoteEC.utcDateChanged;
    
    // CASE 1: No local change OR remote is newer
    if (!localEC || localECIsOlderOrSameAsRemote) {
        if (remoteEC.isErased) {
            // Remote deletion wins
            eraseEntity(remoteEC);
            updateContext.erased++;
        } else {
            // Remote update wins
            sql.replace(remoteEC.entityName, remoteEntityRow);
            updateContext.updated[remoteEC.entityName].push(remoteEC.entityId);
        }
        
        // Record the remote change locally
        entityChangesService.putEntityChangeWithInstanceId(remoteEC, instanceId);
        return true;
    } 
    // CASE 2: Local is newer
    else if ((localEC.hash !== remoteEC.hash || localEC.isErased !== remoteEC.isErased) 
             && !localECIsOlderOrSameAsRemote) {
        // Local change is newer, re-queue it for pushing
        entityChangesService.putEntityChangeForOtherInstances(localEC);
        return false;
    }
    
    return false;
}
```

**Conflict Resolution Rules**:

| Scenario | Local Time | Remote Time | Winner | Action |
|----------|-----------|-------------|--------|--------|
| No local change | - | any | Remote | Apply remote |
| Same timestamp | T1 | T1 | Remote | Apply remote (deterministic) |
| Local newer | T2 | T1 (T2 > T1) | Local | Queue for re-push |
| Remote newer | T1 | T2 (T2 > T1) | Remote | Apply remote |
| Already erased | - | - | N/A | No-op (idempotent) |

**Why Timestamp-Based?**
- No leader election needed (works in multi-master setup)
- Deterministic and reproducible on all instances
- Handles clock skew gracefully
- Works with offline instances (eventual consistency)

### 4.4 Entity Ordering and Dependencies

**Referential Ordering** (ws.ts lines 175-186):

```typescript
const ORDERING: Record<string, number> = {
    etapi_tokens: 0,    // No dependencies
    attributes: 2,      // Can reference notes
    branches: 2,        // Reference notes
    blobs: 0,           // No dependencies
    note_reordering: 2, // References notes
    revisions: 2,       // Reference notes
    attachments: 3,     // Reference notes and blobs
    notes: 1,           // Might reference blobs
    options: 0          // No dependencies
};
```

**Why Ordering Matters**:

Froca (frontend cache) is incomplete - it can't create "skeleton" entities like Becca can. Therefore, referenced entities must exist before referencing entities are applied.

```typescript
// When sending updates to frontend
entityChanges.sort((a, b) => ORDERING[a.entityName] - ORDERING[b.entityName]);

// Now blobs (0) are applied before notes (1)
// Notes (1) are applied before branches (2)
// Branches (2) are applied before attachments (3)
```

### 4.5 Special Cases

**Note Reordering** (sync_update.ts lines 132-144):

```typescript
function updateNoteReordering(remoteEC: EntityChange, remoteEntityRow: EntityRow, 
                             instanceId: string) {
    // remoteEntityRow is a map: { branchId1: position1, branchId2: position2, ... }
    
    for (const key in remoteEntityRow) {
        sql.execute("UPDATE branches SET notePosition = ? WHERE branchId = ?",
            [remoteEntityRow[key], key]);
    }
    
    entityChangesService.putEntityChangeWithInstanceId(remoteEC, instanceId);
    return true;
}
```

**Erased Entities** (sync_update.ts lines 147-160):

```typescript
function eraseEntity(entityChange: EntityChange) {
    const { entityName, entityId } = entityChange;
    
    const entityNames = ["notes", "branches", "attributes", "revisions", "attachments", "blobs"];
    
    if (!entityNames.includes(entityName)) {
        log.error(`Cannot erase ${entityName} '${entityId}'.`);
        return;
    }
    
    // Physical deletion from database
    const primaryKeyName = entityConstructor.getEntityFromEntityName(entityName).primaryKeyName;
    sql.execute(`DELETE FROM ${entityName} WHERE ${primaryKeyName} = ?`, [entityId]);
}
```

---

## 5. WEBSOCKET REAL-TIME SYNCHRONIZATION

### 5.1 WebSocket Architecture

**Initialization** (ws.ts lines 19-61):

```typescript
function init(httpServer: HttpServer, sessionParser: SessionParser) {
    webSocketServer = new WebSocketServer({
        verifyClient: (info, done) => {
            sessionParser(info.req, {}, () => {
                // Verify: either electron, logged-in session, or no auth required
                const allowed = isElectron || 
                               (info.req as any).session.loggedIn || 
                               (config.General && config.General.noAuthentication);
                
                done(allowed);
            });
        },
        server: httpServer
    });
    
    // Handle incoming messages from clients
    webSocketServer.on("connection", (ws, req) => {
        ws.on("message", async (messageJson) => {
            const message = JSON.parse(messageJson as any);
            
            if (message.type === "log-error") {
                // Log JS errors from frontend
                log.info(`JS Error: ${message.error}`);
            } else if (message.type === "ping") {
                // Client keepalive
                await syncMutexService.doExclusively(() => sendPing(ws));
            }
        });
    });
}
```

### 5.2 Message Types

**Sent by Server to Clients**:

```typescript
// Sync lifecycle messages
{ type: "sync-pull-in-progress", lastSyncedPush: 1500 }
{ type: "sync-push-in-progress", lastSyncedPush: 1500 }
{ type: "sync-finished", lastSyncedPush: 1500 }
{ type: "sync-failed", lastSyncedPush: 1500 }

// Frontend update with entity changes
{
    type: "frontend-update",
    data: {
        lastSyncedPush: 1500,
        entityChanges: [
            {
                entityChange: { id: 1000, entityName: "notes", ... },
                entity: { noteId: "abc", title: "...", ... }
            },
            ...
        ]
    }
}

// UI reload directive
{ type: "reload-frontend", reason: "schema-update" }

// Session change
{ type: "protectedSessionLogin" }
```

### 5.3 Entity Change Propagation

**Transaction-Based Broadcasting** (ws.ts lines 222-228):

```typescript
function sendTransactionEntityChangesToAllClients() {
    if (webSocketServer) {
        const entityChangeIds = cls.getAndClearEntityChangeIds();
        
        webSocketServer.clients.forEach((client) => 
            sendPing(client, entityChangeIds)
        );
    }
}
```

**Flow**:
1. Database transaction commits
2. Entity changes collected in CLS (Continuation Local Storage)
3. After transaction completes, changes broadcast to all connected clients
4. Clients update their Froca cache and UI

**Entity Enrichment** (ws.ts lines 97-173):

```typescript
function fillInAdditionalProperties(entityChange: EntityChange) {
    // Frontend needs extra data not in entity_changes table
    
    if (entityChange.entityName === "attributes") {
        entityChange.entity = becca.getAttribute(entityChange.entityId);
        if (!entityChange.entity) {
            entityChange.entity = sql.getRow(
                `SELECT * FROM attributes WHERE attributeId = ?`,
                [entityChange.entityId]
            );
        }
    } else if (entityChange.entityName === "notes") {
        entityChange.entity = becca.getNote(entityChange.entityId);
        if (!entityChange.entity) {
            entityChange.entity = sql.getRow(
                `SELECT * FROM notes WHERE noteId = ?`,
                [entityChange.entityId]
            );
            
            // Decrypt protected note titles
            if (entityChange.entity?.isProtected) {
                entityChange.entity.title = 
                    protectedSessionService.decryptString(entityChange.entity.title || "");
            }
        }
    } else if (entityChange.entityName === "note_reordering") {
        // Map branch IDs to positions
        entityChange.positions = {};
        const parentNote = becca.getNote(entityChange.entityId);
        
        if (parentNote) {
            for (const childBranch of parentNote.getChildBranches()) {
                entityChange.positions[childBranch.branchId] = childBranch.notePosition;
            }
        }
    }
}
```

---

## 6. SYNC AUTHENTICATION AND SECURITY

### 6.1 HMAC-Based Authentication

**Login Flow** (routes/api/login.ts lines 81-121):

```typescript
function loginSync(req: Request) {
    const timestampStr = req.body.timestamp;  // ISO 8601 datetime
    const timestamp = dateUtils.parseDateTime(timestampStr);
    const now = new Date();
    
    // CHECK 1: Timestamp validity (±5 minutes)
    if (Math.abs(timestamp.getTime() - now.getTime()) > 5 * 60 * 1000) {
        return [401, { message: "Auth request time is out of sync, ..." }];
    }
    
    // CHECK 2: Sync version compatibility
    const syncVersion = req.body.syncVersion;
    if (syncVersion !== appInfo.syncVersion) {
        return [400, { message: "Non-matching sync versions, ..." }];
    }
    
    // CHECK 3: HMAC verification
    const documentSecret = options.getOption("documentSecret");
    const expectedHash = utils.hmac(documentSecret, timestampStr);
    const givenHash = req.body.hash;
    
    if (expectedHash !== givenHash) {
        return [400, { message: "Sync login credentials are incorrect, ..." }];
    }
    
    req.session.loggedIn = true;
    
    return {
        instanceId: instanceId,
        maxEntityChangeId: sql.getValue("SELECT COALESCE(MAX(id), 0) FROM entity_changes WHERE isSynced = 1")
    };
}
```

**HMAC Calculation** (services/utils.ts):

```typescript
function hmac(secret: string, message: string): string {
    return crypto
        .createHmac('sha256', secret)
        .update(message)
        .digest('hex');
}
```

**Why This Design?**:
- No persistent session tokens (stateless)
- Timestamp prevents replay attacks
- HMAC proves knowledge of documentSecret
- No passwords transmitted

### 6.2 Protected Content Handling

**Protected Note Encryption** (abstract_becca_entity.ts lines 204-212):

```typescript
function getUnencryptedContentForHashCalculation(unencryptedContent: Buffer | string) {
    if (this.isProtected) {
        // Add random-looking prefix
        const encryptedPrefixSuffix = "t$[nvQg7q)&_ENCRYPTED_?M:Bf&j3jr_";
        
        return Buffer.isBuffer(unencryptedContent) 
            ? Buffer.concat([Buffer.from(encryptedPrefixSuffix), unencryptedContent])
            : `${encryptedPrefixSuffix}${unencryptedContent}`;
    } else {
        return unencryptedContent;
    }
}
```

**Effect**: Hash calculated on (prefix + content). This ensures:
- Same content encrypted differently still has same blobId
- Hash reveals content is encrypted (not plaintext)
- Deterministic content addressing across devices

### 6.3 Instance Identity and Fingerprints

**Instance ID** (instance_id.ts):

```typescript
const instanceId = randomString(12);
export default instanceId;
```

**Generation**: 12-character random string, generated once per server startup
**Purpose**: 
- Identify source of changes during sync
- Prevent echo: don't apply changes you sent
- Detect self-sync: reject if server instanceId == local instanceId

**Change ID** (entity_changes.ts lines 32-33):

```typescript
if (!ec.changeId) {
    ec.changeId = randomString(12);
}
```

**Purpose**: 
- Idempotency token
- Prevents duplicate application of same change
- Checked before applying pulled changes (sync_update.ts lines 29-35)

**Component ID** (entity_changes.ts line 36):

```typescript
ec.componentId = ec.componentId || cls.getComponentId() || "NA";
```

**Purpose**:
- Tracks which code component made the change
- Frontend uses to decide when to reload
- Can be overridden (e.g., force reload with random componentId)

---

## 7. API ENDPOINTS REFERENCE

### 7.1 Authentication Endpoint

**POST /api/login/sync**
- Purpose: Authenticate sync client with HMAC
- Request: `{ timestamp, hash, syncVersion }`
- Response: `{ instanceId, maxEntityChangeId }`
- Security: HMAC-SHA256(documentSecret, timestamp)
- Time Validation: ±5 minutes

### 7.2 Sync Data Endpoints

**GET /api/sync/changed?instanceId=X&lastEntityChangeId=Y&logMarkerId=Z**
- Purpose: Pull changes from server
- Query Parameters:
  - `instanceId`: Requesting client ID
  - `lastEntityChangeId`: Last change applied locally
  - `logMarkerId`: Request identifier for logging
- Response: `{ entityChanges[], lastEntityChangeId, outstandingPullCount }`
- Batch Size: ~1MB per request
- Filtering: Excludes changes from requesting instanceId

**PUT /api/sync/update?logMarkerId=Z**
- Purpose: Push changes to server
- Headers: pageCount, pageIndex, requestId (for pagination)
- Body: `{ instanceId, entities[] }`
- Process: Updates server database, records entity changes
- Pagination: Supports multi-part uploads for large payloads

**POST /api/sync/finished**
- Purpose: Signal end of sync cycle
- Effect: Marks application as initialized
- Called: After both push and pull complete

### 7.3 Verification and Control Endpoints

**GET /api/sync/check**
- Purpose: Content hash verification
- Response: `{ entityHashes: { entityName: { sector: hash }}, maxEntityChangeId }`
- Used: After sync to detect inconsistencies
- Comparison: Local hashes vs. server hashes by sector

**POST /api/sync/queue-sector/:entityName/:sector**
- Purpose: Re-queue specific sector for sync
- Used: When content hash mismatch detected
- Effect: Creates new entity_changes entries for all entities in sector

**POST /api/sync/check-entity-changes**
- Purpose: Run consistency checks on entity_changes table
- Used: Before re-queueing after hash failure

**GET /api/sync/stats**
- Purpose: Return sync statistics
- Response: `{ initialized, outstandingPullCount }`
- Usage: UI status display

### 7.4 Management Endpoints

**POST /api/sync/test**
- Purpose: Test sync configuration
- Effect: Attempts login and triggers sync
- Used: Configuration validation

**POST /api/sync/now**
- Purpose: Manually trigger sync
- Effect: Immediate sync cycle
- Used: User-initiated sync

**POST /api/sync/force-full-sync**
- Purpose: Reset sync state
- Effect: Sets lastSyncedPull=0, lastSyncedPush=0
- Used: Disaster recovery or debugging

**POST /api/sync/fill-entity-changes**
- Purpose: Populate entity_changes for existing entities
- Used: Recovery from corrupted entity_changes table

---

## 8. KEY ALGORITHMS AND DATA STRUCTURES

### 8.1 Sync State Machine

```
[IDLE] 
  ↓
[SYNCING] ←────────────────────────┐
  ├→ LOGIN                         │
  │   Authenticate with server    │
  │                               │
  ├→ PUSH (1st pass)             │
  │   Send local unsynced changes│
  │                               │
  ├→ PULL                        │
  │   Receive server changes      │
  │   Update local entities       │
  │                               │
  ├→ PUSH (2nd pass)             │
  │   Send changes triggered by  │
  │   pulled entities            │
  │                               │
  ├→ SYNC FINISHED              │
  │   Notify server sync done    │
  │                               │
  ├→ CHECK CONTENT HASH         │
  │   If PASS ──→ [IDLE]        │
  │   If FAIL ──→ (re-queue) ────┘
```

### 8.2 Conflict Resolution Decision Tree

```
Remote change arrives for entity E with timestamp TR, hash HR

├─ Does local E exist?
│  ├─ NO → Apply remote
│  │
│  └─ YES → Compare timestamps
│     ├─ TL < TR → Apply remote (remote is newer)
│     │
│     ├─ TL == TR → Apply remote (deterministic on tie)
│     │
│     └─ TL > TR → Keep local (local is newer)
│        └─ If (HL ≠ HR OR isErasedL ≠ isErasedR)
│           └─ Re-queue local for push
```

### 8.3 Entity Hash Composition

```
For entity with properties P1, P2, ..., Pn:

hash = SHA256("|P1|P2|...|Pn" + ["|deleted" if isDeleted])
return hash[0:10]  // First 10 characters

Sector hash = SHA256(concatenate all hashes + isErased status in sector)
```

### 8.4 Bloom Filter-like Sector Verification

```
Global state verification without checking every entity:

For each entityName:
  For each sector (first character):
    localHash[entityName][sector] = hash(all entities in sector)
    
    if localHash[entityName][sector] != serverHash[entityName][sector]:
        → Inconsistency found
        → Re-queue entire sector for sync
        → Server will compare and identify specific differing entities
```

### 8.5 CLS (Continuation Local Storage) for Transaction Tracking

```typescript
class CLS {
    private static entityChangeIds: number[] = [];
    
    static putEntityChange(ec: EntityChange) {
        this.entityChangeIds.push(ec.id);
    }
    
    static getAndClearEntityChangeIds(): number[] {
        const ids = this.entityChangeIds;
        this.entityChangeIds = [];
        return ids;
    }
}

// Usage in transaction
sql.transactional(() => {
    entity.save();  // Calls cls.putEntityChange()
    entity2.save(); // Accumulates in CLS
});  // On commit: broadcast changes via WebSocket
```

---

## 9. PERFORMANCE CONSIDERATIONS

### 9.1 Batching

- **Push**: 1000 changes per request (max ~1MB)
- **Pull**: Continuous loop until no changes
- **Hash Calculation**: Disables slow query logging during calculation

### 9.2 Indexing

```sql
CREATE UNIQUE INDEX IDX_entityChanges_entityName_entityId 
    ON entity_changes(entityName, entityId);
CREATE INDEX IDX_entity_changes_changeId ON entity_changes(changeId);
```

- First index: Quick lookup of latest change for any entity
- Second index: Change deduplication during pull

### 9.3 Network Optimization

- **Paging**: Large requests split into 1MB chunks with headers
- **Gzip**: (Implicit in HTTP)
- **Content Addressable**: Blobs use hash as ID, preventing duplicates

---

## 10. SECURITY MATRIX

| Threat | Mitigation | Mechanism |
|--------|-----------|-----------|
| Unauthorized sync | HMAC authentication | documentSecret HMAC |
| Replay attacks | Timestamp validation | ±5 minute window |
| Session hijacking | Stateless auth | No session tokens |
| Version mismatch | Protocol checking | syncVersion comparison |
| Self-sync | Instance identity | Check instanceId match |
| Duplicate changes | Idempotency tokens | changeId deduplication |
| Eavesdropping | HTTPS required | Protocol-level |
| Protected content | Encryption prefix | Content-dependent hash |
| Conflict corruption | Last-writer-wins | Timestamp ordering |

---

## 11. TROUBLESHOOTING GUIDE

### Sync Failures

1. **"Auth request time is out of sync"**
   - Solution: Sync system clocks (NTP)
   - Tolerance: ±5 minutes

2. **"Non-matching sync versions"**
   - Solution: Upgrade all instances to same version
   - Check: appInfo.syncVersion

3. **"No connection to sync server"**
   - Solution: Check network connectivity
   - Check: syncServerHost option
   - Check: Proxy settings if configured

4. **Content hash mismatches**
   - Cause: Entity corruption or sync race condition
   - Recovery: Automatic sector re-sync
   - Manual: `/api/sync/queue-sector/{entityName}/{sector}`

### Debugging

- **Log Marker**: Every sync includes logMarkerId for correlating client/server logs
- **Metrics**: /api/sync/stats shows outstandingPullCount
- **Hash Check**: /api/sync/check returns sector hashes for verification
- **Consistency**: Content hash computed without slow query logging

---

## 12. EXAMPLE SYNC CYCLE

```
T0: Client A makes change to Note 123
    → entity_changes record created (id=1000)
    → isSynced=1, instanceId=abc123

T5: Auto-sync triggers
    → Login to server (HMAC with documentSecret)
    → Push: Find id > lastSyncedPush (e.g., 999)
            Send change #1000
            Server stores in its entity_changes
    → Pull: Request id > lastSyncedPull (e.g., 2500)
            Server returns changes #2501-2600 (from other instances)
            Apply to local database
            Update lastSyncedPull=2600
    → Push: Send any new changes triggered by pull
    → Check: Calculate content hashes, compare with server
            If match → Sync complete
            If mismatch → Queue sector, loop back

T10: Client B connects
    → Pull: Gets changes #1000-2600
    → Last-writer-wins applied by timestamps
    → Local database now identical to A and Server
    → All see same Note 123 version
```

