// Prevents additional console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::{Command, Child, Stdio};
use std::sync::Mutex;
use std::path::PathBuf;
use tauri::{Manager, AppHandle};

struct AppState {
    server_process: Mutex<Option<Child>>,
}

fn get_server_binary_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
    // Get the sidecar path
    let resource_dir = app_handle.path().resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {}", e))?;

    #[cfg(target_os = "windows")]
    let binary_name = "trilium-rust.exe";
    #[cfg(not(target_os = "windows"))]
    let binary_name = "trilium-rust";

    let binary_path = resource_dir.join("binaries").join(binary_name);

    if !binary_path.exists() {
        return Err(format!(
            "Server binary not found at: {}. Expected path for bundled binary.",
            binary_path.display()
        ));
    }

    Ok(binary_path)
}

fn start_rust_server(app_handle: &AppHandle, state: tauri::State<AppState>) -> Result<String, String> {
    // Check if server is already running
    let mut server = state.server_process.lock()
        .map_err(|e| format!("Failed to lock server process: {}", e))?;

    if server.is_some() {
        return Ok("Server is already running".to_string());
    }

    // Get the binary path
    let binary_path = get_server_binary_path(app_handle)?;

    println!("Starting Trilium Rust server from: {}", binary_path.display());

    // Set environment variables
    let data_dir = app_handle.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data dir: {}", e))?;

    let trilium_data = data_dir.join("trilium-data");
    std::fs::create_dir_all(&trilium_data)
        .map_err(|e| format!("Failed to create data directory: {}", e))?;

    // Start the server process
    let child = Command::new(binary_path)
        .env("TRILIUM_DATA_DIR", trilium_data.to_str().unwrap())
        .env("TRILIUM_RUST_ADDR", "127.0.0.1:8081")
        .env("RUST_LOG", "trilium_rust=info,tower_http=info")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start server: {}", e))?;

    let pid = child.id();
    *server = Some(child);

    println!("Trilium Rust server started with PID: {}", pid);

    Ok(format!("Server started successfully on http://127.0.0.1:8081 (PID: {})", pid))
}

#[tauri::command]
fn start_server(app_handle: AppHandle, state: tauri::State<AppState>) -> Result<String, String> {
    start_rust_server(&app_handle, state)
}

#[tauri::command]
fn stop_server(state: tauri::State<AppState>) -> Result<(), String> {
    let mut server = state.server_process.lock()
        .map_err(|e| format!("Failed to lock server process: {}", e))?;

    if let Some(mut child) = server.take() {
        println!("Stopping Trilium Rust server (PID: {})", child.id());
        child.kill()
            .map_err(|e| format!("Failed to kill server process: {}", e))?;
        println!("Server stopped successfully");
    }

    Ok(())
}

#[tauri::command]
fn get_server_status(state: tauri::State<AppState>) -> Result<String, String> {
    let server = state.server_process.lock()
        .map_err(|e| format!("Failed to lock server process: {}", e))?;

    if let Some(child) = server.as_ref() {
        Ok(format!("Server is running (PID: {})", child.id()))
    } else {
        Ok("Server is not running".to_string())
    }
}

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            server_process: Mutex::new(None),
        })
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![start_server, stop_server, get_server_status])
        .setup(|app| {
            println!("=== Trilium Tauri Desktop Starting ===");
            println!("Resource dir: {:?}", app.path().resource_dir());
            println!("App data dir: {:?}", app.path().app_data_dir());

            // Auto-start the Rust server
            let app_handle = app.handle().clone();
            let state = app.state::<AppState>();

            match start_rust_server(&app_handle, state) {
                Ok(msg) => {
                    println!("✓ {}", msg);
                    // Give the server a moment to start up
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
                Err(e) => {
                    eprintln!("✗ Failed to start server: {}", e);
                    eprintln!("Note: In development, you may need to start the server manually:");
                    eprintln!("  cd apps/rust-server && cargo run --release");
                }
            }

            println!("=== Opening Trilium window ===");

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                // Stop the server when the window is closed
                let state = window.state::<AppState>();
                if let Err(e) = stop_server(state) {
                    eprintln!("Error stopping server: {}", e);
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
