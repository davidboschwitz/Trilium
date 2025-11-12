# Trilium Notes - Comprehensive Node.js to Rust Migration Analysis

## Executive Summary

Trilium Notes is a complex, feature-rich note-taking application with a sophisticated Node.js backend. A complete Rust rewrite would be a **major undertaking (6-18 months for a small team)** with significant architectural challenges, particularly around user scripting, complex synchronization logic, and the extensive LLM integration system.

---

## 1. CORE SERVICES ANALYSIS

### 1.1 Critical Service Inventory

#### **SIMPLE Complexity Services** (Easy to port)
- `config.ts` - Configuration management
- `date_utils.ts` - Date utilities (dayjs wrapper)
- `data_dir.ts` - Directory handling
- `instance_id.ts` - Instance tracking
- `log.ts` - Logging
- `app_info.ts` - Application metadata
- `port.ts` - Port configuration
- `resource_dir.ts` - Resource directory handling
- `asset_path.ts` - Asset path management
- `host.ts` - Host configuration
- `session_secret.ts` - Session key management
- `sanitize_attribute_name.ts` - Input validation

**Porting Effort**: 1-2 weeks
**Rust Equivalent**: Standard libraries + logging crates (tracing, log)

---

#### **MEDIUM Complexity Services** (Moderate porting effort)

| Service | Lines | Dependencies | Challenge | Est. Effort |
|---------|-------|--------------|-----------|-------------|
| `sql.ts` | 438 | better-sqlite3 | Prepared statement caching, transaction model | 3-4 weeks |
| `sync.ts` | 465 | Multiple services | Complex sync state machine, conflict resolution | 6-8 weeks |
| `attributes.ts` | 115 | sql, becca, events | Attribute system with inheritance | 2-3 weeks |
| `branches.ts` | 50 | sql, becca | Tree structure management | 1-2 weeks |
| `options.ts` | ~150 | sql | Configuration storage | 1 week |
| `revisions.ts` | ~200 | sql, becca | Version history system | 2 weeks |
| `image.ts` | ~150 | jimp, image-type, is-animated | Image processing with compression | 3-4 weeks |
| `html_sanitizer.ts` | ~100 | sanitize-html | HTML sanitization | 1-2 weeks |
| `backup.ts` | ~200 | archiver, fs-extra | Backup creation/restoration | 2-3 weeks |
| `protected_session.ts` | ~200 | crypto, encryption | Session encryption management | 2-3 weeks |

**Subtotal Porting Effort**: 24-34 weeks

---

#### **COMPLEX Complexity Services** (Very difficult to port)

| Service | Lines | Dependencies | Challenge | Est. Effort |
|---------|-------|--------------|-----------|-------------|
| `notes.ts` | 1,086 | 30+ services | Core note CRUD, attachment mgmt, cascading deletes | 8-10 weeks |
| `tree.ts` | 280 | sql, becca, attributes | Hierarchical tree operations, path management | 3-4 weeks |
| `search.ts` (services/search/services/search.ts) | 796 | 35+ expression types | Full-text search with FTS5, complex query parsing | 8-10 weeks |
| **Search subsystem** | 35 files | Custom expression language | Entire search engine: parsers, lexers, evaluators | 12-15 weeks |
| `backend_script_api.ts` | 728 | All services | **CANNOT PORT** - Executes user Node.js code | ❌ N/A |
| `sync_update.ts` | ~400 | sql, sync, entities | Entity change tracking and propagation | 4-5 weeks |
| `cloning.ts` | ~300 | notes, branches, attributes | Note cloning with relationship duplication | 3-4 weeks |
| `entity_changes.ts` | ~150 | sql | Entity change tracking system | 2 weeks |
| `bulk_actions.ts` | ~200 | notes, branches | Batch operations on note hierarchies | 2-3 weeks |

**Subtotal Porting Effort**: 47-62 weeks

---

#### **CRITICAL - LLM Integration System** (NEW - Major Subsystem)

The LLM system is **extremely complex** (97+ files) and newer than most of the core:

```
apps/server/src/services/llm/
├── base services (6 files) - AI provider abstraction
├── chat/ (11 files) - Chat interface and streaming
├── config/ (2 files) - LLM configuration
├── constants/ (6 files) - Prompt/format constants
├── context/ (8 files) - Context extraction and chunking
├── context_extractors/ (3 files) - Tool implementations
├── formatters/ (2 files) - Message formatting per provider
├── interfaces/ (8 files) - TypeScript interfaces
├── pipeline/ (10 files) - Multi-stage processing pipeline
├── providers/ (10 files) - OpenAI, Ollama, Anthropic SDKs
├── tools/ (14 files) - Agent tools (search, note creation, etc.)
├── utils/ (2 files) - Utilities
└── Test files (8 files)
```

**Est. Porting Effort**: 14-18 weeks (NEW DEVELOPMENT, not porting)

---

### 1.2 Service Dependency Map

```
Entry Point (main.ts → www.ts)
├─ initializeTranslations() [i18next]
└─ HTTP Server Setup
   ├─ sql.ts [better-sqlite3]
   │  └─ Database Connection, Statement Cache, Transactions
   │
   ├─ auth.ts [password, totp, openid]
   │  └─ Encryption services
   │
   ├─ config.ts
   │  └─ Options service
   │
   ├─ routes.ts [Express]
   │  ├─ API routes (405 endpoints across 61 files)
   │  ├─ ETAPI routes (14 files)
   │  └─ Share/public routes
   │
   ├─ becca.ts [Becca cache loader]
   │  ├─ BNote, BBranch, BAttribute, BAttachment, etc.
   │  └─ Loaded from database at startup
   │
   ├─ notes.ts
   │  ├─ attributes.ts
   │  ├─ branches.ts
   │  ├─ tree.ts
   │  ├─ revisions.ts
   │  ├─ image.ts
   │  ├─ cloning.ts
   │  ├─ bulk_actions.ts
   │  └─ protected_session.ts
   │
   ├─ search.ts
   │  ├─ Expression evaluator (35 expression types)
   │  ├─ Full-text search (FTS5)
   │  └─ Query parser/lexer
   │
   ├─ sync.ts
   │  ├─ sync_update.ts
   │  ├─ sync_options.ts
   │  ├─ sync_mutex.ts
   │  ├─ entity_changes.ts
   │  ├─ content_hash.ts
   │  └─ request.ts [HTTP client - axios]
   │
   ├─ ws.ts [WebSocket Server]
   │  └─ Real-time updates to connected clients
   │
   ├─ backup.ts [archiver, fs-extra]
   │
   ├─ import/ (7 files)
   │  ├─ Single file import
   │  ├─ ZIP import
   │  ├─ Markdown import
   │  ├─ ENEX (Evernote) import
   │  ├─ OPML import
   │  └─ HTML import
   │
   ├─ export/ (10 files)
   │  ├─ Single note export
   │  ├─ ZIP export (HTML/Markdown/PDF)
   │  └─ OPML/Markdown export
   │
   ├─ encryption/ (7 files)
   │  ├─ password.ts [scrypt password hashing]
   │  ├─ password_encryption.ts
   │  ├─ data_encryption.ts
   │  ├─ totp_encryption.ts
   │  ├─ open_id_encryption.ts
   │  ├─ my_scrypt.ts [custom scrypt wrapper]
   │  └─ recovery_codes.ts
   │
   ├─ llm/ (97+ files)
   │  ├─ AI service abstraction (OpenAI, Ollama, Anthropic)
   │  ├─ Chat service with streaming
   │  ├─ Tool-based agent system
   │  ├─ Context extraction pipeline
   │  └─ Multiple formatting options
   │
   ├─ share/ (15 files)
   │  ├─ Shaca cache for published notes
   │  ├─ Share root management
   │  ├─ Content renderer for public pages
   │  └─ Share-specific SQL layer
   │
   ├─ openai.ts, ollama.ts, anthropic.ts [LLM provider configs]
   │
   ├─ migrations/ (4 migration files)
   │  └─ Database schema evolution
   │
   ├─ events.ts [Event emitter system]
   │
   ├─ backend_script_api.ts ❌ CANNOT PORT
   │  └─ User-defined Node.js scripts execution
   │
   └─ Various utilities...
```

---

## 2. DATABASE LAYER ANALYSIS

### 2.1 Schema Overview

**12 Core Tables**:
1. `notes` - Note records with metadata (title, type, mime, protected flag)
2. `branches` - Tree structure (noteId, parentNoteId, position, expand state)
3. `attributes` - Key-value metadata (inheritable labels/relations)
4. `revisions` - Version history of notes
5. `blobs` - Large content storage (separate from notes table)
6. `attachments` - File attachments to notes
7. `recent_notes` - User's recently accessed notes
8. `entity_changes` - Sync change tracking
9. `options` - Global configuration key-value store
10. `etapi_tokens` - External API tokens
11. `sessions` - HTTP session storage
12. `user_data` - Authentication/encryption keys

### 2.2 Current Usage Pattern (better-sqlite3)

```typescript
// Prepared statement caching
const stmt = dbConnection.prepare(sql);
const result = stmt.all(params);

// Batch operations with param limit (100)
const getManyRows = (query, params) => {
  // Batches large param sets to avoid statement complexity
  while (params.length > 0) {
    const batch = params.slice(0, 100);
    // Replace ??? placeholders with :param1, :param2, etc.
  }
}

// Transactions with better-sqlite3
dbConnection.transaction(() => {
  // Multiple operations in transaction
})();

// WAL mode for concurrency
dbConnection.pragma("journal_mode = WAL");
```

### 2.3 Rust Equivalent (rusqlite)

**Challenges**:
1. **Prepared statement caching** - rusqlite doesn't have as efficient caching as better-sqlite3
2. **Batch parameter handling** - Need custom query rewriting logic
3. **Connection pooling** - better-sqlite3 is single-threaded; rusqlite needs r2d2 or sqlx for async
4. **Transaction API** - Different savepoint/rollback semantics

**Recommendation**: Use **sqlx** instead of rusqlite for async/await support and better connection pooling.

---

## 3. EXPRESS API ANALYSIS

### 3.1 Route Statistics
- **Total endpoints**: 405 (counted from grep)
- **Route files**: 61 files
- **Main route groups**:

| Group | Files | Endpoints | Category |
|-------|-------|-----------|----------|
| Notes API | notes.ts | ~20 | CRUD + metadata |
| Branches API | branches.ts | ~8 | Tree operations |
| Attributes API | attributes.ts | ~10 | Metadata |
| Search API | search.ts | ~5 | Full-text search |
| Sync API | sync.ts | ~8 | Synchronization |
| Tree API | tree.ts | ~5 | Hierarchy |
| Attachments API | attachments.ts | ~6 | File management |
| Export/Import API | export.ts, import.ts | ~15 | Data portability |
| Script API | script.ts | ~5 | User script execution |
| Files API | files.ts | ~10 | File operations |
| Setup API | setup.ts, login.ts | ~10 | Auth |
| LLM APIs | ollama.ts, openai.ts, anthropic.ts, llm.ts | ~20 | AI integration |
| ETAPI | 14 files | ~100+ | External API (RESTful) |
| Share/Public | share/routes.ts | ~30 | Published notes |
| Other | various | ~150+ | Stats, metrics, health, etc. |

### 3.2 Middleware Used

1. **Authentication**: `auth.checkAuth`, `auth.checkApiAuthOrElectron`
2. **Session**: `express-session` with store
3. **CSRF**: `csrf-csrf` (double CSRF protection)
4. **Rate Limiting**: `express-rate-limit` (15-min windows, 10 attempts)
5. **Compression**: `compression` middleware
6. **File Upload**: `multer` with 250MB max size
7. **Logging**: Custom via `log` service
8. **CORS**: Implicit via same-origin requirement

### 3.3 Recommended Rust Web Framework

**Axum** is best choice:
- Async/await with Tokio
- Type-safe middleware
- Built-in JSON support
- Easy integration with sqlx
- Good ecosystem for compression, sessions, CSRF
- However: smaller ecosystem than Express, fewer pre-built middlewares

**Actix-web** alternative:
- More mature, larger ecosystem
- Higher performance
- Steeper learning curve
- Good for high-concurrency scenarios

---

## 4. DEPENDENCY INVENTORY & RUST EQUIVALENTS

### 4.1 Runtime Dependencies (3 listed, many devDependencies)

```json
"better-sqlite3": "12.4.1"
"html-to-text": "9.0.5"
"node-html-parser": "7.0.1"
```

### 4.2 DevDependencies (60+ packages)

**Database & Storage**:
- ✅ `better-sqlite3` → `rusqlite` or `sqlx` + `tokio`
- ✅ `archiver` → `tar`, `zip` crates

**Web Framework & HTTP**:
- ✅ `express` → `axum` or `actix-web`
- ✅ `express-session` → `axum-sessions` or custom
- ✅ `cookie-parser` → built into web frameworks
- ✅ `compression` → `tower-http`
- ✅ `cors` → `tower-http`
- ✅ `helmet` → `tower-http` security headers
- ✅ `express-rate-limit` → `tower-governor`
- ✅ `csrf-csrf` → `tower-sessions` + custom CSRF
- ✅ `axios` → `reqwest` or `http`
- ✅ `multer` → `tower-multipart` or custom

**HTML/XML Processing**:
- ✅ `html-to-text` → `html2text` crate
- ✅ `node-html-parser` → `scraper` or `html5ever`
- ✅ `cheerio` → `scraper`
- ✅ `sanitize-html` → `ammonia`
- ✅ `xml2js` → `serde_xml_rs` or `quick-xml`
- ✅ `html` → `html5ever`

**Image Processing**:
- ✅ `jimp` → `image` crate (PIL equivalent)
- ✅ `image-type` → `infer` crate
- ⚠️ `is-animated` → No direct equivalent, needs custom GIF detection
- ✅ `is-svg` → Simple string match with `xml2html` parsing

**File System**:
- ✅ `fs-extra` → `tokio::fs`, `walkdir`, `tempfile`
- ✅ `path` → `std::path`

**Date/Time**:
- ✅ `dayjs` → `chrono` (more feature-rich than dayjs)
- ✅ `dayjs/plugin/` → `chrono` built-in

**Utilities**:
- ✅ `marked` → `pulldown-cmark`
- ✅ `turndown` → `html2md` or `htmd`
- ✅ `mime-types` → `mime` crate
- ✅ `js-yaml` → `serde_yaml`
- ✅ `ini` → `configparser` or `ini` crate
- ✅ `sanitize-filename` → `sanitize-filename` crate
- ✅ `escape-html` → `ammonia` or `html-escape`
- ✅ `striptags` → `html2text` or scraper
- ✅ `normalize-strings` → custom or `unicode-normalization`
- ✅ `unescape` → `html-escape`
- ✅ `safe-compare` → `subtle::ConstantTimeComparison`

**Cryptography**:
- ✅ `time2fa` (TOTP) → `totp-rs`
- ⚠️ `openai`, `ollama`, `@anthropic-ai/sdk` → Maintain as HTTP clients

**i18n**:
- ✅ `i18next`, `i18next-fs-backend` → `fluent-rs` or `i18n-embed`

**OpenID/Auth**:
- ✅ `express-openid-connect` → `openidconnect` crate or maintain custom OAuth2

**Debugging**:
- ✅ `debug` → `env_logger`, `tracing`
- ✅ `electron-debug` → Not needed in backend

**WebSockets**:
- ✅ `ws` → `tokio-tungstenite` or `axum` with `axum::extract::ws`

**Other**:
- ✅ `debounce` → `tokio::time::sleep` with async
- ✅ `cls-hooked` → `tokio-context` or task-local storage

---

### 4.3 Dependency Migration Summary

| Category | Status | Effort |
|----------|--------|--------|
| Database | rusqlite/sqlx | Medium |
| Web Framework | Axum/Actix-web | High |
| HTML/XML | Good Rust equivalents | Low |
| Images | Good Rust equivalents | Low |
| Crypto | Good Rust equivalents | Low |
| LLM Client SDKs | Use HTTP directly | Medium |
| i18n | Rust solutions exist | Low |
| OpenID | Custom or existing crates | Medium |

**Total Missing Equivalents**: 2-3 (animated image detection, specific LLM SDK behaviors)

---

## 5. CRITICAL FEATURES ANALYSIS

### 5.1 Features That CAN'T Be Ported

#### **User Script Execution** ❌
- `backend_script_api.ts` (728 lines) allows users to write Node.js code
- Runs arbitrary JavaScript in backend context
- Has access to full API: `api.getNote()`, `api.search()`, `api.axios`, etc.
- **Solution options**:
  1. **Remove scripting entirely** - Simplest, loses feature
  2. **Rust-based scripting** - Use `rlua` or `rhai` (Lua/Rust native scripting)
  3. **Keep Node.js subprocess** - Call Node.js via stdin/stdout (messy)
  4. **WebAssembly scripting** - Compile user code to WASM (complex)

**Recommendation**: Switch to **rhai** scripting language (Rust-native, similar syntax to JavaScript)

### 5.2 Features That Need Complete Reimplementation

#### **Full-Text Search System** (35+ files, 800+ lines)
- SQLite FTS5 integration
- Custom expression language (19 expression types):
  - `ancestor:`, `child_of:`, `descendant_of:`, `parent_of:`
  - `attribute_exists:`, `label:`, `relation:`, `note:`
  - `type:`, `mime:`, `isProtected:`, `isDeleted:`
  - `hasNote:`, `hasOwnedAttribute:`, `hasRelation:`
  - `dateCreated:`, `dateModified:`, `utcDateModified:`
  - `content:`, `flatText:`, `order_by:`, `limit:`
  - Boolean operators: `AND`, `OR`, `NOT`
  - Custom comparators

**Effort**: 8-10 weeks to reimplement + test

#### **Synchronization Engine** (465+ lines of complex logic)
- Push/pull sync mechanism
- Conflict detection via content hashing
- Entity change tracking with IDs/hashes
- Retry logic with exponential backoff
- Mutex for preventing concurrent syncs
- State machine: login → push → pull → push → verify hash

**Effort**: 6-8 weeks + extensive testing

#### **Image Processing Pipeline**
- JIMP integration for compression
- Animated GIF detection (skip compression)
- SVG detection
- Format detection and conversion
- Async processing with callbacks

**Effort**: 3-4 weeks

### 5.3 Features That Are Straightforward

#### **Notes & Attributes System** ✅
- CRUD operations
- Attribute inheritance
- Protected sessions (encryption)
- Would need ~10 weeks

#### **Import/Export** ✅
- ENEX, OPML, Markdown, HTML, ZIP formats
- ~7-10 weeks with format libraries

#### **Encryption** ✅
- Password hashing with scrypt
- TOTP generation
- OpenID integration
- ~3-4 weeks (good Rust crypto libraries)

#### **LLM Integration** ✅ (but large)
- Wraps existing OpenAI/Ollama/Anthropic APIs
- ~14-18 weeks for feature parity

---

## 6. ARCHITECTURAL MISMATCHES

### 6.1 JavaScript Runtime Assumptions
1. **Single-threaded event loop** - Express assumes single thread with async/await
   - Rust async is different (Tokio-based)
   - Need to be careful with blocking operations

2. **Dynamic typing** - Entity objects created dynamically
   - Rust requires strong typing
   - Need struct definitions for all 11 entity types

3. **Prepared statement caching**
   - better-sqlite3 has efficient caching built-in
   - Rust requires explicit management or connection pool

4. **Synchronous-looking code** - async/await in Node.js is syntactic sugar
   - Rust async/await is more explicit

### 6.2 Transaction & Concurrency Model
- better-sqlite3 is **single-threaded** with WAL mode for reader/writer parallelism
- Rust with connection pooling will have different semantics
- Sync/Pull operations need careful locking (currently uses `syncMutexService`)

### 6.3 File Descriptor Management
- Node.js handles FD cleanup automatically (GC)
- Rust requires explicit cleanup (implement Drop traits)
- File uploads with multer are complex

---

## 7. EFFORT ESTIMATION BY COMPONENT

### Component Breakdown

| Component | Files | Complexity | Est. Weeks | Priority |
|-----------|-------|------------|------------|----------|
| **Database Layer** | 1 | Medium | 3-4 | P0 |
| **SQL Entity Layer** | 11 | Medium | 3-4 | P0 |
| **Core CRUD Services** | 6 | Medium | 5-6 | P0 |
| **Tree/Hierarchy System** | 3 | Medium | 3-4 | P0 |
| **Search Engine** | 35 | Hard | 8-10 | P0 |
| **Synchronization** | 8 | Hard | 6-8 | P0 |
| **Encryption/Auth** | 7 | Medium | 3-4 | P0 |
| **Image Processing** | 1 | Medium | 3-4 | P1 |
| **Import/Export** | 17 | Medium | 7-10 | P1 |
| **Backup System** | 1 | Medium | 2-3 | P2 |
| **WebSocket Server** | 1 | Easy | 1-2 | P1 |
| **API Endpoints** | 61 | Easy | 8-10 | P0 |
| **Middleware/Auth** | 5 | Easy | 2-3 | P0 |
| **LLM System** | 97 | Hard | 14-18 | P2 |
| **Share/Published Notes** | 15 | Medium | 3-4 | P2 |
| **Configuration/Utilities** | 20+ | Easy | 3-4 | P3 |
| **Testing Infrastructure** | - | Medium | 4-6 | P1 |
| **Scripting System** | 728 lines | Hard | 6-8 | P2 (rhai) |

**TOTAL: 95-135 weeks (2.3-3.2 years for one developer)**

### Realistic Team Effort
- **Small team (3-4 devs)**: 8-12 months
- **Medium team (5-6 devs)**: 5-8 months
- **Large team (8-10 devs)**: 3-4 months (with coordination overhead)

---

## 8. CRITICAL PATH ITEMS (Sequence Required)

```
Week 1-4:   Database layer (sql.ts → sqlx)
            ↓
Week 5-8:   Becca entity layer, CRUD ops
            ↓
Week 9-14:  Notes, Branches, Attributes services
            ↓
Week 15-22: Search engine (complex - parallel if possible)
Week 15-22: Sync engine (complex - parallel if possible)
            ↓
Week 23-28: API endpoints (all routes, can be parallel)
            ↓
Week 29-32: WebSocket server, real-time updates
            ↓
Week 33-38: Import/Export
            ↓
Week 39-44: LLM integration (or skip for MVP)
            ↓
Week 45+:   Testing, bug fixes, performance tuning
```

---

## 9. FEATURE LOSS / REDUCTION

### What CANNOT be preserved:
1. **Backend JavaScript scripting** (complete feature loss unless rhai substituted)
   - Impacts power users
   - Plugin/extension ecosystem lost

### What MUST be reimplemented:
1. **Search expression language** (19 operators)
2. **Sync algorithm** (complex conflict resolution)
3. **Encryption system** (needs different libs)

### What Would Be Difficult to Port:
1. **LLM streaming** (complex async handling)
2. **Animated image detection** (no direct Rust equivalent)
3. **Some import formats** (ENEX is Evernote proprietary)

---

## 10. RECOMMENDED RUST TECH STACK

```
Frontend: (keep as-is)
├─ React/TypeScript (apps/client/)
├─ Vite build
└─ WebSocket client

Backend (Rust):
├─ Runtime: Tokio (async runtime)
├─ Web Framework: Axum
├─ Database: sqlx + sqlx-sqlite
├─ WebSocket: tokio-tungstenite + axum::extract::ws
├─ Serialization: serde + serde_json
├─ Logging: tracing + tracing-subscriber
├─ Error Handling: anyhow + thiserror
├─ Crypto: argon2 + ring + totp-rs
├─ HTML/XML: scraper + ammonia
├─ Markdown: pulldown-cmark
├─ Images: image + infer
├─ HTTP Client: reqwest
├─ File Handling: tokio::fs + walkdir + tempfile
├─ Date/Time: chrono
├─ Config: serde_yaml + serde_ini
├─ i18n: fluent-rs
├─ Scripting: rhai (for user scripts)
└─ Testing: criterion + proptest + tokio::test

Deployment:
├─ Docker: Multi-stage build
├─ Binary size: ~50-80MB (vs 200MB+ Node.js bundle)
├─ Memory: 100-200MB at runtime (vs 400-800MB Node.js)
├─ Performance: 2-3x faster for I/O operations
```

---

## 11. RISK ASSESSMENT

### High Risk Items
1. **Scripting System** - Must decide on rhai vs feature removal
2. **Search Engine** - Complex expression language, many edge cases
3. **Sync Algorithm** - Highly stateful, easy to introduce bugs
4. **Encryption** - Any mistakes = data loss

### Medium Risk Items
1. **LLM Integration** - Streaming, error handling, multiple providers
2. **Database migrations** - Schema changes, data compatibility
3. **File upload handling** - Memory management with multer equivalent

### Low Risk Items
1. **CRUD operations** - Straightforward to port
2. **API endpoints** - Just route wiring
3. **Configuration** - Standard Rust patterns

---

## 12. ALTERNATIVES TO FULL REWRITE

### Option A: Hybrid Approach (Recommended)
- Keep Node.js backend for **complex features** (search, sync, scripting)
- Write new features in Rust
- Use gRPC or REST bridges
- Gradual migration over 2-3 years
- **Timeline**: 6-12 months for first hybrid version

### Option B: Module Extraction
- Extract critical components to shared libraries
- Reimplement database layer in Rust (biggest bottleneck)
- Keep scripting in Node.js
- **Timeline**: 3-4 months

### Option C: WebAssembly Port
- Compile Node.js backend to WASM
- Run in browser/Electron
- No server needed for some features
- **Timeline**: 2-3 months (high risk)

### Option D: Complete Rewrite (Original Ask)
- All-in Rust
- **Timeline**: 8-18 months
- **Risk**: High (large codebase, many features)

---

## 13. ESTIMATED COMPLEXITY BREAKDOWN

### Line Count Analysis
- **Server TypeScript**: ~50,000 lines (excluding tests)
- **Server node_modules**: ~300,000+ files (30GB+)
- **Estimated Rust equivalent**: ~80,000-120,000 lines (more verbose due to type system)

### Service Complexity Distribution
```
Easy (10-15%):        Utilities, config, logging
Medium (30-40%):      CRUD, attributes, files, backup
Hard (35-50%):        Search, sync, scripting, LLM, encryption
Unknown (5-15%):      Edge cases, performance characteristics
```

---

## 14. DECISION MATRIX

| Decision Point | Rust Rewrite | Hybrid | Keep Node.js |
|----------------|--------------|--------|--------------|
| **Long-term perf** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐ |
| **Development speed** | ⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐ |
| **Memory usage** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐ |
| **Maintainability** | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐ |
| **Feature parity** | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| **Risk** | ⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐ |
| **Time to complete** | ⭐ (18mo) | ⭐⭐⭐ (6mo) | ⭐⭐⭐⭐⭐ (0) |

---

## CONCLUSION

A **complete Rust rewrite of Trilium's Node.js backend is feasible but represents an 8-18 month investment** for a small team. The main challenges are:

1. **Complexity**: 50,000+ lines across 100+ files with intricate dependencies
2. **Scripting**: User script execution requires either rhai substitution or significant architectural change
3. **Search/Sync**: Custom query language and stateful sync logic are inherently complex
4. **Testing**: Extensive test coverage needed for data integrity

**Recommendation**: Pursue a **hybrid approach** for the first 6-12 months, focusing on rewriting the database layer and critical services in Rust while keeping complex features in Node.js. This reduces risk and provides incremental benefits.

