# Build Instructions for Trilium Tauri Prototype

## Prerequisites

### Linux (Ubuntu/Debian)
```bash
sudo apt-get update
sudo apt-get install -y \
    libgtk-3-dev \
    libwebkit2gtk-4.1-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    patchelf
```

### macOS
```bash
# Xcode Command Line Tools
xcode-select --install
```

### Windows
```bash
# Install Microsoft Visual Studio C++ Build Tools
# Install WebView2 (included in Windows 11, optional download for Windows 10)
```

## Building

### Development Mode

```bash
# Terminal 1: Start Trilium server
cd ../..
pnpm server:start

# Terminal 2: Run Tauri in dev mode
cd apps/tauri-desktop
pnpm install
pnpm dev
```

The Tauri window will open and connect to `http://localhost:8080` where the Trilium server is running.

### Production Build

```bash
# Build for current platform
cd apps/tauri-desktop
pnpm build
```

**Outputs**:
- **Linux**: `src-tauri/target/release/bundle/deb/trilium-tauri_*.deb`
- **Linux**: `src-tauri/target/release/bundle/appimage/trilium-tauri_*.AppImage`
- **macOS**: `src-tauri/target/release/bundle/macos/Trilium Notes.app`
- **Windows**: `src-tauri/target/release/bundle/msi/Trilium Notes_*.msi`

## Troubleshooting

### Missing GTK Libraries (Linux)
```bash
# Error: "The system library `gdk-3.0` required by crate `gdk-sys` was not found"
# Solution: Install GTK development libraries
sudo apt-get install libgtk-3-dev libwebkit2gtk-4.1-dev
```

### WebView2 Missing (Windows)
```bash
# Error: "WebView2 runtime not found"
# Solution: Download and install WebView2 Runtime
# https://developer.microsoft.com/en-us/microsoft-edge/webview2/
```

### Rust Not Installed
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## App Size Comparison

### Before (Electron)
```
Electron App:           ~80 MB
  ├─ Chromium:          ~50 MB
  ├─ Node.js:           ~15 MB
  ├─ Trilium code:      ~10 MB
  └─ Resources:         ~5 MB
```

### After (Tauri Prototype - without sidecar)
```
Tauri App:              ~3-5 MB
  ├─ Tauri binary:      ~2-3 MB
  ├─ Trilium client:    ~1-2 MB
  └─ Resources:         ~1 MB

Savings: ~75 MB (94% reduction!)
```

### Future (Tauri + Node.js Sidecar)
```
Tauri App:              ~20-25 MB
  ├─ Tauri binary:      ~3 MB
  ├─ Node.js runtime:   ~15 MB
  ├─ Trilium server:    ~5 MB
  └─ Resources:         ~2 MB

Savings: ~55 MB (69% reduction!)
```

## Next Steps

### Phase 1: Basic Tauri Wrapper (✅ Complete - This Prototype)
- [x] Create Tauri project structure
- [x] Configure to point to localhost:8080
- [x] Generate app icons
- [x] Basic window configuration

### Phase 2: Auto-Start Server (Next)
- [ ] Bundle Node.js server as sidecar
- [ ] Auto-start server on app launch
- [ ] Handle server shutdown on app close
- [ ] Detect and handle port conflicts

### Phase 3: Native Integration (Future)
- [ ] System tray integration
- [ ] Native file dialogs for import/export
- [ ] Custom protocol handler (trilium://)
- [ ] Native notifications
- [ ] Auto-updater

### Phase 4: Polish (Future)
- [ ] Splash screen during server startup
- [ ] Better error handling
- [ ] Logging to app data directory
- [ ] Database backup/restore UI
- [ ] Multiple profile support
