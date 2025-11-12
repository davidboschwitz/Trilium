use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use anyhow::Result;

use crate::db::Database;

/// Represents a note in Trilium
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Note {
    #[sqlx(rename = "noteId")]
    #[serde(rename = "noteId")]
    pub note_id: String,

    pub title: String,

    #[sqlx(rename = "type")]
    #[serde(rename = "type")]
    pub note_type: String,

    pub mime: String,

    #[sqlx(rename = "isProtected")]
    #[serde(rename = "isProtected")]
    pub is_protected: i32,  // SQLite stores boolean as integer

    #[sqlx(rename = "blobId")]
    #[serde(rename = "blobId")]
    pub blob_id: Option<String>,

    #[sqlx(rename = "dateCreated")]
    #[serde(rename = "dateCreated")]
    pub date_created: String,

    #[sqlx(rename = "dateModified")]
    #[serde(rename = "dateModified")]
    pub date_modified: String,

    #[sqlx(rename = "utcDateCreated")]
    #[serde(rename = "utcDateCreated")]
    pub utc_date_created: String,

    #[sqlx(rename = "utcDateModified")]
    #[serde(rename = "utcDateModified")]
    pub utc_date_modified: String,
}

impl Note {
    /// Find note by ID
    pub async fn find_by_id(db: &Database, note_id: &str) -> Result<Option<Self>> {
        let result = sqlx::query_as::<_, Note>(
            "SELECT * FROM notes WHERE noteId = ?"
        )
        .bind(note_id)
        .fetch_optional(db.pool())
        .await?;

        Ok(result)
    }

    /// Find all notes
    pub async fn find_all(db: &Database) -> Result<Vec<Self>> {
        let notes = sqlx::query_as::<_, Note>(
            "SELECT * FROM notes ORDER BY title LIMIT 1000"
        )
        .fetch_all(db.pool())
        .await?;

        Ok(notes)
    }

    /// Find notes by type
    pub async fn find_by_type(db: &Database, note_type: &str) -> Result<Vec<Self>> {
        let notes = sqlx::query_as::<_, Note>(
            "SELECT * FROM notes WHERE type = ? ORDER BY title LIMIT 1000"
        )
        .bind(note_type)
        .fetch_all(db.pool())
        .await?;

        Ok(notes)
    }

    /// Insert a new note
    pub async fn insert(&self, db: &Database) -> Result<()> {
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

    /// Update note
    pub async fn update(&self, db: &Database) -> Result<()> {
        sqlx::query(
            "UPDATE notes SET title = ?, type = ?, mime = ?, isProtected = ?,
             blobId = ?, dateModified = ?, utcDateModified = ?
             WHERE noteId = ?"
        )
        .bind(&self.title)
        .bind(&self.note_type)
        .bind(&self.mime)
        .bind(self.is_protected)
        .bind(&self.blob_id)
        .bind(&self.date_modified)
        .bind(&self.utc_date_modified)
        .bind(&self.note_id)
        .execute(db.pool())
        .await?;

        Ok(())
    }

    /// Delete note
    pub async fn delete(db: &Database, note_id: &str) -> Result<()> {
        sqlx::query("DELETE FROM notes WHERE noteId = ?")
            .bind(note_id)
            .execute(db.pool())
            .await?;

        Ok(())
    }

    /// Check if note is protected (encrypted)
    pub fn is_protected(&self) -> bool {
        self.is_protected != 0
    }
}
