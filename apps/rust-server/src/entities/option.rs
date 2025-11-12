use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use anyhow::Result;

use crate::db::Database;

/// Represents an option/setting (named TriliumOption to avoid conflicts with Rust's Option)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TriliumOption {
    pub name: String,
    pub value: String,

    #[sqlx(rename = "isSynced")]
    #[serde(rename = "isSynced")]
    pub is_synced: i32,

    #[sqlx(rename = "utcDateModified")]
    #[serde(rename = "utcDateModified")]
    pub utc_date_modified: String,
}

impl TriliumOption {
    /// Get option by name
    pub async fn get(db: &Database, name: &str) -> Result<Option<Self>> {
        let result = sqlx::query_as::<_, TriliumOption>(
            "SELECT * FROM options WHERE name = ?"
        )
        .bind(name)
        .fetch_optional(db.pool())
        .await?;

        Ok(result)
    }

    /// Get all options
    pub async fn get_all(db: &Database) -> Result<Vec<Self>> {
        let options = sqlx::query_as::<_, TriliumOption>(
            "SELECT * FROM options ORDER BY name"
        )
        .fetch_all(db.pool())
        .await?;

        Ok(options)
    }

    /// Set option value
    pub async fn set(db: &Database, name: &str, value: &str, is_synced: bool) -> Result<()> {
        let utc_now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
        let is_synced_int = if is_synced { 1 } else { 0 };

        sqlx::query(
            "INSERT INTO options (name, value, isSynced, utcDateModified)
             VALUES (?, ?, ?, ?)
             ON CONFLICT(name) DO UPDATE SET
             value = excluded.value,
             isSynced = excluded.isSynced,
             utcDateModified = excluded.utcDateModified"
        )
        .bind(name)
        .bind(value)
        .bind(is_synced_int)
        .bind(&utc_now)
        .execute(db.pool())
        .await?;

        Ok(())
    }

    /// Delete option
    pub async fn delete(db: &Database, name: &str) -> Result<()> {
        sqlx::query("DELETE FROM options WHERE name = ?")
            .bind(name)
            .execute(db.pool())
            .await?;

        Ok(())
    }
}
