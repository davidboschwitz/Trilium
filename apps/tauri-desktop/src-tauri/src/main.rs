// Prevents additional console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::process::{Command, Child};
use std::sync::Mutex;
use tauri::Manager;

struct AppState {
    server_process: Mutex<Option<Child>>,
}

#[tauri::command]
fn start_server(app_handle: tauri::AppHandle) -> Result<String, String> {
    // Get the resource path where the server will be bundled
    let resource_path = app_handle.path().resource_dir()
        .map_err(|e| format!("Failed to get resource dir: {}", e))?;

    // For now, we'll just return a success message
    // In production, this would start the Node.js server process
    Ok(format!("Server would start from: {:?}", resource_path))
}

#[tauri::command]
fn stop_server(state: tauri::State<AppState>) -> Result<(), String> {
    let mut server = state.server_process.lock()
        .map_err(|e| format!("Failed to lock server process: {}", e))?;

    if let Some(mut child) = server.take() {
        child.kill()
            .map_err(|e| format!("Failed to kill server process: {}", e))?;
    }

    Ok(())
}

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            server_process: Mutex::new(None),
        })
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![start_server, stop_server])
        .setup(|app| {
            // Print some debug info
            println!("Tauri app starting...");
            println!("Resource dir: {:?}", app.path().resource_dir());
            println!("App data dir: {:?}", app.path().app_data_dir());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
