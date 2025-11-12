use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use anyhow::Result;

use crate::db::Database;

/// Represents a blob (note content storage)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Blob {
    #[sqlx(rename = "blobId")]
    #[serde(rename = "blobId")]
    pub blob_id: String,

    /// Content stored as TEXT (may be NULL for large content)
    pub content: Option<String>,

    #[sqlx(rename = "dateModified")]
    #[serde(rename = "dateModified")]
    pub date_modified: String,

    #[sqlx(rename = "utcDateModified")]
    #[serde(rename = "utcDateModified")]
    pub utc_date_modified: String,
}

impl Blob {
    /// Find blob by ID
    pub async fn find_by_id(db: &Database, blob_id: &str) -> Result<Option<Self>> {
        let result = sqlx::query_as::<_, Blob>(
            "SELECT * FROM blobs WHERE blobId = ?"
        )
        .bind(blob_id)
        .fetch_optional(db.pool())
        .await?;

        Ok(result)
    }

    /// Get blob content as bytes
    pub async fn get_content(db: &Database, blob_id: &str) -> Result<Option<Vec<u8>>> {
        let result = sqlx::query_scalar::<_, Option<Vec<u8>>>(
            "SELECT content FROM blobs WHERE blobId = ?"
        )
        .bind(blob_id)
        .fetch_optional(db.pool())
        .await?;

        Ok(result.flatten())
    }

    /// Get blob content as string (for text content)
    pub async fn get_content_string(db: &Database, blob_id: &str) -> Result<Option<String>> {
        let result = sqlx::query_scalar::<_, Option<String>>(
            "SELECT content FROM blobs WHERE blobId = ?"
        )
        .bind(blob_id)
        .fetch_optional(db.pool())
        .await?;

        Ok(result.flatten())
    }

    /// Insert a new blob with content
    pub async fn insert(db: &Database, blob_id: &str, content: &[u8], date_modified: &str, utc_date_modified: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO blobs (blobId, content, dateModified, utcDateModified)
             VALUES (?, ?, ?, ?)"
        )
        .bind(blob_id)
        .bind(content)
        .bind(date_modified)
        .bind(utc_date_modified)
        .execute(db.pool())
        .await?;

        Ok(())
    }

    /// Update blob content
    pub async fn update_content(db: &Database, blob_id: &str, content: &[u8], date_modified: &str, utc_date_modified: &str) -> Result<()> {
        sqlx::query(
            "UPDATE blobs SET content = ?, dateModified = ?, utcDateModified = ?
             WHERE blobId = ?"
        )
        .bind(content)
        .bind(date_modified)
        .bind(utc_date_modified)
        .bind(blob_id)
        .execute(db.pool())
        .await?;

        Ok(())
    }

    /// Delete blob
    pub async fn delete(db: &Database, blob_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM blobs WHERE blobId = ?")
            .bind(blob_id)
            .execute(db.pool())
            .await?;

        Ok(())
    }

    /// Check if blob exists
    pub async fn exists(db: &Database, blob_id: &str) -> Result<bool> {
        let count: i32 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM blobs WHERE blobId = ?"
        )
        .bind(blob_id)
        .fetch_one(db.pool())
        .await?;

        Ok(count > 0)
    }
}
