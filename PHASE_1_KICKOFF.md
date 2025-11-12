# Phase 1 Kickoff: Rust Migration Foundation

**Status**: Ready to begin implementation
**Duration**: 8 weeks (with 4-person team) or 16 weeks (solo)
**Goal**: Establish Rust backend foundation with database layer and basic HTTP server

---

## Prerequisites

### Required Tools
```bash
# Install Rust (latest stable)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable

# Verify installation
rustc --version  # Should be 1.75+
cargo --version

# Install development tools
cargo install cargo-watch  # Auto-rebuild on file changes
cargo install cargo-edit    # Easier dependency management
```

### Recommended IDE Setup
- **VS Code** with extensions:
  - rust-analyzer (official Rust language server)
  - CodeLLDB (debugging)
  - Even Better TOML (Cargo.toml editing)
  - Error Lens (inline error display)

---

## Week 1-2: Project Setup

### Task 1.1: Initialize Rust Workspace

Create `apps/rust-server/Cargo.toml`:
```toml
[package]
name = "trilium-rust"
version = "0.1.0"
edition = "2021"

[dependencies]
# Async runtime
tokio = { version = "1.35", features = ["full"] }

# Web framework
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["fs", "cors", "trace"] }

# Database
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "sqlite"] }
rusqlite = { version = "0.30", features = ["bundled"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Utilities
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.6", features = ["v4", "serde"] }
thiserror = "1.0"
anyhow = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[dev-dependencies]
tokio-test = "0.4"
```

### Task 1.2: Create Project Structure

```bash
cd apps
mkdir -p rust-server/src/{db,entities,services,routes,utils}
cd rust-server

# Create initial files
touch src/main.rs
touch src/db/mod.rs
touch src/db/connection.rs
touch src/db/migrations.rs
touch src/entities/mod.rs
touch src/entities/note.rs
touch src/entities/branch.rs
touch src/entities/attribute.rs
```

### Task 1.3: Basic Main Entry Point

Create `apps/rust-server/src/main.rs`:
```rust
use anyhow::Result;
use tracing::info;
use tracing_subscriber;

mod db;
mod entities;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("trilium_rust=debug,tower_http=debug")
        .init();

    info!("Starting Trilium Rust server...");

    // Initialize database
    let db = db::initialize().await?;
    info!("Database initialized");

    // TODO: Start HTTP server (Week 3-4)

    Ok(())
}
```

---

## Week 3-4: Database Layer

### Task 2.1: Database Connection Pool

Create `apps/rust-server/src/db/connection.rs`:
```rust
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use anyhow::Result;
use std::path::Path;

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new(db_path: &Path) -> Result<Self> {
        let connection_string = format!("sqlite://{}", db_path.display());

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&connection_string)
            .await?;

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub async fn execute(&self, query: &str) -> Result<()> {
        sqlx::query(query)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn query_one<T>(&self, query: &str) -> Result<T>
    where
        T: for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> + Send + Unpin,
    {
        let row = sqlx::query_as::<_, T>(query)
            .fetch_one(&self.pool)
            .await?;
        Ok(row)
    }
}
```

### Task 2.2: Entity Definitions

Create `apps/rust-server/src/entities/note.rs`:
```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Note {
    #[sqlx(rename = "noteId")]
    pub note_id: String,

    pub title: String,

    #[sqlx(rename = "type")]
    pub note_type: String,

    pub mime: String,

    #[sqlx(rename = "isProtected")]
    pub is_protected: bool,

    #[sqlx(rename = "blobId")]
    pub blob_id: Option<String>,

    #[sqlx(rename = "dateCreated")]
    pub date_created: String,

    #[sqlx(rename = "dateModified")]
    pub date_modified: String,

    #[sqlx(rename = "utcDateCreated")]
    pub utc_date_created: String,

    #[sqlx(rename = "utcDateModified")]
    pub utc_date_modified: String,
}

impl Note {
    pub async fn find_by_id(db: &crate::db::Database, note_id: &str) -> anyhow::Result<Option<Self>> {
        let result = sqlx::query_as::<_, Note>(
            "SELECT * FROM notes WHERE noteId = ?"
        )
        .bind(note_id)
        .fetch_optional(db.pool())
        .await?;

        Ok(result)
    }

    pub async fn find_all(db: &crate::db::Database) -> anyhow::Result<Vec<Self>> {
        let notes = sqlx::query_as::<_, Note>(
            "SELECT * FROM notes ORDER BY title"
        )
        .fetch_all(db.pool())
        .await?;

        Ok(notes)
    }

    pub async fn insert(&self, db: &crate::db::Database) -> anyhow::Result<()> {
        sqlx::query(
            "INSERT INTO notes (noteId, title, type, mime, isProtected, blobId,
             dateCreated, dateModified, utcDateCreated, utcDateModified)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&self.note_id)
        .bind(&self.title)
        .bind(&self.note_type)
        .bind(&self.mime)
        .bind(self.is_protected)
        .bind(&self.blob_id)
        .bind(&self.date_created)
        .bind(&self.date_modified)
        .bind(&self.utc_date_created)
        .bind(&self.utc_date_modified)
        .execute(db.pool())
        .await?;

        Ok(())
    }
}
```

### Task 2.3: Database Initialization

Create `apps/rust-server/src/db/mod.rs`:
```rust
mod connection;
mod migrations;

pub use connection::Database;

use anyhow::Result;
use std::path::PathBuf;

pub async fn initialize() -> Result<Database> {
    // Use same database path as Node.js version
    let data_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?
        .join("trilium-data");

    std::fs::create_dir_all(&data_dir)?;

    let db_path = data_dir.join("document.db");
    let db = Database::new(&db_path).await?;

    tracing::info!("Connected to database at: {}", db_path.display());

    Ok(db)
}
```

---

## Week 5-6: Basic HTTP Server

### Task 3.1: Axum Server Setup

Create `apps/rust-server/src/routes/mod.rs`:
```rust
use axum::{
    Router,
    routing::{get, post},
    extract::State,
    Json,
};
use std::sync::Arc;

use crate::db::Database;
use crate::entities::Note;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
}

pub fn create_router(db: Arc<Database>) -> Router {
    let state = AppState { db };

    Router::new()
        .route("/health", get(health_check))
        .route("/api/notes", get(get_notes))
        .route("/api/notes/:noteId", get(get_note))
        .with_state(state)
}

async fn health_check() -> &'static str {
    "OK"
}

async fn get_notes(
    State(state): State<AppState>,
) -> Result<Json<Vec<Note>>, String> {
    let notes = Note::find_all(&state.db)
        .await
        .map_err(|e| e.to_string())?;

    Ok(Json(notes))
}

async fn get_note(
    State(state): State<AppState>,
    axum::extract::Path(note_id): axum::extract::Path<String>,
) -> Result<Json<Note>, String> {
    let note = Note::find_by_id(&state.db, &note_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Note not found".to_string())?;

    Ok(Json(note))
}
```

### Task 3.2: Update Main to Start Server

Update `apps/rust-server/src/main.rs`:
```rust
use anyhow::Result;
use tracing::info;
use std::sync::Arc;
use axum::Router;
use tower_http::trace::TraceLayer;

mod db;
mod entities;
mod routes;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("trilium_rust=debug,tower_http=debug")
        .init();

    info!("Starting Trilium Rust server...");

    // Initialize database
    let db = Arc::new(db::initialize().await?);
    info!("Database initialized");

    // Create router
    let app = routes::create_router(db)
        .layer(TraceLayer::new_for_http());

    // Start server
    let addr = "127.0.0.1:8081";
    let listener = tokio::net::TcpListener::bind(addr).await?;

    info!("Server listening on http://{}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
```

---

## Week 7-8: Testing & Validation

### Task 4.1: Integration Tests

Create `apps/rust-server/tests/integration_test.rs`:
```rust
use trilium_rust::db::Database;
use trilium_rust::entities::Note;
use std::path::PathBuf;

#[tokio::test]
async fn test_note_crud() {
    // Use test database
    let test_db = PathBuf::from("/tmp/test_trilium.db");
    let db = Database::new(&test_db).await.unwrap();

    // Create note
    let note = Note {
        note_id: "test123".to_string(),
        title: "Test Note".to_string(),
        note_type: "text".to_string(),
        mime: "text/html".to_string(),
        is_protected: false,
        blob_id: None,
        date_created: "2024-01-01".to_string(),
        date_modified: "2024-01-01".to_string(),
        utc_date_created: "2024-01-01".to_string(),
        utc_date_modified: "2024-01-01".to_string(),
    };

    note.insert(&db).await.unwrap();

    // Retrieve note
    let retrieved = Note::find_by_id(&db, "test123").await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().title, "Test Note");

    // Cleanup
    std::fs::remove_file(test_db).ok();
}
```

### Task 4.2: Benchmark Against Node.js

Create simple performance comparison:
```bash
# Run Node.js server
cd apps/server
pnpm start

# In another terminal, benchmark
ab -n 1000 -c 10 http://localhost:8080/api/notes/root

# Run Rust server
cd apps/rust-server
cargo run --release

# Benchmark
ab -n 1000 -c 10 http://localhost:8081/api/notes/root

# Expected: Rust should be 2-3x faster
```

---

## Validation Checklist

Phase 1 is complete when:

- [ ] Rust project compiles without errors
- [ ] Can connect to existing Trilium SQLite database
- [ ] Can read notes from database
- [ ] Basic HTTP server responds on port 8081
- [ ] `/health` endpoint returns 200 OK
- [ ] `/api/notes` endpoint returns JSON array of notes
- [ ] `/api/notes/:noteId` endpoint returns single note
- [ ] Integration tests pass
- [ ] No data corruption in existing database
- [ ] Performance is at least equal to Node.js version

---

## Running the Prototype

```bash
# Terminal 1: Keep existing Node.js server running (for comparison)
cd apps/server
pnpm start  # Runs on :8080

# Terminal 2: Run Rust server
cd apps/rust-server
cargo run  # Runs on :8081

# Test both:
curl http://localhost:8080/api/notes/root  # Node.js
curl http://localhost:8081/api/notes/root  # Rust

# Development mode (auto-reload)
cargo watch -x run
```

---

## Common Issues & Solutions

### Issue: sqlx compile-time verification fails
```bash
# Solution: Use offline mode during development
cargo sqlx prepare  # Run this when queries change
```

### Issue: Database locked errors
```bash
# Solution: Ensure Node.js server is not running or use different DB
# Or configure SQLite for shared access in connection.rs:
# .pragma("journal_mode", "WAL")
# .pragma("synchronous", "NORMAL")
```

### Issue: Slow compile times
```bash
# Solution: Use cargo's incremental compilation
export CARGO_INCREMENTAL=1

# Or use mold linker (Linux)
cargo install mold
```

---

## Next Steps After Phase 1

Once Phase 1 validation passes:

1. **Phase 2**: Implement remaining entities (Branch, Attribute, Revision)
2. **Parallel work**: Start API endpoint migration
3. **Documentation**: Create Rust API documentation with `cargo doc`
4. **CI/CD**: Add GitHub Actions for Rust build and test

---

## Decision Point: Full Migration vs Hybrid

After completing Phase 1, evaluate:

**Metrics to measure**:
- Performance improvement (should see 2-3x speedup)
- Memory usage (should see 30-50% reduction)
- Development velocity (how fast can team write Rust?)
- Code complexity (is Rust code maintainable?)

**Decision criteria**:
- If performance gains > 2x → Continue full migration
- If team velocity < 50% of TypeScript → Consider hybrid approach
- If bugs/issues multiply → Pause and reassess

**Hybrid option**: Keep Node.js for complex parts (LLM, search) while migrating hot paths (database, sync, API) to Rust.

---

## Resources

- [Rust Book](https://doc.rust-lang.org/book/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [Axum Examples](https://github.com/tokio-rs/axum/tree/main/examples)
- [SQLx Documentation](https://docs.rs/sqlx/latest/sqlx/)
- [Existing Trilium API docs](./apps/server/src/routes/api/)

---

**Status**: Ready to begin
**Estimated effort**: 160-320 hours (depending on team size)
**Risk level**: MEDIUM (database compatibility is proven, main risk is team Rust experience)
**Go/No-Go decision**: After Phase 1 completion (Week 8)
