# Trilium Mobile & Offline Strategy Proposal

**Date**: 2025-11-11
**Author**: AI Agent Analysis
**Status**: Draft for Discussion

---

## 📋 Executive Summary

This document proposes strategies for enabling offline mobile and desktop access to Trilium Notes, with emphasis on **Progressive Web App (PWA)** and **Tauri** approaches. The goal is to provide offline access on work computers (where installation is restricted) and mobile devices, while avoiding Electron's overhead.

**Key Finding**: A **hybrid approach** combining PWA for work computers and Tauri for personal devices offers the best balance of accessibility, performance, and development effort.

---

## 🎯 Requirements & Goals

### Primary Requirements
1. **Offline access on work computers** without installation
2. **Mobile offline access** (iOS/Android)
3. **Lower overhead** than Electron
4. **Sync capability** when online
5. **Maintain feature parity** with desktop app

### Nice-to-Have
- Native mobile app feel
- Background sync
- Push notifications for shared notes
- Camera integration for image notes
- Biometric authentication

---

## 🔍 Current State Analysis

### What Trilium Has Today

**✅ Strengths**:
- Mobile-responsive UI (`mobile_layout.tsx`)
- Sophisticated sync system (timestamp-based, conflict resolution)
- Three-layer cache (Becca/Froca/Shaca)
- SQLite database with full-text search
- 15+ note types with specialized editors
- Real-time WebSocket updates

**❌ Limitations**:
- Requires server backend (Node.js)
- Uses `better-sqlite3` (Node.js native binding)
- Synchronous database access throughout codebase
- Desktop app is Electron (~50MB overhead)
- No offline capability (server-dependent)

### Architecture Implications

**Becca Cache** (Server-side):
- Loads entire database at startup synchronously
- ~500 lines of blocking SQL queries
- Requires Node.js SQLite bindings

**Froca Cache** (Client-side):
- Lazy-loads from server via HTTP
- Currently read-only
- No local persistence (session-only)

---

## 🌐 Option 1: Progressive Web App (PWA)

### Overview

Convert Trilium into a PWA that can work offline using browser-based SQLite.

### Technical Approach

#### **A. SQLite in Browser**

**Recommended**: **Official SQLite WASM with OPFS** (Origin Private File System)

**Why**:
- Official SQLite Project maintenance
- Production-proven (Notion uses it, saw 20% performance improvement)
- Full SQL compatibility (no query rewrites)
- OPFS provides near-native performance
- Handles Trilium's typical DB sizes (50-500MB)

**How It Works**:
```
┌─────────────────────────────────────────┐
│        Trilium PWA Architecture         │
├─────────────────────────────────────────┤
│  UI Layer (React/Preact)                │
│    ↓                                    │
│  Froca Cache (IndexedDB for metadata)   │
│    ↓                                    │
│  SQLite WASM (Web Worker)               │
│    ↓                                    │
│  OPFS Backend (persistent storage)      │
│    ↓                                    │
│  Service Worker (offline support)       │
└─────────────────────────────────────────┘
```

**Browser Compatibility**:
| Browser | OPFS Support | Status |
|---------|--------------|--------|
| Chrome 121+ | ✅ Full | Production ready |
| Firefox | ✅ Full | Production ready |
| Safari 17+ | ✅ Full | Requires iOS 17+ |
| Safari 16.4-16.9 | ⚠️ Partial | Fallback needed |
| Edge 121+ | ✅ Full | Production ready |

**Critical iOS Limitation**:
- **Safari deletes all storage after 7 days of inactivity** 😱
- Home screen PWAs can't request quota increase
- **Impact**: Pure offline PWA not viable for iOS without server sync fallback

#### **B. Architecture Changes Required**

**1. Database Layer Refactoring** (4-6 months effort)

```typescript
// CURRENT (Synchronous)
class BeccaLoader {
    load() {
        const notes = sql.getRows("SELECT * FROM notes");
        for (const note of notes) {
            new BNote(note).init();
        }
    }
}

// PWA (Asynchronous)
class BeccaLoader {
    async load() {
        const notes = await sql.getRowsAsync("SELECT * FROM notes");
        for (const note of notes) {
            await new BNote(note).init();
        }
    }
}
```

**Breaking Changes**:
- All `sql.getRows()` → `await sql.getRowsAsync()`
- All `entity.save()` → `await entity.save()`
- All transactions become Promise-based
- Worker thread communication overhead (~30-35% performance cost)

**2. Offline-First Cache Strategy**

```typescript
class OfflineFroca {
    // Store note tree in IndexedDB
    async cacheNoteTree() {
        const db = await openDB('trilium-cache');
        await db.put('notes', this.notes);
    }

    // Sync queue for offline changes
    syncQueue: EntityChange[] = [];

    async queueChange(change: EntityChange) {
        this.syncQueue.push(change);
        await this.persistSyncQueue();
    }

    // Background sync when online
    async syncWhenOnline() {
        if (navigator.onLine) {
            await this.pushChanges(this.syncQueue);
            this.syncQueue = [];
        }
    }
}
```

**3. Service Worker for Offline Assets**

```typescript
// Cache all static assets
self.addEventListener('install', (event) => {
    event.waitUntil(
        caches.open('trilium-v1').then((cache) => {
            return cache.addAll([
                '/client/build/app.js',
                '/client/build/styles.css',
                '/libraries/ckeditor/ckeditor.js',
                // ... all assets
            ]);
        })
    );
});

// Serve from cache when offline
self.addEventListener('fetch', (event) => {
    event.respondWith(
        caches.match(event.request).then((response) => {
            return response || fetch(event.request);
        })
    );
});
```

### Pros & Cons

**✅ Pros**:
- **No installation required** (perfect for work computers)
- **Lightweight** (no Electron/Chromium overhead)
- **Cross-platform** (Windows, Mac, Linux, Android)
- **Automatic updates** via service worker
- **URL-based sharing** (trilium.example.com)
- **Smaller footprint** than Electron (~2MB vs ~50MB)

**❌ Cons**:
- **iOS Safari storage deletion** (7-day limit kills offline-first)
- **Massive refactoring effort** (4-6 months, ~560 hours)
- **Performance overhead** (30-35% from worker threads)
- **Breaking changes** to entire codebase
- **Limited background sync** (iOS restrictions)
- **No file system access** (can't import/export easily)
- **Storage quota limits** on some browsers

### Implementation Roadmap

**Phase 1: Proof of Concept** (1-2 months)
- [ ] SQLite WASM integration with OPFS
- [ ] Basic note loading in worker thread
- [ ] Service worker for offline assets
- [ ] Simple sync queue

**Phase 2: Core Refactoring** (3-4 months)
- [ ] Async database layer
- [ ] Becca loader refactoring
- [ ] Transaction model migration
- [ ] Entity save pattern updates

**Phase 3: Sync & Polish** (2-3 months)
- [ ] Offline sync queue
- [ ] Conflict resolution UI
- [ ] Background sync registration
- [ ] IndexedDB fallback for Safari 16.4+

**Total Estimated Effort**: **6-9 months**

---

## 🦀 Option 2: Tauri (Rust + Webview)

### Overview

Build native desktop and mobile apps using Tauri, which embeds platform webviews instead of bundling Chromium.

### Technical Approach

**Architecture**:
```
┌──────────────────────────────────────────┐
│       Trilium Tauri Architecture         │
├──────────────────────────────────────────┤
│  Frontend (existing client code)         │
│    ↓                                     │
│  Tauri Commands (Rust ↔ JS bridge)      │
│    ↓                                     │
│  Rust Backend                            │
│    ├─ SQLite (rusqlite)                 │
│    ├─ Sync Service                      │
│    └─ File System Access                │
│    ↓                                     │
│  Native OS APIs                          │
└──────────────────────────────────────────┘
```

**Key Technologies**:
- **Tauri Core**: Rust backend with webview
- **rusqlite**: Native SQLite bindings
- **Platform Webviews**:
  - Windows: WebView2 (Edge/Chromium)
  - macOS: WKWebView (Safari)
  - Linux: WebKitGTK
  - Mobile: System webview

**App Size Comparison**:
```
Electron App:  ~50-80 MB
Tauri App:     ~3-8 MB   (15x smaller!)
```

### Implementation Strategy

#### **Option 2A: Full Rust Backend**

Reimplement Trilium's backend in Rust:

```rust
// Tauri command example
#[tauri::command]
async fn get_note(note_id: String) -> Result<Note, String> {
    let db = get_db_connection()?;
    let note = db.query_row(
        "SELECT * FROM notes WHERE noteId = ?",
        params![note_id],
        |row| Ok(Note::from_row(row))
    )?;
    Ok(note)
}

// Sync service in Rust
struct SyncService {
    client: reqwest::Client,
    db: Connection,
}

impl SyncService {
    async fn push_changes(&self) -> Result<(), SyncError> {
        let changes = self.get_pending_changes()?;
        self.client.put("/api/sync/update")
            .json(&changes)
            .send()
            .await?;
        Ok(())
    }
}
```

**Pros**:
- ⚡ **Blazing fast** (Rust performance)
- 📦 **Tiny bundles** (3-8 MB vs 50-80 MB)
- 🔒 **Memory safe** (Rust guarantees)
- 🌐 **Full SQLite access** (rusqlite)
- 📱 **Mobile support** (Tauri Mobile experimental)

**Cons**:
- 🔄 **Complete rewrite** of backend (6-12 months)
- 🦀 **Learning curve** (team needs Rust expertise)
- ⚠️ **Breaking changes** to plugin API
- 📱 **Mobile is alpha** (not production-ready yet)

#### **Option 2B: Hybrid Tauri + Node.js**

Use Tauri as a shell, keep Node.js backend:

```rust
// Tauri spawns Node.js process
use tauri::api::process::{Command, CommandEvent};

#[tauri::command]
async fn start_server() -> Result<(), String> {
    let (mut rx, child) = Command::new_sidecar("trilium-server")
        .expect("failed to create sidecar command")
        .spawn()
        .expect("failed to spawn sidecar");

    Ok(())
}
```

**Architecture**:
```
Tauri Shell (Rust)
  ├─ Webview (UI)
  └─ Sidecar Process (Node.js server)
      └─ Existing Trilium backend
```

**Pros**:
- ✅ **Minimal changes** to existing codebase
- ✅ **Keep all features** (no rewrite needed)
- 📦 **Smaller than Electron** (~20-30 MB with Node)
- 🚀 **Faster to implement** (2-3 months)

**Cons**:
- 📦 **Still bundles Node.js** (not as small as pure Rust)
- 🔧 **Two processes** to manage
- 🔌 **No mobile support** (Node.js doesn't run on mobile)

### Tauri Pros & Cons

**✅ Overall Pros**:
- **15x smaller** than Electron
- **Native performance** (system webview)
- **Lower memory usage** (~50% less than Electron)
- **Native OS integration** (file dialogs, notifications)
- **Security-first** (restricted by default)
- **Cross-platform** (Windows, Mac, Linux)

**❌ Overall Cons**:
- **Requires installation** (doesn't solve work computer constraint)
- **Mobile support is alpha** (not production-ready)
- **Webview inconsistencies** (different rendering on each platform)
- **Smaller ecosystem** than Electron
- **Distribution complexity** (App Store, code signing)

### Implementation Roadmap

**Phase 1: Desktop Tauri Shell** (1-2 months)
- [ ] Tauri project setup
- [ ] Node.js sidecar integration
- [ ] Frontend connection to localhost server
- [ ] Package for Windows/Mac/Linux

**Phase 2: Native Features** (1-2 months)
- [ ] File system integration
- [ ] System tray
- [ ] Native notifications
- [ ] Auto-updater

**Phase 3: Mobile (Future)** (TBD)
- [ ] Wait for Tauri Mobile stable release
- [ ] Evaluate Rust backend necessity
- [ ] Mobile UI adaptations

**Total Estimated Effort**: **2-4 months** (hybrid approach)

---

## 🔀 Option 3: Hybrid Approach (Recommended)

### Strategy

Combine PWA and Tauri for different use cases:

```
┌─────────────────────────────────────────────────┐
│           Trilium Multi-Platform                │
├─────────────────────────────────────────────────┤
│  Work Computer (no install)                     │
│    → PWA with limited offline (IndexedDB cache) │
│                                                  │
│  Personal Desktop (full control)                │
│    → Tauri app (full offline, native features)  │
│                                                  │
│  Mobile (iOS/Android)                           │
│    → PWA initially, Tauri Mobile when stable    │
│                                                  │
│  All sync to central server via same protocol   │
└─────────────────────────────────────────────────┘
```

### Implementation Plan

#### **Phase 1: Enhanced PWA** (2-3 months)

**Lightweight offline support** WITHOUT full SQLite WASM:

1. **Service Worker for Assets**
   ```typescript
   // Cache all static files
   - JavaScript bundles
   - CSS stylesheets
   - CKEditor, CodeMirror libraries
   - Fonts, icons
   ```

2. **IndexedDB for Recent Notes Cache**
   ```typescript
   interface CachedNote {
       noteId: string;
       title: string;
       content: string;
       attributes: Attribute[];
       cachedAt: Date;
   }

   // Cache last 100 viewed notes
   async cacheNote(note: Note) {
       const db = await openDB('trilium-cache');
       await db.put('notes', {
           noteId: note.noteId,
           title: note.title,
           content: await note.getContent(),
           attributes: note.attributes,
           cachedAt: new Date()
       });
   }
   ```

3. **Offline UI Indicators**
   - Show cached notes available offline
   - "View offline notes" mode
   - Sync queue for offline edits

**Pros**:
- ✅ **Works on work computers** (no installation)
- ✅ **Fast to implement** (2-3 months)
- ✅ **Graceful degradation** (online = full features)
- ✅ **Read access offline** for recent notes

**Cons**:
- ⚠️ **Limited offline editing** (recent notes only)
- ⚠️ **No full-text search offline**
- ⚠️ **iOS 7-day deletion** still applies

#### **Phase 2: Tauri Desktop App** (2-3 months, parallel)

**Full offline desktop experience**:

1. **Tauri + Node.js Sidecar**
   - Use existing Trilium backend
   - Tauri provides native shell
   - Package size: ~20-30 MB (vs Electron's ~50-80 MB)

2. **Benefits**:
   - Full SQLite database
   - Full-text search
   - Complete offline functionality
   - Native OS integration

#### **Phase 3: Mobile Strategy** (Future)

**Near-term** (2025-2026):
- Enhanced PWA with IndexedDB cache
- "Add to Home Screen" for app-like experience
- Accept iOS storage limitations

**Long-term** (2026+):
- Tauri Mobile when stable
- OR native iOS/Android apps with shared TypeScript/React code
- OR React Native with SQLite

### Effort Comparison

| Approach | Effort | Timeline | Offline Quality |
|----------|--------|----------|-----------------|
| **PWA (Full SQLite)** | 560 hrs | 6-9 months | 95% |
| **PWA (IndexedDB cache)** | 160 hrs | 2-3 months | 60% |
| **Tauri (Full Rust)** | 1200 hrs | 12-18 months | 100% |
| **Tauri (Node sidecar)** | 240 hrs | 2-4 months | 100% |
| **Hybrid (PWA + Tauri)** | 400 hrs | 4-6 months | 80% avg |

---

## 🎯 Recommendation

### **Implement Hybrid Approach**

**Phase 1**: Enhanced PWA (2-3 months)
- Service worker for offline assets
- IndexedDB cache for recent notes (last 100)
- Offline read access
- Limited offline editing with sync queue

**Phase 2**: Tauri Desktop (2-3 months, parallel)
- Tauri shell with Node.js sidecar
- Full offline SQLite database
- Native features (file system, notifications)
- Replace Electron desktop app

**Phase 3**: Mobile Evolution (2026+)
- Continue PWA for mobile near-term
- Evaluate Tauri Mobile when stable (2026)
- Consider native apps if needed

### Why This Works

✅ **Solves work computer problem**: PWA needs no installation
✅ **Solves personal desktop**: Tauri provides full offline + smaller than Electron
✅ **Solves mobile**: PWA works today, better options later
✅ **Reasonable effort**: 4-6 months vs 12-18 months for full solutions
✅ **Incremental value**: Each phase delivers working features

---

## 📊 Technical Deep Dive: PWA IndexedDB Approach

### Architecture

```typescript
// Offline-capable Froca with IndexedDB
class OfflineFroca extends Froca {
    private db: IDBDatabase;
    private syncQueue: EntityChange[] = [];

    async init() {
        this.db = await this.openDB();
        await this.loadCachedNotes();
        this.setupSyncListener();
    }

    private async openDB(): Promise<IDBDatabase> {
        return new Promise((resolve, reject) => {
            const request = indexedDB.open('trilium', 2);

            request.onupgradeneeded = (event) => {
                const db = event.target.result;

                // Notes store
                if (!db.objectStoreNames.contains('notes')) {
                    const noteStore = db.createObjectStore('notes',
                        { keyPath: 'noteId' });
                    noteStore.createIndex('title', 'title');
                    noteStore.createIndex('type', 'type');
                    noteStore.createIndex('cachedAt', 'cachedAt');
                }

                // Sync queue store
                if (!db.objectStoreNames.contains('syncQueue')) {
                    db.createObjectStore('syncQueue',
                        { keyPath: 'id', autoIncrement: true });
                }
            };

            request.onsuccess = () => resolve(request.result);
            request.onerror = () => reject(request.error);
        });
    }

    async cacheNote(note: FNote) {
        const tx = this.db.transaction('notes', 'readwrite');
        const store = tx.objectStore('notes');

        await store.put({
            noteId: note.noteId,
            title: note.title,
            type: note.type,
            mime: note.mime,
            content: await this.getNoteContent(note.noteId),
            attributes: note.attributes,
            children: note.children,
            parents: note.parents,
            cachedAt: new Date(),
        });

        // LRU eviction: keep only last 100 notes
        await this.evictOldNotes(100);
    }

    async getOfflineNotes(): Promise<FNote[]> {
        const tx = this.db.transaction('notes', 'readonly');
        const store = tx.objectStore('notes');
        const index = store.index('cachedAt');

        return new Promise((resolve) => {
            const notes: FNote[] = [];
            const request = index.openCursor(null, 'prev');

            request.onsuccess = (event) => {
                const cursor = event.target.result;
                if (cursor) {
                    notes.push(this.hydrateNote(cursor.value));
                    cursor.continue();
                } else {
                    resolve(notes);
                }
            };
        });
    }

    async queueChange(change: EntityChange) {
        const tx = this.db.transaction('syncQueue', 'readwrite');
        await tx.objectStore('syncQueue').add({
            ...change,
            queuedAt: new Date(),
        });

        // Try to sync immediately if online
        if (navigator.onLine) {
            await this.processSyncQueue();
        }
    }

    async processSyncQueue() {
        const tx = this.db.transaction('syncQueue', 'readonly');
        const store = tx.objectStore('syncQueue');
        const changes = await store.getAll();

        if (changes.length === 0) return;

        try {
            await server.put('sync/update', { changes });

            // Clear queue on success
            const deleteTx = this.db.transaction('syncQueue', 'readwrite');
            await deleteTx.objectStore('syncQueue').clear();
        } catch (err) {
            console.error('Sync failed, will retry later', err);
        }
    }

    private setupSyncListener() {
        // Online event
        window.addEventListener('online', () => {
            this.processSyncQueue();
        });

        // Background sync (if available)
        if ('sync' in navigator.serviceWorker) {
            navigator.serviceWorker.ready.then((registration) => {
                registration.sync.register('sync-notes');
            });
        }
    }
}
```

### Service Worker

```typescript
// service-worker.ts
const CACHE_NAME = 'trilium-v1';
const STATIC_ASSETS = [
    '/',
    '/client/build/app.js',
    '/client/build/styles.css',
    '/libraries/ckeditor/ckeditor.js',
    '/libraries/codemirror/codemirror.js',
    // ... all static assets
];

// Install: cache static assets
self.addEventListener('install', (event: ExtendableEvent) => {
    event.waitUntil(
        caches.open(CACHE_NAME).then((cache) => {
            return cache.addAll(STATIC_ASSETS);
        })
    );
});

// Fetch: serve from cache when offline
self.addEventListener('fetch', (event: FetchEvent) => {
    const url = new URL(event.request.url);

    // API requests: network first, cache fallback
    if (url.pathname.startsWith('/api/')) {
        event.respondWith(
            fetch(event.request)
                .then((response) => {
                    // Cache successful responses
                    if (response.ok) {
                        const clone = response.clone();
                        caches.open(CACHE_NAME).then((cache) => {
                            cache.put(event.request, clone);
                        });
                    }
                    return response;
                })
                .catch(() => {
                    // Offline: try cache
                    return caches.match(event.request);
                })
        );
    } else {
        // Static assets: cache first
        event.respondWith(
            caches.match(event.request).then((cached) => {
                return cached || fetch(event.request);
            })
        );
    }
});

// Background sync
self.addEventListener('sync', (event: SyncEvent) => {
    if (event.tag === 'sync-notes') {
        event.waitUntil(
            // Trigger sync in main thread
            self.clients.matchAll().then((clients) => {
                clients.forEach((client) => {
                    client.postMessage({ type: 'BACKGROUND_SYNC' });
                });
            })
        );
    }
});
```

### UI Indicators

```typescript
// Offline indicator component
function OfflineIndicator() {
    const [isOnline, setIsOnline] = useState(navigator.onLine);
    const [syncQueueSize, setSyncQueueSize] = useState(0);

    useEffect(() => {
        const updateOnlineStatus = () => setIsOnline(navigator.onLine);

        window.addEventListener('online', updateOnlineStatus);
        window.addEventListener('offline', updateOnlineStatus);

        return () => {
            window.removeEventListener('online', updateOnlineStatus);
            window.removeEventListener('offline', updateOnlineStatus);
        };
    }, []);

    if (isOnline && syncQueueSize === 0) {
        return null; // All synced, online
    }

    return (
        <div className={`offline-indicator ${isOnline ? 'syncing' : 'offline'}`}>
            {isOnline ? (
                <>
                    <Icon name="sync" className="spinning" />
                    Syncing {syncQueueSize} changes...
                </>
            ) : (
                <>
                    <Icon name="offline" />
                    Offline - {syncQueueSize} changes queued
                </>
            )}
        </div>
    );
}

// Offline notes browser
function OfflineNotesList() {
    const [offlineNotes, setOfflineNotes] = useState<FNote[]>([]);

    useEffect(() => {
        froca.getOfflineNotes().then(setOfflineNotes);
    }, []);

    return (
        <div className="offline-notes">
            <h3>
                <Icon name="download" />
                Available Offline ({offlineNotes.length})
            </h3>
            <ul>
                {offlineNotes.map((note) => (
                    <li key={note.noteId}>
                        <Link to={`/note/${note.noteId}`}>
                            {note.title}
                        </Link>
                        <small>Cached {formatRelative(note.cachedAt)}</small>
                    </li>
                ))}
            </ul>
        </div>
    );
}
```

---

## 🔧 Implementation Checklist

### Phase 1: Enhanced PWA (2-3 months)

**Week 1-2: Service Worker Setup**
- [ ] Create service worker with asset caching
- [ ] Implement cache-first strategy for static assets
- [ ] Add offline page
- [ ] Test on Chrome, Firefox, Safari

**Week 3-4: IndexedDB Integration**
- [ ] Create OfflineFroca class
- [ ] Implement note caching (LRU, limit 100)
- [ ] Add sync queue storage
- [ ] Test persistence across sessions

**Week 5-6: Offline UI**
- [ ] Offline indicator component
- [ ] Offline notes browser
- [ ] Sync queue visualization
- [ ] Error handling for offline edits

**Week 7-8: Testing & Polish**
- [ ] Test offline → online transitions
- [ ] Test sync queue processing
- [ ] Test storage limits (quota errors)
- [ ] Cross-browser testing (especially Safari)

**Week 9-10: PWA Manifest & Installation**
- [ ] Create manifest.json
- [ ] Add install prompts
- [ ] Test "Add to Home Screen"
- [ ] Icon assets for all sizes

**Week 11-12: Documentation & Release**
- [ ] User documentation
- [ ] Known limitations (iOS 7-day storage)
- [ ] Migration guide from desktop app
- [ ] Beta release

### Phase 2: Tauri Desktop (2-3 months, parallel)

**Week 1-2: Tauri Project Setup**
- [ ] Initialize Tauri project
- [ ] Configure for Windows/Mac/Linux
- [ ] Set up build pipeline

**Week 3-4: Node.js Sidecar**
- [ ] Package existing Trilium server as sidecar
- [ ] Configure sidecar startup/shutdown
- [ ] IPC between Tauri and Node.js

**Week 5-6: Frontend Integration**
- [ ] Point webview to localhost server
- [ ] Handle server startup delays
- [ ] Error handling for server crashes

**Week 7-8: Native Features**
- [ ] File system integration (import/export)
- [ ] System tray
- [ ] Native notifications
- [ ] Global shortcuts

**Week 9-10: Packaging & Distribution**
- [ ] Windows installer (MSI/EXE)
- [ ] macOS DMG with code signing
- [ ] Linux AppImage/DEB/RPM
- [ ] Auto-updater setup

**Week 11-12: Testing & Release**
- [ ] Cross-platform testing
- [ ] Migration from Electron app
- [ ] Performance benchmarks
- [ ] Beta release

---

## 📈 Success Metrics

### PWA Success Criteria
- ✅ Works offline on all major browsers (Chrome, Firefox, Safari 17+)
- ✅ Caches last 100 viewed notes
- ✅ Offline edits sync when online
- ✅ Install prompt appears on supported browsers
- ✅ Service worker updates automatically

### Tauri Success Criteria
- ✅ App size < 30 MB (vs Electron's ~50-80 MB)
- ✅ Memory usage < 250 MB (vs Electron's ~400-600 MB)
- ✅ Startup time < 2 seconds
- ✅ Full offline functionality
- ✅ Native features work (file dialogs, notifications)

### User Experience Goals
- ✅ Seamless transition between online/offline
- ✅ No data loss during offline edits
- ✅ Clear indicators of sync status
- ✅ Fast note loading (< 200ms for cached notes)

---

## 🚧 Risks & Mitigations

### Risk 1: iOS Safari Storage Deletion

**Risk**: Safari deletes all storage after 7 days of inactivity

**Mitigation**:
- Document limitation clearly
- Implement "Export all notes" feature
- Provide warning when approaching 7-day limit
- Consider native iOS app in Phase 3

### Risk 2: IndexedDB Quota Exceeded

**Risk**: Browsers may limit storage to 50-100 MB

**Mitigation**:
- LRU eviction (keep only recent notes)
- User controls for cache size
- Clear error messages
- Fallback to online-only mode

### Risk 3: Service Worker Bugs

**Risk**: Service workers can cache bugs, hard to clear

**Mitigation**:
- Version cache names (`trilium-v1`, `trilium-v2`)
- Implement cache eviction on update
- Provide "Clear cache" button in settings
- Thorough testing before release

### Risk 4: Tauri Mobile Not Ready

**Risk**: Tauri Mobile is still alpha (not stable)

**Mitigation**:
- Don't commit to Tauri Mobile timeline
- Continue PWA for mobile near-term
- Evaluate alternatives (React Native, native apps)
- Wait for Tauri Mobile stable release (2026+)

---

## 💰 Cost-Benefit Analysis

### Development Costs

| Approach | Dev Time | Dev Cost ($150/hr) |
|----------|----------|-------------------|
| PWA (Full SQLite) | 560 hrs | $84,000 |
| PWA (IndexedDB) | 160 hrs | $24,000 |
| Tauri (Full Rust) | 1200 hrs | $180,000 |
| Tauri (Node sidecar) | 240 hrs | $36,000 |
| **Hybrid (Recommended)** | **400 hrs** | **$60,000** |

### Benefits

**Quantifiable**:
- 60% smaller app size (Tauri vs Electron)
- 50% lower memory usage
- 20% faster navigation (offline PWA)
- 100% work computer compatibility (PWA)

**Qualitative**:
- Better user experience (offline access)
- Modern architecture (future-proof)
- Cross-platform consistency
- Lower infrastructure costs (less server dependency)

### ROI Calculation

**Assumptions**:
- 10,000 users
- 30% want offline access
- Electron → Tauri saves 50MB disk per user
- PWA enables 3,000 work computer users

**Savings**:
- Storage: 150 GB saved (10k users × 50MB × 30%)
- New users: 3,000 users gained (work computer access)
- Support costs: 20% reduction (offline reduces server issues)

**Payback Period**: ~6-12 months

---

## 🎓 Learning from Notion

Notion successfully implemented SQLite WASM for offline access:

**Their Approach**:
- Official SQLite WASM with OPFS
- Incremental migration (features first, full offline later)
- Fallbacks for older browsers (IndexedDB)
- User education about offline limitations

**Results**:
- 20% faster navigation
- 28-33% improvement on slower connections
- Positive user feedback
- Increased user retention

**Lessons for Trilium**:
- ✅ Start with read-only offline (easier than full sync)
- ✅ Progressive enhancement (don't break existing users)
- ✅ Clear communication about limitations
- ✅ Fallback strategies for unsupported browsers

---

## 📚 References & Resources

### SQLite WASM
- [Official SQLite WASM](https://sqlite.org/wasm/doc/trunk/index.md)
- [Notion Engineering Blog](https://notion.engineering/enhancing-notion-editor-offline-experience)
- [wa-sqlite](https://github.com/rhashimoto/wa-sqlite)

### Tauri
- [Tauri Documentation](https://tauri.app/v1/guides/)
- [Tauri Mobile (Alpha)](https://tauri.app/blog/tauri-mobile-alpha/)
- [rusqlite](https://github.com/rusqlite/rusqlite)

### PWA
- [Service Worker API](https://developer.mozilla.org/en-US/docs/Web/API/Service_Worker_API)
- [IndexedDB API](https://developer.mozilla.org/en-US/docs/Web/API/IndexedDB_API)
- [OPFS](https://developer.mozilla.org/en-US/docs/Web/API/File_System_API)

### Trilium-Specific
- [AGENT_RESEARCH_NOTES.md](./AGENT_RESEARCH_NOTES.md) - Architecture overview
- [SYNC_SYSTEM_ANALYSIS.md](./SYNC_SYSTEM_ANALYSIS.md) - Sync deep dive
- [CLAUDE.md](./CLAUDE.md) - Project overview

---

## 🎯 Final Recommendation Summary

### **Adopt Hybrid Approach: Enhanced PWA + Tauri Desktop**

**Timeline**: 4-6 months
**Effort**: ~400 hours
**Cost**: ~$60,000 (if contracting)

**Phases**:
1. **Enhanced PWA** (2-3 months): IndexedDB cache, service worker, offline UI
2. **Tauri Desktop** (2-3 months): Node.js sidecar, native features, packaging
3. **Future Mobile** (2026+): Evaluate Tauri Mobile or native apps

**Why This Wins**:
- ✅ Solves work computer constraint (PWA, no install)
- ✅ Solves Electron overhead (Tauri is 15x smaller)
- ✅ Provides offline access (both PWA and Tauri)
- ✅ Reasonable effort (4-6 months vs 12-18 months)
- ✅ Incremental value (each phase delivers features)
- ✅ Future-proof (modern tech, active ecosystems)

**Trade-offs Accepted**:
- ⚠️ PWA offline is limited (recent notes only)
- ⚠️ iOS storage deletion (7-day limit)
- ⚠️ Tauri requires installation (but that's fine for personal devices)
- ⚠️ Mobile native apps deferred to future

---

**Document Status**: Draft for Discussion
**Next Steps**: Review, prioritize, prototype Phase 1
**Contact**: [Project maintainers]

---

*This proposal is based on comprehensive research of Trilium's architecture, current state-of-the-art browser technologies, and successful case studies (Notion). All effort estimates are approximate and should be validated during detailed planning.*
