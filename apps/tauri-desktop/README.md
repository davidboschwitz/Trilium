# Trilium Tauri Desktop - Prototype

This is a proof-of-concept Tauri wrapper for Trilium Notes.

## Architecture

```
Tauri Shell (Rust + System Webview)
  ↓
  Points to http://localhost:8080
  ↓
Trilium Server (Node.js)
  ↓
SQLite Database
```

## How It Works

1. Tauri creates a native window with system webview
2. Points webview to `http://localhost:8080`
3. User manually starts Trilium server with `pnpm server:start`
4. Tauri app connects to the running server

## Development

```bash
# Terminal 1: Start Trilium server
cd ../..
pnpm server:start

# Terminal 2: Start Tauri dev mode
cd apps/tauri-desktop
pnpm install
pnpm dev
```

## Building

```bash
# Build for current platform
pnpm build
```

## Future Enhancements

- [ ] Auto-start Node.js server as sidecar process
- [ ] Bundle server with app
- [ ] Handle server startup/shutdown automatically
- [ ] System tray integration
- [ ] Native file dialogs
- [ ] Auto-updater

## Size Comparison

**Current Electron App**: ~50-80 MB
**Tauri App** (estimated): ~20-30 MB with Node.js sidecar

**Breakdown**:
- Tauri binary: ~3-5 MB
- Node.js runtime: ~15-20 MB
- Trilium server code: ~5 MB

**Savings**: ~50% reduction in app size
