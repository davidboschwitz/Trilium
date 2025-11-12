mod connection;

pub use connection::Database;

use anyhow::Result;
use std::path::PathBuf;
use tracing::info;

/// Initialize database connection using the same path as Node.js Trilium
pub async fn initialize() -> Result<Database> {
    let data_dir = get_data_directory()?;
    let db_path = data_dir.join("document.db");

    if !db_path.exists() {
        anyhow::bail!(
            "Database not found at {}. Please run the Node.js Trilium server first to create the database.",
            db_path.display()
        );
    }

    info!("Connecting to database at: {}", db_path.display());

    let db = Database::new(&db_path).await?;

    Ok(db)
}

/// Get Trilium data directory (same as Node.js version)
fn get_data_directory() -> Result<PathBuf> {
    // Check environment variable first
    if let Ok(data_dir) = std::env::var("TRILIUM_DATA_DIR") {
        return Ok(PathBuf::from(data_dir));
    }

    // Default to ~/trilium-data
    let home = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine home directory"))?;

    let data_dir = home.join("trilium-data");

    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir)?;
    }

    Ok(data_dir)
}
