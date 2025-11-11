# Trilium Notes Synchronization System - Exploration Index

## Overview

This index provides navigation to the comprehensive exploration of Trilium's synchronization system, including complete code analysis, architecture diagrams, and implementation details.

---

## Documentation Files

### 1. **SYNC_SYSTEM_ANALYSIS.md** (35 KB, 1103 lines)
   - **Most Comprehensive**: Complete technical analysis
   - **Sections**:
     - Complete sync flow (push/pull mechanism)
     - Entity changes tracking mechanism
     - Sync tables and database schema
     - Sync protocol architecture (push, pull, conflict resolution)
     - WebSocket real-time synchronization
     - Authentication and security
     - API endpoints reference
     - Key algorithms and data structures
     - Performance considerations
     - Security matrix
     - Troubleshooting guide
     - Example sync cycle
   - **Best for**: In-depth understanding of every component

### 2. **SYNC_ANALYSIS_SUMMARY.txt** (11 KB, ~400 lines)
   - **Quick Reference**: Executive summary
   - **Sections**:
     - Key findings summary
     - Critical algorithms
     - Entity dependencies and ordering
     - Sync state options
     - API endpoint summary
     - Dataflow diagram
     - Idempotency and deduplication
   - **Best for**: Getting up to speed quickly

---

## Source Code Files Referenced

### Core Synchronization Services

| File | Lines | Key Content |
|------|-------|-------------|
| `/apps/server/src/services/sync.ts` | 466 | Main orchestration: login, push, pull, hash checks |
| `/apps/server/src/services/sync_update.ts` | 171 | Conflict resolution: timestamp-based LWW |
| `/apps/server/src/services/entity_changes.ts` | 209 | Change tracking and hash generation |
| `/apps/server/src/services/content_hash.ts` | 92 | Sector-based hash verification |
| `/apps/server/src/services/sync_options.ts` | 35 | Configuration management |
| `/apps/server/src/services/sync_mutex.ts` | 22 | Exclusive sync cycle guarantee |
| `/apps/server/src/services/instance_id.ts` | 6 | 12-char random instance identifier |

### API Routes and Endpoints

| File | Lines | Key Content |
|------|-------|-------------|
| `/apps/server/src/routes/api/sync.ts` | 341 | Endpoints: getChanged, update, checkSync, etc. |
| `/apps/server/src/routes/api/login.ts` | 150+ | HMAC authentication: POST /api/login/sync |
| `/apps/server/src/routes/routes.ts` | 370+ | Endpoint registration and middleware |

### Entity and Blob Management

| File | Lines | Key Content |
|------|-------|-------------|
| `/apps/server/src/becca/entities/abstract_becca_entity.ts` | 250+ | generateHash(), _setContent(), blob management |

### WebSocket Communication

| File | Lines | Key Content |
|------|-------|-------------|
| `/apps/server/src/services/ws.ts` | 265 | Entity ordering, enrichment, broadcasting |

### Database Schema

| File | Lines | Key Content |
|------|-------|-------------|
| `/apps/server/src/assets/db/schema.sql` | 155+ | entity_changes table, indexes |

### Client-Side

| File | Lines | Key Content |
|------|-------|-------------|
| `/apps/client/src/services/sync.ts` | 31 | syncNow() API call |

---

## Key Concepts Quick Reference

### 1. Sync Flow
```
login() → push() → pull() → push() → finished() → checkHash()
                                          ↓
                                    [repeat if mismatch]
```

### 2. Conflict Resolution Algorithm
```
if (localTimestamp <= remoteTimestamp) {
    apply remote  // Remote wins (deterministic on tie)
} else {
    keep local    // Local is newer, re-queue for push
}
```

### 3. Hash Calculation
```
hash = SHA256("|prop1|prop2|...|propN"[+"|deleted"]).substr(0, 10)
```

### 4. Sector-Based Verification
```
sector = entityId[0]  // First character (0-9, a-z)
sectorHash = SHA256(concat(allHashesInSector + isErasedFlags))
```

### 5. Entity Ordering (Referential Dependencies)
```
Level 0: etapi_tokens, options, blobs
Level 1: notes
Level 2: branches, attributes, note_reordering, revisions
Level 3: attachments
```

### 6. Authentication
```
HMAC = SHA256(documentSecret, ISO8601_timestamp)
Validate: ±5 minutes clock tolerance, syncVersion match
```

---

## API Endpoints Summary

### Authentication
- **POST /api/login/sync**
  - Request: `{ timestamp, hash, syncVersion }`
  - Response: `{ instanceId, maxEntityChangeId }`

### Data Synchronization
- **GET /api/sync/changed** - Pull changes from server
- **PUT /api/sync/update** - Push changes to server
- **POST /api/sync/finished** - Signal sync completion

### Verification
- **GET /api/sync/check** - Content hash verification
- **POST /api/sync/check-entity-changes** - Consistency checks
- **POST /api/sync/queue-sector/:entityName/:sector** - Re-queue sector

### Management
- **POST /api/sync/test** - Test configuration
- **POST /api/sync/now** - Manual sync trigger
- **POST /api/sync/force-full-sync** - Full reset
- **POST /api/sync/fill-entity-changes** - Recovery
- **GET /api/sync/stats** - Status info

---

## Entity Changes Table Structure

```sql
CREATE TABLE entity_changes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,    -- Monotonic sequence number
    entityName TEXT,                         -- notes, branches, attributes, etc.
    entityId TEXT,                           -- Primary key of entity
    hash TEXT,                               -- SHA256 of properties (10 chars)
    isErased INT,                            -- Soft delete marker
    changeId TEXT,                           -- 12-char random (idempotency)
    componentId TEXT,                        -- Component making change
    instanceId TEXT,                         -- Source instance ID
    isSynced INT,                            -- Eligible for sync?
    utcDateChanged TEXT                      -- ISO 8601 timestamp
);

UNIQUE INDEX (entityName, entityId);
INDEX (changeId);
```

---

## Key Configuration Options

| Option | Type | Purpose | Example |
|--------|------|---------|---------|
| lastSyncedPull | Integer | Last change ID pulled | 2500 |
| lastSyncedPush | Integer | Last change ID pushed | 2450 |
| documentSecret | String | HMAC key | (SHA256 hash) |
| documentId | String | Document UUID | (UUID) |
| syncServerHost | URL | Sync server address | https://example.com |
| syncServerTimeout | Integer | Request timeout (ms) | 120000 |
| syncVersion | Integer | Protocol version | 34 |

---

## Synchronization State Machine

```
       ┌─────────────────────────────────────┐
       │                                     │
       ▼                                     │
[IDLE] → [LOGIN] → [PUSH1] → [PULL] → [PUSH2] → [FINISHED] → [CHECK]
                                                                  │
                                                    ┌─ PASS ──────┘
                                                    │
                                            ┌───────┴─ FAIL ────────┐
                                            │                      │
                                    [QUEUE_SECTOR] ────────────────┘
```

---

## Conflict Resolution Decision Tree

```
Remote change for entity E arrives with timestamp TR, hash HR

├─ Entity exists locally?
│  ├─ NO → Apply remote
│  └─ YES → Compare timestamps
│     ├─ TL < TR → Apply remote (remote is newer)
│     ├─ TL == TR → Apply remote (deterministic)
│     └─ TL > TR → Keep local (local is newer)
│        └─ Re-queue local for push
```

---

## Performance Characteristics

| Operation | Batch Size | Limit | Details |
|-----------|-----------|-------|---------|
| Push | 1000 changes | ~1MB | Per request; automatic pagination |
| Pull | 1000 changes | ~1MB | Per request; loops until empty |
| Hash Calculation | All synced | O(alphabet) | Sectored (36 sectors) |
| Sync Interval | 60 seconds | - | Auto-trigger frequency |
| Clock Tolerance | ±5 minutes | - | HMAC timestamp validation |

---

## Security Features

✓ **Authentication**: HMAC-SHA256(documentSecret, timestamp)
✓ **No Persistent Sessions**: Stateless auth with timestamp
✓ **Replay Protection**: Timestamp window ±5 minutes
✓ **Protected Content**: Encryption prefix in hash calculation
✓ **Instance Identity**: Prevents self-sync (instanceId comparison)
✓ **Idempotency**: changeId prevents duplicate application
✓ **Deterministic Hash**: Same content always produces same hash

---

## Troubleshooting Guide

### Sync Failures
1. **"Auth request time is out of sync"**
   - Check: System clock synchronization
   - Fix: Sync clocks (NTP)

2. **"Non-matching sync versions"**
   - Check: appInfo.syncVersion
   - Fix: Upgrade all instances to same version

3. **"No connection to sync server"**
   - Check: Network connectivity
   - Check: syncServerHost configuration
   - Check: Proxy settings

### Content Hash Mismatches
- **Automatic Recovery**: Sector re-queued automatically
- **Manual Recovery**: POST /api/sync/queue-sector/:entityName/:sector

---

## Important Implementation Details

### 1. Dual-Push Architecture
- First push sends local changes (timestamp ordering)
- Pull receives server changes
- Second push re-sends any changes triggered by pull

### 2. Sectored Hashing
- Reduces verification from O(entity count) to O(36)
- Enables localized recovery (sector-by-sector)
- Example: If sector 'a' of notes differs, only re-queue sector 'a'

### 3. Change IDs
- 12-character random string
- Idempotency token: same changeId = idempotent
- Prevents duplicate application of same change

### 4. Instance IDs
- 12-character random per server startup
- Prevents echo: don't apply your own changes
- Enables multi-master sync

### 5. Entity Ordering
- Prevents foreign key violations
- Respects referential dependencies
- Frontend (Froca) can't create skeletons like backend (Becca)

---

## WebSocket Message Types

```typescript
// Sync status
{ type: "sync-pull-in-progress", lastSyncedPush: N }
{ type: "sync-push-in-progress", lastSyncedPush: N }
{ type: "sync-finished", lastSyncedPush: N }
{ type: "sync-failed", lastSyncedPush: N }

// Entity updates
{
    type: "frontend-update",
    data: {
        lastSyncedPush: N,
        entityChanges: [
            { entityChange: {...}, entity: {...} },
            ...
        ]
    }
}

// UI control
{ type: "reload-frontend", reason: "..." }
{ type: "protectedSessionLogin" }
```

---

## Entity Type Hashing Details

| Entity Type | Hashed Properties | Notes |
|-------------|-------------------|-------|
| notes | noteId, title, isProtected, type, mime, blobId, isDeleted | - |
| branches | branchId, noteId, parentNoteId, notePosition, isDeleted | - |
| attributes | attributeId, noteId, type, name, value, isDeleted | - |
| revisions | revisionId, noteId, title, isProtected, blobId | - |
| attachments | attachmentId, ownerId, role, mime, title, isProtected, blobId, isDeleted | - |
| blobs | (content-addressable) | Hash used as blobId |
| note_reordering | (special case) | hash = "N/A", positions separate |
| options | (depends on isSynced) | Filtered by isSynced flag |

---

## Example Sync Cycle Timeline

```
T+0s:  Client A edits Note 123
       → entity_changes record: id=1000, entityName=notes, hash=abc...
       
T+5s:  Auto-sync triggers
       → Login: HMAC check ✓
       
T+6s:  PUSH (1st pass)
       → Send changes 1000-1005
       → Server stores in its entity_changes
       
T+8s:  PULL
       → Request changes > lastSyncedPull (2500)
       → Server returns 2501-2600 from other instances
       → Local DB updated
       
T+9s:  PUSH (2nd pass)
       → Any new changes from pull? None
       → Send nothing
       
T+10s: FINISHED
       → Notify server: sync complete
       
T+11s: CHECK CONTENT HASH
       → Calculate local sector hashes
       → Compare with server hashes
       → All match ✓ → Sync complete
```

---

## File Structure

```
/home/user/Trilium/
├── SYNC_EXPLORATION_INDEX.md          ← You are here
├── SYNC_SYSTEM_ANALYSIS.md            ← Full technical analysis
├── SYNC_ANALYSIS_SUMMARY.txt          ← Quick reference
└── apps/server/src/
    ├── services/
    │   ├── sync.ts                    ← Main orchestration
    │   ├── sync_update.ts             ← Conflict resolution
    │   ├── entity_changes.ts           ← Change tracking
    │   ├── content_hash.ts             ← Hash verification
    │   ├── sync_options.ts             ← Configuration
    │   ├── sync_mutex.ts               ← Exclusivity
    │   ├── instance_id.ts              ← Instance ID
    │   └── ws.ts                       ← WebSocket
    ├── routes/api/
    │   ├── sync.ts                    ← Sync endpoints
    │   ├── login.ts                   ← Auth endpoint
    │   └── routes.ts                  ← Route registration
    ├── becca/entities/
    │   └── abstract_becca_entity.ts    ← Hash generation
    └── assets/db/
        └── schema.sql                 ← Database schema
```

---

## How to Use This Documentation

### I want to understand...

1. **How sync works end-to-end**
   → Read: SYNC_SYSTEM_ANALYSIS.md Section 1

2. **How conflicts are resolved**
   → Read: SYNC_SYSTEM_ANALYSIS.md Section 4.3

3. **How hashing works**
   → Read: SYNC_SYSTEM_ANALYSIS.md Section 2.2

4. **How WebSocket updates work**
   → Read: SYNC_SYSTEM_ANALYSIS.md Section 5

5. **API endpoints**
   → Read: SYNC_ANALYSIS_SUMMARY.txt "API Endpoint Summary"

6. **Authentication mechanism**
   → Read: SYNC_SYSTEM_ANALYSIS.md Section 6.1

7. **Database schema**
   → Read: SYNC_SYSTEM_ANALYSIS.md Section 3

8. **Troubleshooting**
   → Read: SYNC_SYSTEM_ANALYSIS.md Section 11

---

## Additional Resources

- **CLAUDE.md**: Project overview and architecture
- **Schema**: `/apps/server/src/assets/db/schema.sql`
- **Tests**: Various `.spec.ts` files throughout

---

Generated: 2024-11-11
Last Updated: See git history
