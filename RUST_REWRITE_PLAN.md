# Trilium Rust Rewrite - Implementation Plan

**Status**: Pre-Planning Phase
**Timeline**: 8-12 months (team of 4-6 developers)
**Effort**: ~2,080-2,800 developer-hours
**Risk**: HIGH

---

## 🎯 Executive Summary

Complete Rust rewrite of Trilium's Node.js backend to enable:
- Mobile support (iOS/Android via Tauri Mobile)
- Smaller binary size (50-80 MB vs 200+ MB)
- Better performance (2-3x faster I/O)
- Lower memory usage (50-75% reduction)

**Key Tradeoff**: Loses user backend scripts feature (728 lines of Node.js execution)

---

## 📊 Scale Assessment

### Codebase Statistics
- **50,000+ lines** of TypeScript server code
- **405 API endpoints** across 61 route files
- **79 service files** (excluding tests)
- **97 LLM integration files** (newest subsystem)
- **12 database tables** with complex relationships

### Effort Breakdown

| Complexity Tier | Components | Effort (weeks) | % of Total |
|----------------|------------|----------------|------------|
| **Simple** | 12 utility services | 1-2 | 2% |
| **Medium** | 10 core services | 24-34 | 30% |
| **Complex** | 9 critical systems | 47-62 | 58% |
| **New Development** | LLM system | 14-18 | 17% |
| **Testing & QA** | Full test suite | 10-15 | - |
| **TOTAL** | **95-135 weeks** | **~2.5 years solo** |

### Realistic Team Timeline

| Team Size | Timeline | Notes |
|-----------|----------|-------|
| 1 developer | 2.3-3.2 years | Not recommended |
| 2 developers | 14-18 months | High risk |
| 3-4 developers | **8-12 months** | Recommended |
| 5-6 developers | 5-8 months | Diminishing returns |
| 8-10 developers | 3-4 months | Communication overhead |

**Recommended**: **4-person team, 10 months**

---

## 🗺️ Phased Implementation Roadmap

### Phase 1: Foundation (Weeks 1-8)

**Goal**: Core infrastructure + database layer

**Team Structure**:
- Lead: Database & API architect
- Dev 1: Database layer (rusqlite/sqlx)
- Dev 2: HTTP server (Axum/Actix)
- Dev 3: Utilities & logging

**Deliverables**:
```rust
// Database layer
mod database {
    use sqlx::SqlitePool;

    pub struct Database {
        pool: SqlitePool,
        prepared_cache: HashMap<String, PreparedStatement>,
    }

    impl Database {
        pub async fn get_note(&self, note_id: &str) -> Result<Note>;
        pub async fn create_note(&self, note: &Note) -> Result<()>;
        pub async fn transaction<F>(&self, f: F) -> Result<()>;
    }
}

// HTTP server
mod server {
    use axum::{Router, routing::get};

    pub fn app() -> Router {
        Router::new()
            .route("/api/notes/:id", get(get_note))
            .route("/api/notes/:id", put(update_note))
            .route("/api/notes/:id", delete(delete_note))
    }
}
```

**Milestones**:
- [ ] Week 2: Database connection + basic CRUD
- [ ] Week 4: Transaction support + prepared statements
- [ ] Week 6: HTTP server + 50 basic endpoints
- [ ] Week 8: Integration tests passing

**Risk**: Database layer complexity (prepared statement caching)

---

### Phase 2: Core Entities (Weeks 9-16)

**Goal**: Notes, Branches, Attributes systems

**Team Structure**:
- Dev 1: Notes service (1,086 lines to port)
- Dev 2: Branches + tree operations
- Dev 3: Attributes + inheritance
- QA: Test suite development

**Deliverables**:
```rust
// Note entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub note_id: String,
    pub title: String,
    pub type_: NoteType,
    pub mime: String,
    pub is_protected: bool,
    pub blob_id: Option<String>,
    pub date_created: DateTime<Utc>,
    pub date_modified: DateTime<Utc>,
}

impl Note {
    pub async fn save(&self, db: &Database) -> Result<()>;
    pub async fn delete(&self, db: &Database) -> Result<()>;
    pub async fn get_content(&self, db: &Database) -> Result<Vec<u8>>;
    pub async fn set_content(&mut self, content: Vec<u8>) -> Result<()>;
}

// Branch entity (parent-child relationships)
pub struct Branch {
    pub branch_id: String,
    pub note_id: String,
    pub parent_note_id: String,
    pub note_position: i32,
    pub prefix: Option<String>,
}

// Attribute entity (labels & relations)
pub struct Attribute {
    pub attribute_id: String,
    pub note_id: String,
    pub type_: AttributeType,  // Label or Relation
    pub name: String,
    pub value: String,
    pub is_inheritable: bool,
}
```

**Milestones**:
- [ ] Week 10: Note CRUD complete
- [ ] Week 12: Branch operations + tree traversal
- [ ] Week 14: Attributes + inheritance logic
- [ ] Week 16: 150+ API endpoints implemented

**Risk**: Cascading delete complexity in notes.ts

---

### Phase 3: Search Engine (Weeks 17-28)

**Goal**: Full-text search with custom expression language

**Team Structure**:
- Lead: Search architect (design query language)
- Dev 1: Lexer + parser
- Dev 2: Expression evaluators (19 types)
- Dev 3: FTS5 integration

**Challenge**: Trilium has **custom search language** with 19 operators:
```
#todo                          // Label search
~task                          // Relation search
note.title *= "meeting"        // Fuzzy title search
note.content *= "client"       // Content search
note.dateCreated >= MONTH-1    // Date arithmetic
#priority = high AND ~parent   // Boolean logic
```

**Deliverables**:
```rust
// Search expression AST
pub enum SearchExpr {
    LabelComparison { name: String, value: String, op: CompareOp },
    RelationComparison { name: String, target: String },
    PropertySearch { property: String, value: String, op: SearchOp },
    DateComparison { property: String, date: DateTime<Utc>, op: CompareOp },
    And(Box<SearchExpr>, Box<SearchExpr>),
    Or(Box<SearchExpr>, Box<SearchExpr>),
    Not(Box<SearchExpr>),
}

// Search service
pub struct SearchService {
    db: Database,
}

impl SearchService {
    pub async fn search(&self, expr: &str) -> Result<Vec<Note>> {
        let ast = parse_expression(expr)?;
        let sql = ast_to_sql(&ast)?;
        self.db.query_notes(&sql).await
    }
}
```

**Milestones**:
- [ ] Week 19: Lexer + parser for search expressions
- [ ] Week 21: 10 basic expression types working
- [ ] Week 24: All 19 expression types implemented
- [ ] Week 26: FTS5 full-text search integrated
- [ ] Week 28: Performance optimization (indexing)

**Risk**: HIGH - Complex custom language, no reference implementation in Rust

---

### Phase 4: Synchronization (Weeks 29-36)

**Goal**: Multi-device sync with conflict resolution

**Team Structure**:
- Lead: Sync protocol architect
- Dev 1: Entity change tracking
- Dev 2: Content hashing + diff
- Dev 3: Conflict resolution

**Challenge**: Complex state machine with 5 phases:
1. login() - Authentication
2. pushChanges() - Send local changes
3. pullChanges() - Receive remote changes
4. pushChanges() - Re-send triggered changes
5. checkContentHash() - Verify consistency

**Deliverables**:
```rust
// Sync service
pub struct SyncService {
    db: Database,
    state: SyncState,
}

pub enum SyncState {
    Idle,
    Login,
    PushFirst,
    Pull,
    PushSecond,
    Finished,
    CheckHash,
}

impl SyncService {
    pub async fn sync(&mut self) -> Result<SyncResult> {
        self.login().await?;
        self.push_changes().await?;
        self.pull_changes().await?;
        self.push_changes().await?;  // Second pass
        self.sync_finished().await?;
        self.check_content_hash().await
    }

    async fn resolve_conflict(&self, local: &Entity, remote: &Entity) -> Entity {
        // Last-Writer-Wins based on timestamp
        if local.utc_date_modified <= remote.utc_date_modified {
            remote.clone()  // Remote wins (deterministic)
        } else {
            local.clone()   // Local wins, re-queue for push
        }
    }
}
```

**Milestones**:
- [ ] Week 30: Entity change tracking
- [ ] Week 32: Push/pull mechanism
- [ ] Week 34: Conflict resolution logic
- [ ] Week 36: Multi-device testing

**Risk**: State machine complexity, edge cases

---

### Phase 5: LLM Integration (Weeks 37-50)

**Goal**: AI assistant with multiple providers

**Team Structure**:
- Lead: LLM architect
- Dev 1: Provider adapters (OpenAI, Anthropic, Ollama)
- Dev 2: Context extraction + chunking
- Dev 3: Tool system (search, note creation)

**Challenge**: **97 files** in LLM subsystem - newest, most complex

**Deliverables**:
```rust
// LLM provider trait
#[async_trait]
pub trait LlmProvider {
    async fn chat(&self, messages: &[Message]) -> Result<String>;
    async fn stream_chat(&self, messages: &[Message]) -> Result<Stream<String>>;
    fn supports_tools(&self) -> bool;
}

// Provider implementations
pub struct OpenAiProvider {
    client: OpenAiClient,
    api_key: String,
}

pub struct AnthropicProvider {
    client: AnthropicClient,
    api_key: String,
}

pub struct OllamaProvider {
    base_url: String,
}

// Tool system
pub enum Tool {
    SearchNotes { query: String },
    CreateNote { title: String, content: String },
    UpdateNote { note_id: String, content: String },
    GetNoteContent { note_id: String },
}

// LLM service
pub struct LlmService {
    provider: Box<dyn LlmProvider>,
    tools: Vec<Tool>,
}
```

**Milestones**:
- [ ] Week 39: Provider abstraction + OpenAI
- [ ] Week 42: Anthropic + Ollama providers
- [ ] Week 45: Context extraction + chunking
- [ ] Week 48: Tool system (4 core tools)
- [ ] Week 50: Streaming chat interface

**Risk**: Provider SDK differences, tool calling variations

---

### Phase 6: API Compatibility (Weeks 51-60)

**Goal**: 405 API endpoints implemented

**Team Structure**:
- All 4 developers porting endpoints in parallel
- Each dev: ~100 endpoints

**Deliverables**:
```rust
// API router
pub fn api_routes() -> Router {
    Router::new()
        // Notes
        .route("/api/notes/:id", get(get_note))
        .route("/api/notes/:id", put(update_note))
        .route("/api/notes/:id", delete(delete_note))
        .route("/api/notes", post(create_note))

        // Branches
        .route("/api/branches/:id", get(get_branch))
        .route("/api/branches/:id", put(update_branch))
        .route("/api/branches/:id", delete(delete_branch))

        // Search
        .route("/api/search", post(search_notes))
        .route("/api/search/note/:id", get(search_from_note))

        // Sync
        .route("/api/sync/changed", get(get_sync_changes))
        .route("/api/sync/update", put(update_sync))
        .route("/api/sync/finished", post(sync_finished))

        // ... 395 more endpoints
}
```

**Milestones**:
- [ ] Week 52: 100 endpoints
- [ ] Week 54: 200 endpoints
- [ ] Week 56: 300 endpoints
- [ ] Week 58: 405 endpoints complete
- [ ] Week 60: API compatibility tests passing

**Risk**: API endpoint count, subtle behavioral differences

---

### Phase 7: Missing Features (Weeks 61-70)

**Goal**: Port remaining services

**Components**:
- Image processing (jimp → image crate)
- Import/export (markdown, HTML, OPML)
- Backup/restore (archiver → tar/zip crates)
- Encryption (crypto → argon2, ring)
- ETAPI (external API)

**Deliverables**:
```rust
// Image processing
pub async fn process_image(data: Vec<u8>) -> Result<ProcessedImage> {
    let img = image::load_from_memory(&data)?;
    let resized = img.resize(800, 800, FilterType::Lanczos3);
    let compressed = resized.to_jpeg(85)?;

    Ok(ProcessedImage {
        data: compressed,
        width: resized.width(),
        height: resized.height(),
    })
}

// Import/export
pub async fn export_markdown(note_id: &str) -> Result<String> {
    let note = get_note(note_id).await?;
    let content = note.get_content().await?;

    markdown::convert_html_to_markdown(&content)
}

// Backup
pub async fn create_backup() -> Result<PathBuf> {
    let backup_path = get_backup_dir().join(format!("backup-{}.db", Utc::now()));

    // Copy database with VACUUM
    sqlx::query("VACUUM INTO ?")
        .bind(&backup_path)
        .execute(&db)
        .await?;

    Ok(backup_path)
}
```

**Milestones**:
- [ ] Week 62: Image processing
- [ ] Week 64: Import/export formats
- [ ] Week 66: Backup/restore
- [ ] Week 68: Encryption
- [ ] Week 70: ETAPI complete

---

### Phase 8: Testing & QA (Weeks 71-80)

**Goal**: Full test coverage + performance testing

**Team Structure**:
- 2 QA engineers join
- 4 developers fix bugs

**Test Coverage**:
```rust
// Unit tests
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_create_note() {
        let db = test_db().await;
        let note = Note::new("Test", NoteType::Text);
        note.save(&db).await.unwrap();

        let loaded = Note::get(&db, &note.note_id).await.unwrap();
        assert_eq!(loaded.title, "Test");
    }

    #[tokio::test]
    async fn test_sync_conflict() {
        let db = test_db().await;
        let local = create_test_note("local").await;
        let remote = create_test_note("remote").await;

        let result = resolve_conflict(&local, &remote).await;
        assert!(result.utc_date_modified >= local.utc_date_modified);
    }
}

// Integration tests
#[tokio::test]
async fn test_full_sync_cycle() {
    let client_a = test_client("A").await;
    let client_b = test_client("B").await;

    // Client A creates note
    client_a.create_note("Test").await.unwrap();
    client_a.sync().await.unwrap();

    // Client B syncs
    client_b.sync().await.unwrap();

    // Verify note exists on B
    let notes = client_b.get_notes().await.unwrap();
    assert!(notes.iter().any(|n| n.title == "Test"));
}

// Performance tests
#[tokio::test]
async fn bench_search_performance() {
    let db = test_db_with_10k_notes().await;

    let start = Instant::now();
    let results = search(&db, "#todo").await.unwrap();
    let duration = start.elapsed();

    assert!(duration < Duration::from_millis(50));
    assert!(results.len() > 0);
}
```

**Milestones**:
- [ ] Week 72: Unit test coverage >80%
- [ ] Week 74: Integration test suite complete
- [ ] Week 76: Performance benchmarks passing
- [ ] Week 78: Memory leak testing
- [ ] Week 80: Load testing (10k+ notes)

---

### Phase 9: Migration & Deployment (Weeks 81-88)

**Goal**: Smooth migration path from Node.js

**Components**:
- Database migration scripts (if schema changes)
- Configuration migration
- Data validation
- Rollback procedures

**Migration Strategy**:
```rust
// Migration checker
pub async fn check_migration_compatibility() -> Result<MigrationReport> {
    let db = open_database().await?;

    // Check schema version
    let version = db.query_scalar("SELECT value FROM options WHERE name = 'dbVersion'").await?;

    if version < REQUIRED_VERSION {
        return Err(Error::MigrationRequired {
            current: version,
            required: REQUIRED_VERSION,
        });
    }

    // Validate data integrity
    let integrity = db.query_scalar("PRAGMA integrity_check").await?;
    if integrity != "ok" {
        return Err(Error::DatabaseCorrupted);
    }

    Ok(MigrationReport {
        compatible: true,
        notes_count: count_notes(&db).await?,
        database_size: get_db_size().await?,
    })
}

// Gradual rollout
pub enum DeploymentMode {
    Canary,      // 5% of users
    Beta,        // 25% of users
    Stable,      // 100% of users
}
```

**Milestones**:
- [ ] Week 82: Migration scripts tested
- [ ] Week 84: Canary deployment (5% users)
- [ ] Week 86: Beta deployment (25% users)
- [ ] Week 88: Stable release (100% users)

---

## 🚧 Critical Blockers & Solutions

### Blocker 1: User Backend Scripts ❌

**Problem**: Current system executes arbitrary Node.js code (728 lines in `backend_script_api.ts`)

**Current**:
```javascript
// Users write backend scripts like this:
const note = api.currentNote;
note.setLabel('processed', 'true');

const childNotes = await note.getChildNotes();
for (const child of childNotes) {
    child.setLabel('parent', note.noteId);
}
```

**Options**:

**Option A**: Remove feature entirely
- ❌ Users revolt
- ❌ Breaks 100s of user workflows
- ✅ Cleanest architecture

**Option B**: Replace with rhai scripting
```rust
// Embedded Rust scripting language
let script = r#"
    let note = get_current_note();
    note.set_label("processed", "true");

    let children = note.get_child_notes();
    for child in children {
        child.set_label("parent", note.note_id);
    }
"#;

let engine = rhai::Engine::new();
engine.register_fn("get_current_note", get_current_note);
engine.register_fn("set_label", set_label);

engine.eval::<()>(script)?;
```

**Pros**:
- ✅ Sandboxed (can't execute arbitrary code)
- ✅ Fast (compiled to bytecode)
- ✅ Rust-native (no Node.js needed)

**Cons**:
- ⚠️ Not JavaScript (users must rewrite scripts)
- ⚠️ Smaller ecosystem
- ⚠️ Learning curve

**Option C**: Keep Node.js subprocess
```rust
// Spawn Node.js for scripts only
pub async fn execute_backend_script(script: &str) -> Result<()> {
    let mut child = Command::new("node")
        .arg("-e")
        .arg(script)
        .spawn()?;

    child.wait().await?;
    Ok(())
}
```

**Pros**:
- ✅ Full JavaScript compatibility
- ✅ No user migration

**Cons**:
- ❌ Defeats purpose of Rust rewrite
- ❌ Still requires Node.js bundled
- ❌ Security risk (arbitrary code)

**Recommendation**: **Option B (rhai)** with migration tools

---

### Blocker 2: Animated GIF Detection

**Problem**: `is-animated` npm package has no Rust equivalent

**Current**:
```javascript
const isAnimated = require('is-animated');
if (isAnimated(buffer)) {
    // Special handling for animated GIFs
}
```

**Solution**: Implement manually
```rust
pub fn is_animated_gif(data: &[u8]) -> bool {
    if data.len() < 13 || &data[0..6] != b"GIF89a" && &data[0..6] != b"GIF87a" {
        return false;
    }

    // Count frames by looking for GCE (Graphic Control Extension)
    let mut frame_count = 0;
    let mut i = 13;  // Skip header + logical screen descriptor

    while i + 2 < data.len() {
        if data[i] == 0x21 && data[i + 1] == 0xF9 {
            frame_count += 1;
            if frame_count > 1 {
                return true;  // Multiple frames = animated
            }
        }
        i += 1;
    }

    false
}
```

**Effort**: 1-2 days

---

### Blocker 3: LLM Provider SDK Differences

**Problem**: OpenAI, Anthropic, Ollama have different APIs

**Solution**: Provider abstraction layer
```rust
#[async_trait]
pub trait LlmProvider {
    async fn chat(&self, messages: &[Message]) -> Result<Response>;
    async fn stream(&self, messages: &[Message]) -> Result<Stream<Response>>;
    fn supports_tools(&self) -> bool;
    fn model_name(&self) -> &str;
}

// Each provider implements trait differently
impl LlmProvider for OpenAiProvider {
    async fn chat(&self, messages: &[Message]) -> Result<Response> {
        let request = CreateChatCompletionRequest {
            model: self.model.clone(),
            messages: messages.iter().map(|m| m.to_openai()).collect(),
            ..Default::default()
        };

        let response = self.client.chat().create(request).await?;
        Ok(Response::from_openai(response))
    }
}
```

**Effort**: 2-3 weeks for all providers

---

## 🏗️ Technology Stack

### Recommended Rust Crates

**Core**:
```toml
[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }

# Web framework
axum = "0.7"  # or actix-web = "4"

# Database
sqlx = { version = "0.7", features = ["runtime-tokio", "sqlite"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Date/time
chrono = "0.4"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Crypto
argon2 = "0.5"
ring = "0.17"
totp-rs = "5"

# HTML/XML
scraper = "0.18"
ammonia = "3"

# Images
image = "0.24"
infer = "0.15"

# Compression
flate2 = "1"
tar = "0.4"

# HTTP client
reqwest = { version = "0.11", features = ["json"] }

# Scripting
rhai = "1"

# Error handling
anyhow = "1"
thiserror = "1"
```

**LLM Integration**:
```toml
async-openai = "0.16"  # OpenAI SDK
anthropic = "0.1"      # Anthropic SDK (if available)
# Ollama: Use reqwest directly (no official SDK)
```

---

## 📊 Team Structure & Roles

### Recommended 4-Person Team

**Person 1: Tech Lead / Architect** (40 hrs/week)
- Overall architecture decisions
- Code review all PRs
- Database layer (Weeks 1-8)
- Search engine architecture (Weeks 17-28)
- Sync protocol (Weeks 29-36)

**Person 2: Senior Backend Engineer** (40 hrs/week)
- Notes service (Weeks 9-16)
- Search engine implementation (Weeks 17-28)
- LLM providers (Weeks 37-50)
- API endpoints (Weeks 51-60)

**Person 3: Mid-Level Backend Engineer** (40 hrs/week)
- Branches + Attributes (Weeks 9-16)
- Search expression evaluators (Weeks 17-28)
- Sync conflict resolution (Weeks 29-36)
- API endpoints (Weeks 51-60)

**Person 4: Junior Backend Engineer** (40 hrs/week)
- Utilities (Weeks 1-8)
- API server setup (Weeks 1-8)
- LLM tools system (Weeks 37-50)
- API endpoints (Weeks 51-60)
- Import/export (Weeks 61-70)

**Weeks 71-80**: Add 2 QA engineers for testing

---

## 📅 Gantt Chart

```
Month 1-2:   Foundation [████████]
Month 3-4:   Core Entities [████████]
Month 5-7:   Search Engine [████████████]
Month 7-9:   Synchronization [████████]
Month 9-12:  LLM Integration [██████████████]
Month 11-15: API Compatibility [██████████████████]
Month 15-17: Missing Features [████████]
Month 18-20: Testing & QA [██████████]
Month 20-22: Migration [████]
```

**Critical Path**: Search Engine → Sync → LLM → API → Testing

---

## 💰 Cost Estimate

### Developer Costs

| Role | Rate | Hours | Cost |
|------|------|-------|------|
| Tech Lead | $150/hr | 1,600 | $240,000 |
| Senior Engineer | $125/hr | 1,600 | $200,000 |
| Mid Engineer | $100/hr | 1,600 | $160,000 |
| Junior Engineer | $75/hr | 1,600 | $120,000 |
| QA Engineer × 2 | $80/hr | 640 | $102,400 |
| **TOTAL** | | **7,680 hrs** | **$822,400** |

### With 20% Contingency: **~$987,000**

---

## ⚠️ Risk Assessment

### High Risks

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| **User Script Migration** | 90% | HIGH | Provide rhai migration tools, extensive docs |
| **Search Engine Complexity** | 70% | HIGH | Prototype early, allocate 12 weeks |
| **Sync State Machine Bugs** | 60% | HIGH | Extensive testing, gradual rollout |
| **API Behavioral Differences** | 80% | MEDIUM | Comprehensive integration tests |
| **Timeline Overrun** | 70% | MEDIUM | 20% buffer, weekly reviews |

### Medium Risks

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| **LLM Provider API Changes** | 50% | MEDIUM | Provider abstraction layer |
| **Performance Regressions** | 40% | MEDIUM | Continuous benchmarking |
| **Database Migration Issues** | 30% | HIGH | Extensive testing, rollback plan |

---

## 🎯 Success Criteria

### Phase Gates

Each phase must pass:
1. ✅ All unit tests passing (>80% coverage)
2. ✅ Integration tests passing
3. ✅ Performance benchmarks met
4. ✅ Code review approved
5. ✅ Documentation complete

### Final Success Criteria

1. ✅ **Functional Parity**: 95%+ of features working
2. ✅ **Performance**: 2x faster I/O operations
3. ✅ **Memory**: 50%+ reduction vs Node.js
4. ✅ **Binary Size**: <80 MB with sidecar
5. ✅ **Migration**: 100% data compatibility
6. ✅ **Mobile**: Works on iOS/Android via Tauri Mobile
7. ✅ **Stability**: <0.1% crash rate

---

## 🔄 Alternative: Hybrid Approach

**Instead of full rewrite**, consider gradual migration:

### Hybrid Architecture (6-12 months, $300k)

```
┌─────────────────────────────────────────┐
│   Tauri Desktop App                     │
├─────────────────────────────────────────┤
│                                         │
│   ┌─────────────────────────────────┐   │
│   │  Rust Services (New Features)   │   │
│   │  - Image processing             │   │
│   │  - Search indexing              │   │
│   │  - Background sync              │   │
│   └─────────────────────────────────┘   │
│              ↕ gRPC/REST                │
│   ┌─────────────────────────────────┐   │
│   │  Node.js Services (Legacy)      │   │
│   │  - Notes CRUD                   │   │
│   │  - Backend scripts ✅           │   │
│   │  - Complex sync logic           │   │
│   └─────────────────────────────────┘   │
└─────────────────────────────────────────┘
```

**Benefits**:
- ✅ Keep backend scripts working
- ✅ Gradual migration (lower risk)
- ✅ Immediate performance gains
- ✅ 50-70% cost reduction

**Drawbacks**:
- ⚠️ More complex architecture
- ⚠️ Two languages to maintain
- ⚠️ Still bundles Node.js (~15 MB)

---

## 📌 Recommendation

### Option A: Full Rewrite (THIS PLAN)
- **Timeline**: 10 months (4-person team)
- **Cost**: ~$1M
- **Risk**: HIGH
- **Benefit**: True mobile support, 50 MB binary
- **Tradeoff**: Lose backend scripts

### Option B: Hybrid Approach
- **Timeline**: 6 months (3-person team)
- **Cost**: ~$300k
- **Risk**: MEDIUM
- **Benefit**: Keep all features, gradual migration
- **Tradeoff**: Still bundles Node.js

### Option C: Node.js Sidecar (DONE)
- **Timeline**: 2 months (1-person)
- **Cost**: ~$40k
- **Risk**: LOW
- **Benefit**: Fast, all features work
- **Tradeoff**: No mobile support

---

## 🚀 Next Steps

If proceeding with full Rust rewrite:

1. **Week -4**: Hire team (Tech Lead + 3 engineers)
2. **Week -2**: Setup (repo, CI/CD, dev environments)
3. **Week -1**: Architecture spike (database layer prototype)
4. **Week 0**: Kickoff (all hands)
5. **Week 1**: Begin Phase 1 (Foundation)

**Decision Point**: Commit $1M + 10 months?

---

**Document Version**: 1.0
**Last Updated**: 2025-11-11
**Status**: PENDING APPROVAL
