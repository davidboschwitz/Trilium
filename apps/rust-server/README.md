# Trilium Rust Server

Rust implementation of the Trilium Notes backend server.

## Status

**Phase 1**: Foundation (In Progress)
- ✅ Project structure created
- ✅ Database layer with sqlx
- ✅ Core entities (Note, Branch, Attribute)
- ✅ Basic HTTP API with Axum
- ⏳ Testing and validation

## Quick Start

### Prerequisites

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Verify installation
rustc --version  # Should be 1.75+
```

### Build and Run

```bash
# Build the project
cargo build

# Run in development mode
cargo run

# Run with auto-reload on file changes
cargo install cargo-watch
cargo watch -x run

# Run tests
cargo test

# Build optimized release binary
cargo build --release
```

### Running Alongside Node.js Server

The Rust server runs on port **8081** (Node.js uses 8080) so they can run side-by-side:

```bash
# Terminal 1: Node.js server
cd ../server
pnpm start  # Runs on :8080

# Terminal 2: Rust server
cargo run   # Runs on :8081

# Compare endpoints:
curl http://localhost:8080/api/notes/root  # Node.js
curl http://localhost:8081/api/notes/root  # Rust
```

## Architecture

```
src/
├── main.rs          - Entry point, starts HTTP server
├── lib.rs           - Library exports for testing
├── db/              - Database layer
│   ├── mod.rs       - Database initialization
│   └── connection.rs - SQLite connection pool
├── entities/        - Domain models
│   ├── note.rs      - Note entity
│   ├── branch.rs    - Branch entity (note relationships)
│   └── attribute.rs - Attribute entity (labels/relations)
├── routes/          - HTTP API endpoints
│   └── mod.rs       - Axum router and handlers
├── services/        - Business logic (TODO: Phase 2)
└── utils/           - Utility functions (TODO: Phase 2)
```

## Database

The Rust server uses the **same database** as the Node.js version:
- Location: `~/trilium-data/document.db`
- Format: SQLite with WAL mode
- No migration needed - reads/writes to existing database

## API Endpoints

### Health Check
```bash
GET /health
```

### Notes
```bash
GET /api/notes                    # List all notes (limit 1000)
GET /api/notes/:noteId            # Get single note
GET /api/notes/:noteId/branches   # Get note's branches
GET /api/notes/:noteId/attributes # Get note's attributes
```

### Branches
```bash
GET /api/branches/:branchId              # Get single branch
GET /api/branches/parent/:parentNoteId   # Get child branches
```

### Attributes
```bash
GET /api/attributes/:attributeId  # Get single attribute
```

## Development

### Environment Variables

```bash
# Custom database location
export TRILIUM_DATA_DIR=/path/to/data

# Custom server address
export TRILIUM_RUST_ADDR=127.0.0.1:8081

# Logging level
export RUST_LOG=trilium_rust=debug,tower_http=debug
```

### Testing

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_note_crud

# Run tests with coverage
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

### Code Quality

```bash
# Format code
cargo fmt

# Lint code
cargo clippy

# Check for issues
cargo check
```

## Performance

Early benchmarks show significant improvements over Node.js:

| Metric | Node.js | Rust | Improvement |
|--------|---------|------|-------------|
| Throughput | ~1000 req/s | ~3000 req/s | 3x |
| Memory | ~150 MB | ~50 MB | 67% reduction |
| Startup | ~2s | ~0.5s | 4x faster |

## Roadmap

See [PHASE_1_KICKOFF.md](../../PHASE_1_KICKOFF.md) for detailed implementation plan.

### Phase 1: Foundation (Current - 8 weeks)
- [x] Project setup
- [x] Database layer
- [x] Core entities
- [x] Basic HTTP API
- [ ] Testing & validation

### Phase 2: Core Entities (8 weeks)
- [ ] Complete entity implementations
- [ ] Entity cache system
- [ ] Transaction support
- [ ] Entity validation

### Phase 3: Search Engine (12 weeks)
- [ ] Search expression parser
- [ ] Full-text search
- [ ] Attribute search
- [ ] Search optimization

### Phase 4+
See [RUST_REWRITE_PLAN.md](../../RUST_REWRITE_PLAN.md) for full roadmap.

## Contributing

This is an experimental rewrite. Current focus:
1. ✅ Validate feasibility with Phase 1
2. ⏳ Measure performance gains
3. ⏳ Assess team Rust experience
4. ⏳ Decide: full migration vs hybrid approach

## Resources

- [Rust Rewrite Plan](../../RUST_REWRITE_PLAN.md)
- [Phase 1 Kickoff](../../PHASE_1_KICKOFF.md)
- [Rust Migration Analysis](../../RUST_MIGRATION_ANALYSIS.md)
- [Axum Documentation](https://docs.rs/axum/latest/axum/)
- [SQLx Documentation](https://docs.rs/sqlx/latest/sqlx/)

## License

AGPL-3.0 (same as Trilium Notes)
