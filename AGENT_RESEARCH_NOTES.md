# Trilium Notes - Comprehensive Agent Research Notes

**Generated**: 2025-11-11
**Purpose**: Reusable research for future AI agents working on Trilium codebase
**Coverage**: Architecture, Mobile UI, Synchronization System

---

## 📋 Table of Contents

1. [Quick Reference](#quick-reference)
2. [Architecture Overview](#architecture-overview)
3. [Three-Layer Cache System (Becca/Froca/Shaca)](#three-layer-cache-system)
4. [Entity System & Database](#entity-system--database)
5. [Widget-Based UI](#widget-based-ui)
6. [Mobile Implementation](#mobile-implementation)
7. [Synchronization System](#synchronization-system)
8. [Key Files Index](#key-files-index)
9. [Development Workflows](#development-workflows)

---

## Quick Reference

### Essential Commands
```bash
# Setup
pnpm install

# Development
pnpm server:start       # http://localhost:8080
pnpm desktop:start      # Electron app

# Testing
pnpm test:all          # All tests
pnpm test:parallel     # Parallel tests
pnpm test:sequential   # Sequential (server, ckeditor plugins)

# Build
pnpm client:build
pnpm server:build
pnpm desktop:build
```

### Key Technologies
- **Backend**: Node.js, Express 5.1.0, better-sqlite3 12.4.1
- **Frontend**: jQuery 3.7.1, Preact 10.27.2, Bootstrap 5.3.8
- **Desktop**: Electron 38.6.0
- **Editors**: CKEditor5 47.2.0, CodeMirror v6
- **Build**: Vite 7.2.2, TypeScript 5.9, pnpm 10.21.0

---

## Architecture Overview

### Monorepo Structure
```
Trilium/
├── apps/           (8 applications)
│   ├── server/     Node.js backend + REST API + WebSocket
│   ├── client/     Browser UI (shared by web + desktop)
│   ├── desktop/    Electron wrapper
│   ├── website/    Landing page
│   └── utilities/  (dump-db, db-compare, edit-docs, build-docs, server-e2e)
│
├── packages/       (13 shared libraries)
│   ├── commons/           Shared types & interfaces
│   ├── ckeditor5/         Rich text editor bundle
│   ├── codemirror/        Code editor (30+ languages)
│   ├── highlightjs/       Syntax highlighting
│   ├── ckeditor5-*/       Custom plugins (5: mermaid, math, admonition, footnotes, keyboard-marker)
│   └── utilities/         UI helpers (split.js, share-theme, turndown, express-partial-content)
│
└── docs/           User documentation
```

### Core Design Principles

1. **Multiple Parents**: Notes can have multiple parents via "branches" (not a traditional tree)
2. **Three-Layer Cache**: Server (Becca), Client (Froca), Share (Shaca)
3. **Event-Driven UI**: Widget-based architecture with 50+ event types
4. **Type-Specific Editors**: 15+ note types with specialized widgets
5. **Multi-Device Sync**: Timestamp-based conflict resolution
6. **Content Deduplication**: Hash-based blob storage

---

## Three-Layer Cache System

### 🔵 Becca (Backend Cache)
**Location**: `apps/server/src/becca/`

**Purpose**: Comprehensive server-side cache of all entities loaded at startup

**Data Structures**:
```typescript
class Becca {
    notes: Record<string, BNote>;                    // All notes
    branches: Record<string, BBranch>;               // Parent-child relationships
    childParentToBranch: Record<string, BBranch>;    // Fast lookup: "child-parent" → branch
    attributes: Record<string, BAttribute>;          // All attributes
    attributeIndex: Record<string, BAttribute[]>;    // Index: "type-name" → attributes[]
    options: Record<string, BOption>;                // Configuration
    etapiTokens: Record<string, BEtapiToken>;       // External API tokens
    allNoteSetCache: NoteSet | null;                // Cached set of all notes
    loaded: boolean;
}
```

**Key Features**:
- Loaded from SQLite at startup
- O(1) lookups via indexed structures
- Incremental updates via entity change events
- Content via `blobs` table with deduplication

**Loading Sequence**:
```
Database → Load notes → Load branches → Load attributes → Build indices → Ready
```

### 🟢 Froca (Frontend Cache)
**Location**: `apps/client/src/services/froca.ts`

**Purpose**: Lazy-loaded client-side mirror of Becca, read-only

**Data Structures**:
```typescript
class FrocaImpl implements Froca {
    notes: Record<string, FNote>;
    branches: Record<string, FBranch>;
    attributes: Record<string, FAttribute>;
    attachments: Record<string, FAttachment>;
    blobPromises: Record<string, Promise<FBlob | null>>;  // Short-lived (1s)
}
```

**Key Features**:
- Loads initial tree at startup (GET /tree)
- Lazy loads notes on demand (POST /tree/load)
- WebSocket updates from server
- Virtual branches for search results (prefixed "virt-")

**Loading Sequence**:
```
Browser → GET /tree → Initial tree → User clicks note → Lazy load → Display
```

### 🟠 Shaca (Share Cache)
**Location**: `apps/server/src/share/shaca/`

**Purpose**: Specialized cache for published/shared notes only

**Data Structures**:
```typescript
class Shaca {
    notes: Record<string, SNote>;
    branches: Record<string, SBranch>;
    attributes: Record<string, SAttribute>;
    attachments: Record<string, SAttachment>;
    aliasToNote: Record<string, SNote>;      // Share aliases
    shareRootNote: SNote | null;
    shareIndexEnabled: boolean;
    loaded: boolean;
}
```

**Key Features**:
- Subset of Becca (only shared subtrees)
- Complete reload on ANY change (simpler strategy)
- Protected notes show as "[protected]"
- Used for generating public share pages

---

## Entity System & Database

### Core Entities

All inherit from `AbstractBeccaEntity<T>`:

#### **BNote** (Notes)
```typescript
class BNote {
    noteId: string;
    title: string;
    type: NoteType;              // text, code, canvas, mermaid, etc.
    mime: string;
    isProtected: boolean;
    blobId: string;              // Reference to blobs table

    // Relationships
    parentBranches: BBranch[];
    parents: BNote[];
    children: BNote[];
    ownedAttributes: BAttribute[];
    targetRelations: BAttribute[];

    // Caches
    __flatTextCache: string | null;
    __attributeCache: BAttribute[] | null;
}
```

#### **BBranch** (Relationships)
```typescript
class BBranch {
    branchId: string;
    noteId: string;              // Child note
    parentNoteId: string;        // Parent note
    prefix: string | null;       // e.g., "Chapter 1 -"
    notePosition: number;        // Ordering among siblings
    isExpanded: boolean;         // UI state
}
```

**Key Innovation**: Branches enable **multiple parents per note**!

#### **BAttribute** (Metadata)
```typescript
class BAttribute {
    attributeId: string;
    noteId: string;
    type: AttributeType;         // "label" or "relation"
    name: string;
    position: number;
    value: string;               // Label value or target noteId for relations
    isInheritable: boolean;      // Inherit from parents
}
```

#### **BRevision** (Version History)
```typescript
class BRevision {
    revisionId: string;
    noteId: string;
    type: NoteType;
    mime: string;
    title: string;
    blobId: string;              // Content snapshot
    dateLastEdited: string;
    utcDateLastEdited: string;
}
```

#### **BOption** (Configuration)
```typescript
class BOption {
    name: string;                // Primary key
    value: string;
    isSynced: boolean;           // Sync to other devices?
}
```

### Database Schema
**Location**: `apps/server/src/assets/db/schema.sql`

**Key Tables**:
- `notes` - Note metadata
- `branches` - Parent-child relationships
- `attributes` - Labels & relations
- `blobs` - Content storage (hash-based deduplication)
- `revisions` - Version history
- `entity_changes` - Change tracking for sync
- `options` - Configuration
- `etapi_tokens` - External API tokens
- `attachments` - File attachments metadata

**Key Indexes**:
```sql
-- Fast branch lookups
CREATE INDEX IDX_branches_noteId_parentNoteId ON branches(noteId, parentNoteId);
CREATE INDEX IDX_branches_parentNoteId ON branches(parentNoteId);

-- Attribute searching
CREATE INDEX IDX_attributes_name_value ON attributes(name, value);
CREATE INDEX IDX_attributes_noteId_index ON attributes(noteId);

-- Sync tracking
CREATE UNIQUE INDEX IDX_entityChanges_entityName_entityId
    ON entity_changes(entityName, entityId);
```

### Entity Relationship Diagram
```
┌─────────────────────────────────────────┐
│              BNote                       │
│  noteId, title, type, mime, blobId      │
└──────┬──────┬──────────┬────────┬───────┘
       │      │          │        │
       │      │          │        └─ targetRelations
       │      │          │
       │      │     children (via branches)
       │      │
       │  ownedAttributes (BAttribute)
       │      ├─ type: "label" (key-value)
       │      └─ type: "relation" (→ targetNote)
       │
  parentBranches (BBranch)
       ├─ notePosition (ordering)
       ├─ prefix (e.g., "Chapter 1")
       └─ isExpanded (UI state)
```

---

## Widget-Based UI

### Widget Hierarchy
```
TypedComponent<T>                    (Base with event system)
├── BasicWidget                      (GUI rendering)
│   ├── NoteContextAwareWidget      (Responds to note changes)
│   │   └── RightPanelWidget        (Right sidebar widgets)
│   └── ReactWrappedWidget          (Wraps Preact/React components)
└── Component                        (Non-rendering components)
```

### Widget Lifecycle
```typescript
1. Constructor          Initialize attrs, classes, children
2. doRender()          Create DOM (this.$widget)
3. render()            Apply attrs, classes, CSS
4. isEnabled()         Determine visibility
5. refresh()           Update on note change
6. cleanup()           Destroy
```

### Type Widgets (15+ Note Types)

**Location**: `apps/client/src/widgets/type_widgets/`

Each note type has a specialized editor:

| Note Type | Widget | Technology | Location |
|-----------|--------|------------|----------|
| **Text** | EditableText | CKEditor5 | `text/EditableText.tsx` |
| **Code** | EditableCode | CodeMirror | `code/Code.tsx` |
| **Canvas** | Canvas | Excalidraw | `canvas/Canvas.tsx` |
| **Mermaid** | Mermaid | Mermaid.js | `mermaid/Mermaid.tsx` |
| **RelationMap** | RelationMap | jsPlumb | `relation_map/RelationMap.tsx` |
| **MindMap** | MindMap | Mind Elixir | `mind_map/MindMap.tsx` |
| **NoteMap** | NoteMap | Force-Graph | `note_map/NoteMap.tsx` |
| **Image** | Image | Canvas | `image/Image.tsx` |
| **File** | File | Download link | `file/File.tsx` |
| **WebView** | WebView | iframe | `webview/WebView.tsx` |
| **Search** | Search | Custom | `search/Search.tsx` |
| **Book/Doc** | Book | Hierarchical | `book/Book.tsx` |
| **aiChat** | AiChat | LLM integration | `llm_chat/AiChat.tsx` |

### Event System

**Location**: `apps/client/src/components/app_context.ts`

**50+ Event Types**, including:
- `noteSwitched` - Active note changed
- `activeContextChanged` - Tab/split context changed
- `noteTypeMimeChanged` - Note type changed
- `frocaReloaded` - Frontend cache reloaded
- `entitiesReloaded` - Entities refreshed
- `beforeNoteSwitch` - Before switching notes
- `noteDetailRefreshed` - Note detail refreshed

**Event Flow**:
```typescript
appContext.triggerEvent("noteSwitched", { noteContext, notePath })
    ↓
Component.handleEvent()
    ↓
Widget.noteSwitchedEvent(data)  // If method exists
    ↓
Widget.refresh()
```

**Synchronous Distribution, Asynchronous Execution**:
- Events sent in order
- Each handler executes independently
- All promises collected and can be awaited

### Widget Initialization Flow

**Entry Point**: `apps/client/src/desktop.ts`

```typescript
1. appContext.earlyInit()
2. Load custom widget bundles from notes
3. Create DesktopLayout with widget bundles
4. appContext.setLayout(layout)
5. appContext.start()
   ├── Render root widget
   ├── Initialize TabManager
   ├── Load saved note state
   ├── Trigger initialRenderComplete event
   └── Start backend sync
```

**Desktop Layout Structure**:
```
RootContainer
├── FlexContainer (tab row)
├── FlexContainer (main horizontal)
│   ├── LeftPaneContainer
│   │   ├── NoteTreeWidget
│   │   └── Custom left-pane widgets
│   ├── FlexContainer (center pane)
│   │   └── SplitNoteContainer
│   │       └── NoteWrapperWidget (per split)
│   │           ├── Ribbon
│   │           ├── ScrollingContainer
│   │           │   ├── ContentHeader
│   │           │   ├── NoteDetail (React)
│   │           │   └── Other details
│   │           └── Custom note-detail-pane widgets
│   └── RightPaneContainer
│       ├── TocWidget
│       ├── HighlightsListWidget
│       └── Custom right-pane widgets
└── Modals/Dialogs
```

---

## Mobile Implementation

### Mobile Detection & Initialization

**Server-Side Detection**: `apps/server/src/routes/index.ts`

```typescript
// User-Agent regex detection
const mobileRegex = /Android|webOS|iPhone|iPad|iPod|BlackBerry|Windows Phone/i;
const isMobile = req.header("user-agent")?.match(mobileRegex);

// Cookie override
if (req.cookies.trilium_device === 'mobile') isMobile = true;
if (req.cookies.trilium_device === 'desktop') isMobile = false;

// Serve appropriate template
res.render(isMobile ? 'mobile' : 'desktop');
```

**Mobile Entry Point**: `apps/client/src/mobile.ts`

```typescript
import mobileLayout from "./layouts/mobile_layout.js";
import appContext from "./components/app_context.js";

await appContext.earlyInit();
appContext.setLayout(mobileLayout);
await appContext.start();
```

### Mobile Layout Architecture

**Location**: `apps/client/src/layouts/mobile_layout.tsx`

**Two-Screen Design**:
- **Tree Screen**: Note tree navigation
- **Detail Screen**: Note editor

Only one screen visible at a time; switched via gesture or button.

```typescript
export default new RootContainer().child(
    new FlexContainer("column").css("height", "100%").id("launcher-pane").children(
        // Screen container with tree OR detail
        new ScreenContainer().children(
            // Tree screen
            new FlexContainer("column").id("mobile-tree").children(
                new GlobalMenuWidget(),
                new SearchBoxWidget(),
                new NoteTreeWidget(),
                new SyncStatusWidget()
            ),

            // Detail screen
            new FlexContainer("column").id("mobile-detail").children(
                new MobileDetailMenuWidget(),
                new TabRowWidget(),
                new SplitNoteContainer()
            )
        ),

        // Floating action buttons
        new CreatePaneButton(),
        new AddNoteButton(),
        // ... 3 more buttons
    )
);
```

### Mobile-Specific Widgets

**Location**: `apps/client/src/widgets/mobile_widgets/`

#### **SidebarContainer** - Gesture Handling
**File**: `sidebar_container.ts` (186 lines)

Features:
- **Left-edge drag** gesture (10px threshold) opens tree
- **Drag anywhere** when open closes tree
- Touch event handling: `touchstart`, `touchmove`, `touchend`
- Velocity calculation for smooth animations
- CSS transforms for hardware acceleration

```typescript
class SidebarContainer extends BasicWidget {
    handleTouchStart(e: TouchEvent) {
        const touch = e.touches[0];

        // Only trigger from left edge (10px)
        if (touch.clientX < 10) {
            this.isDragging = true;
            this.startX = touch.clientX;
        }
    }

    handleTouchMove(e: TouchEvent) {
        if (!this.isDragging) return;

        const touch = e.touches[0];
        const deltaX = touch.clientX - this.startX;

        // Apply transform
        this.$sidebar.css("transform", `translateX(${deltaX}px)`);
    }
}
```

#### **ScreenContainer** - Screen Switching
**File**: `screen_container.ts` (16 lines)

Manages visibility of tree vs detail screen:
```typescript
async noteSwitchedEvent() {
    // Show detail, hide tree
    $("#mobile-tree").addClass("hidden-int");
    $("#mobile-detail").removeClass("hidden-int");
}
```

#### **MobileDetailMenu** - Context Menu
**File**: `mobile_detail_menu.tsx` (58 lines)

Mobile-specific note actions menu.

#### **MobileEditorToolbar** - iOS Keyboard Fix
**File**: `text/mobile_editor_toolbar.tsx` (68 lines)

Special handling for iOS keyboard behavior:
```typescript
// iOS visual viewport differs from layout viewport
const isIOS = /iPhone|iPad|iPod/.test(navigator.userAgent);

if (isIOS) {
    // Fixed positioning relative to visual viewport
    visualViewport.addEventListener("resize", updateToolbarPosition);
    visualViewport.addEventListener("scroll", updateToolbarPosition);
}
```

### Mobile UI Differences

| Feature | Desktop | Mobile |
|---------|---------|--------|
| **Layout** | 3-pane (tree, detail, right) | 2-screen (tree OR detail) |
| **Navigation** | Always visible tree | Gesture/button to toggle |
| **Tree Icons** | 1.5em | 2em (larger touch targets) |
| **Tree Text** | 1em | 1.5em (larger) |
| **Floating Buttons** | 16 buttons | 5 buttons (reduced) |
| **Right Panel** | Always available | Not available |
| **Ribbon** | Full ribbon | Mobile ribbon |
| **Search** | Dedicated widget | Collapsible search box |

### Responsive CSS

**Bootstrap Grid Classes**:
```css
col-12        /* Mobile: full width */
col-sm-5/7    /* Tablet: 5/12 and 7/12 split */
col-md-4/8    /* Desktop: 4/12 and 8/12 split */
```

**Mobile-Specific Styles**:
- Larger touch targets (min 44x44px)
- Simplified navigation
- Bottom-positioned toolbars (iOS keyboard)
- Reduced animation complexity
- Hardware-accelerated transforms

---

## Synchronization System

**📖 Full Documentation**: See `SYNC_SYSTEM_ANALYSIS.md` (1103 lines)

### Sync Flow Overview

```
login() → push(1st) → pull() → push(2nd) → finished() → checkHash()
                                                              ↓
                                                    [If mismatch: queue sector, repeat]
```

**Dual-Push Architecture**:
1. **First Push**: Send local changes (timestamp ordering)
2. **Pull**: Receive remote changes
3. **Second Push**: Send any changes triggered by pull
4. **Finished**: Notify server
5. **Check**: Verify data consistency

### Entity Change Tracking

**Table Structure**:
```sql
CREATE TABLE entity_changes (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    entityName      TEXT NOT NULL,    -- notes, branches, attributes, etc.
    entityId        TEXT NOT NULL,    -- Primary key
    hash            TEXT NOT NULL,    -- SHA256 (first 10 chars)
    isErased        INT NOT NULL,     -- Soft delete (1 = deleted)
    changeId        TEXT NOT NULL,    -- 12-char random (idempotency)
    componentId     TEXT NOT NULL,    -- Component making change
    instanceId      TEXT NOT NULL,    -- Source instance (12-char random)
    isSynced        INTEGER NOT NULL, -- Eligible for sync?
    utcDateChanged  TEXT NOT NULL     -- ISO 8601 timestamp
);
```

**Hash Calculation**:
```typescript
// Concatenate hashed properties
contentToHash = "|noteId|title|type|mime|blobId"
if (isDeleted) contentToHash += "|deleted"

// SHA256, first 10 characters
hash = SHA256(contentToHash).substr(0, 10)
```

### Conflict Resolution

**Timestamp-Based, Last-Writer-Wins**:

```typescript
function handleConflict(local, remote) {
    if (local.utcDateModified <= remote.utcDateModified) {
        // Remote wins (deterministic on tie)
        applyRemote(remote);
    } else {
        // Local is newer
        keepLocal();
        requeueForPush(local);
    }
}
```

**Key Properties**:
- Deterministic (same timestamp → always choose remote)
- No user intervention needed
- Works in multi-master setup
- Simple and fast

### Sectored Hash Verification

**O(36) instead of O(millions)**:

```typescript
// Partition by first character of entityId
sector = entityId[0];  // '0'-'9', 'a'-'z' = 36 sectors

// Hash all hashes in sector
sectorHash = SHA256(concat(allHashesInSector + isErasedFlags))

// Compare with server
if (localSectorHash !== remoteSectorHash) {
    // Re-queue only this sector
    POST /api/sync/queue-sector/:entityName/:sector
}
```

**Benefits**:
- Localized recovery (not full resync)
- Fast verification (36 hashes vs millions)
- Automatic conflict detection

### Authentication

**HMAC-SHA256 Stateless Auth**:

```typescript
// Client generates
timestamp = new Date().toISOString();
hash = HMAC_SHA256(documentSecret, timestamp);

// Server validates
expectedHash = HMAC_SHA256(storedDocumentSecret, timestamp);
if (hash !== expectedHash) return 401;
if (Math.abs(now - timestamp) > 5_minutes) return 401;
```

**Properties**:
- No persistent sessions
- Replay protection (5-minute window)
- Per-document secret
- Sync version check

### WebSocket Real-Time Updates

**Location**: `apps/server/src/services/ws.ts`

**Message Types**:
```typescript
// Sync status
{ type: "sync-pull-in-progress", lastSyncedPush: N }
{ type: "sync-finished", lastSyncedPush: N }

// Entity updates
{
    type: "frontend-update",
    data: {
        lastSyncedPush: N,
        entityChanges: [
            { entityChange: {...}, entity: {...} }
        ]
    }
}

// UI control
{ type: "reload-frontend", reason: "..." }
```

**Entity Ordering** (prevents FK violations):
```typescript
// Level 0: No dependencies
["etapi_tokens", "options", "blobs"]

// Level 1: Depend on level 0
["notes"]

// Level 2: Depend on level 1
["branches", "attributes", "note_reordering", "revisions"]

// Level 3: Depend on level 2
["attachments"]
```

### Key Sync Algorithms

#### **Content-Addressable Blobs**
```typescript
// Use content hash as blob ID
blobId = SHA256(content).substr(0, 40);

// Deduplication: same content = same blobId
if (blobExists(blobId)) {
    return blobId;  // Reuse
}
INSERT INTO blobs (blobId, content) VALUES (?, ?);
```

#### **Idempotent Operations**
```typescript
// changeId prevents duplicate application
if (alreadyProcessed(changeId)) {
    return;  // Skip
}
processChange(change);
markProcessed(changeId);
```

#### **Instance Identity**
```typescript
// Don't apply your own changes
if (change.instanceId === myInstanceId) {
    return;  // Skip echo
}
```

### API Endpoints

**Authentication**:
- `POST /api/login/sync` - HMAC authentication

**Data Sync**:
- `GET /api/sync/changed` - Pull changes
- `PUT /api/sync/update` - Push changes
- `POST /api/sync/finished` - Signal completion

**Verification**:
- `GET /api/sync/check` - Hash verification
- `POST /api/sync/check-entity-changes` - Consistency
- `POST /api/sync/queue-sector/:entityName/:sector` - Re-queue

**Management**:
- `POST /api/sync/test` - Test config
- `POST /api/sync/now` - Manual trigger
- `POST /api/sync/force-full-sync` - Full reset
- `GET /api/sync/stats` - Status info

### Performance Characteristics

| Operation | Batch Size | Details |
|-----------|-----------|---------|
| Push | 1000 changes | ~1MB per request |
| Pull | 1000 changes | Loops until empty |
| Hash Check | 36 sectors | O(alphabet) instead of O(entities) |
| Sync Interval | 60 seconds | Auto-trigger |
| Clock Tolerance | ±5 minutes | HMAC validation |

---

## Key Files Index

### Backend (Server)

#### Core Services
| File | Purpose | Lines |
|------|---------|-------|
| `apps/server/src/main.ts` | Server entry point | - |
| `apps/server/src/becca/becca-interface.ts` | Becca cache class | - |
| `apps/server/src/becca/becca_loader.ts` | Becca initialization | - |
| `apps/server/src/share/shaca/shaca-interface.ts` | Shaca cache class | - |
| `apps/server/src/share/shaca/shaca_loader.ts` | Shaca initialization | - |

#### Entities
| File | Purpose |
|------|---------|
| `apps/server/src/becca/entities/abstract_becca_entity.ts` | Base entity class |
| `apps/server/src/becca/entities/bnote.ts` | Note entity |
| `apps/server/src/becca/entities/bbranch.ts` | Branch entity |
| `apps/server/src/becca/entities/battribute.ts` | Attribute entity |
| `apps/server/src/becca/entities/brevision.ts` | Revision entity |
| `apps/server/src/becca/entities/boption.ts` | Option entity |

#### Sync System
| File | Purpose | Lines |
|------|---------|-------|
| `apps/server/src/services/sync.ts` | Main sync orchestration | 466 |
| `apps/server/src/services/sync_update.ts` | Conflict resolution | 171 |
| `apps/server/src/services/entity_changes.ts` | Change tracking | 209 |
| `apps/server/src/services/content_hash.ts` | Sector hashing | 92 |
| `apps/server/src/services/ws.ts` | WebSocket sync | 265 |
| `apps/server/src/routes/api/sync.ts` | Sync API endpoints | 341 |
| `apps/server/src/routes/api/login.ts` | Auth endpoint | 150+ |

#### API Routes
| File | Purpose |
|------|---------|
| `apps/server/src/routes/api/tree.ts` | Tree endpoints |
| `apps/server/src/routes/api/notes.ts` | Note CRUD |
| `apps/server/src/routes/api/branches.ts` | Branch operations |
| `apps/server/src/routes/api/attributes.ts` | Attribute operations |
| `apps/server/src/routes/api/search.ts` | Search API |
| `apps/server/src/routes/index.ts` | Device detection | 134 |

#### Database
| File | Purpose |
|------|---------|
| `apps/server/src/assets/db/schema.sql` | Database schema |
| `apps/server/src/migrations/` | Migration scripts |

### Frontend (Client)

#### Core Services
| File | Purpose |
|------|---------|
| `apps/client/src/desktop.ts` | Desktop entry point |
| `apps/client/src/mobile.ts` | Mobile entry point |
| `apps/client/src/services/froca.ts` | Froca cache |
| `apps/client/src/components/app_context.ts` | Event system |
| `apps/client/src/components/note_context.ts` | Note context |

#### Layouts
| File | Purpose | Lines |
|------|---------|-------|
| `apps/client/src/layouts/desktop_layout.tsx` | Desktop layout | 201 |
| `apps/client/src/layouts/mobile_layout.tsx` | Mobile layout | 191 |

#### Widgets
| File | Purpose |
|------|---------|
| `apps/client/src/widgets/basic_widget.ts` | Base widget class |
| `apps/client/src/widgets/note_context_aware_widget.ts` | Note-aware widget |
| `apps/client/src/widgets/right_panel_widget.ts` | Right panel widget |
| `apps/client/src/widgets/note_types.tsx` | Type widget registry |

#### Mobile Widgets
| File | Purpose | Lines |
|------|---------|-------|
| `apps/client/src/widgets/mobile_widgets/sidebar_container.ts` | Gesture handling | 186 |
| `apps/client/src/widgets/mobile_widgets/screen_container.ts` | Screen switching | 16 |
| `apps/client/src/widgets/mobile_widgets/toggle_sidebar_button.tsx` | Toggle button | 19 |
| `apps/client/src/widgets/mobile_widgets/mobile_detail_menu.tsx` | Context menu | 58 |

#### Type Widgets
| File | Note Type |
|------|-----------|
| `apps/client/src/widgets/type_widgets/text/EditableText.tsx` | Rich text |
| `apps/client/src/widgets/type_widgets/code/Code.tsx` | Code editor |
| `apps/client/src/widgets/type_widgets/canvas/Canvas.tsx` | Drawing |
| `apps/client/src/widgets/type_widgets/mermaid/Mermaid.tsx` | Diagrams |
| `apps/client/src/widgets/type_widgets/relation_map/RelationMap.tsx` | Relations |
| `apps/client/src/widgets/type_widgets/mind_map/MindMap.tsx` | Mind maps |
| `apps/client/src/widgets/type_widgets/image/Image.tsx` | Images |

#### Entities
| File | Purpose |
|------|---------|
| `apps/client/src/entities/fnote.ts` | Frontend note |
| `apps/client/src/entities/fbranch.ts` | Frontend branch |
| `apps/client/src/entities/fattribute.ts` | Frontend attribute |

### Desktop (Electron)
| File | Purpose |
|------|---------|
| `apps/desktop/src/main.ts` | Electron main process |
| `apps/desktop/forge.config.ts` | Packaging config |

### Documentation
| File | Purpose | Lines |
|------|---------|-------|
| `CLAUDE.md` | Project overview | - |
| `AGENT_RESEARCH_NOTES.md` | This document | - |
| `SYNC_SYSTEM_ANALYSIS.md` | Sync deep dive | 1103 |
| `SYNC_EXPLORATION_INDEX.md` | Sync navigation | - |
| `SYNC_ANALYSIS_SUMMARY.txt` | Sync quick ref | - |

---

## Development Workflows

### Adding a New Note Type

1. **Create Type Widget**: `apps/client/src/widgets/type_widgets/my_type/MyType.tsx`
   ```typescript
   export default function MyType({ note, noteContext }: TypeWidgetProps) {
       // Implement editor
   }
   ```

2. **Register**: `apps/client/src/widgets/note_types.tsx`
   ```typescript
   export const TYPE_MAPPINGS = {
       myType: {
           view: () => import("./type_widgets/my_type/MyType"),
           className: "note-detail-my-type",
           printable: true
       }
   };
   ```

3. **Backend Handling**: `apps/server/src/services/notes.ts`
   - Add MIME type handling
   - Content validation

### Database Migration

1. **Create Migration**: `apps/server/src/migrations/NNNN_my_migration.ts`
   ```typescript
   export function up() {
       sql.execute(`
           ALTER TABLE notes ADD COLUMN newField TEXT;
       `);
   }

   export function down() {
       // Rollback logic
   }
   ```

2. **Update Schema**: `apps/server/src/assets/db/schema.sql`
   ```sql
   ALTER TABLE notes ADD COLUMN newField TEXT;
   ```

### Custom Widget (User-Created)

Users can create widgets in notes:

```javascript
// Note content with #widget attribute
class MyCustomWidget extends api.NoteContextAwareWidget {
    get parentWidget() { return 'right-pane'; }

    doRender() {
        this.$widget = $('<div>').text('Hello!');
    }

    async refreshWithNote(note) {
        // Update on note change
    }
}

module.exports = MyCustomWidget;
```

### Testing Strategy

**Parallel Tests**: Client, packages (except server, ckeditor5-mermaid, ckeditor5-math)
```bash
pnpm test:parallel
```

**Sequential Tests**: Server (shared DB), ckeditor5-mermaid, ckeditor5-math
```bash
pnpm test:sequential
```

**E2E Tests**: Playwright for both server and desktop
```bash
cd apps/server-e2e && pnpm test
cd apps/desktop && pnpm e2e
```

### Debugging Tips

1. **Server**: Debug logs in `apps/server/src/services/`
   ```typescript
   log.info("Debug message");
   ```

2. **Client**: Browser console
   ```typescript
   console.log("Debug:", froca.getNote(noteId));
   ```

3. **Sync**: Check `entity_changes` table
   ```sql
   SELECT * FROM entity_changes
   WHERE entityName = 'notes'
   ORDER BY id DESC LIMIT 10;
   ```

4. **Becca/Froca**: Inspect cache state
   ```typescript
   // Server
   console.log(becca.notes[noteId]);

   // Client
   console.log(froca.getNoteFromCache(noteId));
   ```

---

## Common Patterns

### Entity Modification (Server)
```typescript
const note = becca.getNote(noteId);
note.title = "New Title";
note.save();  // Triggers entity_changes, sync, WebSocket update
```

### Frontend Note Loading
```typescript
// Lazy load
const note = await froca.getNote(noteId);

// Cached
const note = froca.getNoteFromCache(noteId);
if (!note) {
    // Not loaded yet
}
```

### Event Handling
```typescript
class MyWidget extends NoteContextAwareWidget {
    async noteSwitchedEvent({ noteContext }) {
        await this.refresh();
    }

    async refreshWithNote(note) {
        // Update UI for new note
    }
}
```

### Search
```typescript
// Server-side search
const results = await searchService.searchFromNote("#todo");

// Client-side attribute lookup
const todoNotes = froca.notes
    .filter(note => note.hasAttribute("label", "todo"));
```

---

## Architecture Diagrams

### Data Flow: Entity Modification
```
User edits note
    ↓
TypeWidget updates
    ↓
note.save()
    ↓
SQL UPDATE
    ↓
entity_changes INSERT
    ↓
eventService.emit(ENTITY_CHANGED)
    ↓
├─ Becca updates cache
├─ WebSocket broadcasts
│   └─ Other clients: Froca updates
└─ Sync: Next cycle pushes change
```

### Sync Flow: Two Devices
```
Device A                    Server                    Device B
   │                          │                          │
   │──login()────────────────→│                          │
   │←─{instanceId, maxId}─────│                          │
   │                          │                          │
   │──push({changes})────────→│──stores in DB           │
   │←─{lastSyncedPush}────────│                          │
   │                          │                          │
   │──pull()─────────────────→│──query entity_changes───→│
   │←─{changes from B}────────│                          │
   │──apply changes           │                          │
   │                          │                          │
   │──push({new changes})────→│                          │
   │──finished()─────────────→│                          │
   │                          │                          │
   │──checkHash()────────────→│                          │
   │←─{sectorHashes}──────────│                          │
   │──compare                 │                          │
   │                          │                          │
   │                          │←─────login()─────────────│
   │                          │──{instanceId, maxId}────→│
   │                          │←─────pull()──────────────│
   │                          │──{changes from A}───────→│
   │                          │                  apply   │
```

---

## Performance Considerations

### Cache Efficiency

| Cache | Scope | Lifetime | Update Cost |
|-------|-------|----------|-------------|
| Becca | All entities | Session | Incremental (low) |
| Froca | Lazy subset | Session | Incremental (low) |
| Shaca | Shared only | Session | Full reload (medium) |

### Query Performance

**Fast (O(1))**:
- `becca.notes[noteId]`
- `becca.childParentToBranch["child-parent"]`
- `froca.getNoteFromCache(noteId)`

**Medium (O(n) over subset)**:
- `becca.findAttributes("label", "todo")`
- Attribute index lookup

**Slow (O(n) full scan)**:
- Full-text search
- Subtree traversal
- Deleting notes (check all references)

### Sync Optimization

**Sectored Hashing**: O(36) instead of O(millions)
**Batching**: 1000 changes per request
**Compression**: Entity changes are relatively small
**Deduplication**: Content-addressable blobs

---

## Security Features

### Authentication
- HMAC-SHA256 stateless auth
- Per-document secret
- Timestamp validation (±5 min)
- No persistent sessions

### Encryption
- Per-note encryption
- Protected sessions
- Encrypted content in `blobs` table
- Deterministic hashing (hash on plaintext for dedup)

### Sync Security
- Instance ID prevents echo
- changeId prevents duplicates
- Referential ordering prevents FK violations
- ETAPI tokens for external access

---

## Troubleshooting

### Common Issues

**"No connection to sync server"**
- Check `syncServerHost` option
- Verify network connectivity
- Check proxy settings

**"Auth request time is out of sync"**
- System clock needs NTP sync
- Check time zones

**"Non-matching sync versions"**
- Upgrade all instances to same version

**Hash mismatches**
- Automatic sector re-queuing
- Manual: `POST /api/sync/queue-sector/:entityName/:sector`

**Mobile not detecting**
- Check User-Agent header
- Set cookie: `trilium_device=mobile`
- Clear browser cache

---

## Additional Resources

### External Documentation
- **Trilium Docs**: https://triliumnotes.org/docs
- **GitHub**: https://github.com/TriliumNext/Trilium
- **Project Instructions**: `CLAUDE.md`

### Internal Documentation
- **Sync Analysis**: `SYNC_SYSTEM_ANALYSIS.md` (1103 lines)
- **Sync Index**: `SYNC_EXPLORATION_INDEX.md`
- **Sync Summary**: `SYNC_ANALYSIS_SUMMARY.txt`

### Test Files
- `apps/server/test/` - Server tests
- `apps/client/test/` - Client tests
- `apps/server-e2e/` - E2E tests
- `apps/desktop/test/` - Desktop E2E tests

---

## Glossary

**Becca**: Backend Cache (server-side entity cache)
**Froca**: Frontend Cache (client-side lazy-loaded cache)
**Shaca**: Share Cache (published notes cache)
**Branch**: Parent-child relationship (enables multiple parents)
**Attribute**: Label or relation metadata
**Label**: Key-value attribute (e.g., `priority=high`)
**Relation**: Named link to another note (e.g., `child=noteId`)
**Protected**: Encrypted note
**Entity Change**: Record in `entity_changes` table for sync
**Sector**: First character of entityId (for partitioned hashing)
**changeId**: 12-char random idempotency token
**instanceId**: 12-char random server identifier
**Type Widget**: Note type-specific editor component
**Note Context**: Tab/split context with unique `ntxId`

---

**Document Version**: 1.0
**Last Updated**: 2025-11-11
**Contributors**: AI Agent Exploration
**License**: AGPL-3.0-only (matching project)

---

## Usage for Future Agents

This document serves as a comprehensive reference for AI agents working on the Trilium codebase. Use it to:

1. **Understand Architecture**: Read relevant sections before making changes
2. **Find Files**: Use the Key Files Index to locate code
3. **Follow Patterns**: Use Development Workflows for common tasks
4. **Debug Issues**: Refer to Troubleshooting section
5. **Understand Sync**: Read Synchronization System section
6. **Mobile Development**: Review Mobile Implementation section

**Quick Navigation**:
- Need to understand sync? → [Synchronization System](#synchronization-system) + `SYNC_SYSTEM_ANALYSIS.md`
- Need to add note type? → [Development Workflows](#development-workflows)
- Need to understand caching? → [Three-Layer Cache System](#three-layer-cache-system)
- Need to modify UI? → [Widget-Based UI](#widget-based-ui)
- Need mobile info? → [Mobile Implementation](#mobile-implementation)

This document is designed to minimize redundant research and accelerate development velocity for future agents.
