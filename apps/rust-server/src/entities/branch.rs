use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use anyhow::Result;

use crate::db::Database;

/// Represents a branch (parent-child relationship) between notes
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Branch {
    #[sqlx(rename = "branchId")]
    #[serde(rename = "branchId")]
    pub branch_id: String,

    #[sqlx(rename = "noteId")]
    #[serde(rename = "noteId")]
    pub note_id: String,

    #[sqlx(rename = "parentNoteId")]
    #[serde(rename = "parentNoteId")]
    pub parent_note_id: String,

    #[sqlx(rename = "prefix")]
    pub prefix: Option<String>,

    #[sqlx(rename = "notePosition")]
    #[serde(rename = "notePosition")]
    pub note_position: i32,

    #[sqlx(rename = "isExpanded")]
    #[serde(rename = "isExpanded")]
    pub is_expanded: i32,  // SQLite boolean as integer

    #[sqlx(rename = "isDeleted")]
    #[serde(rename = "isDeleted")]
    pub is_deleted: i32,

    #[sqlx(rename = "utcDateModified")]
    #[serde(rename = "utcDateModified")]
    pub utc_date_modified: String,
}

impl Branch {
    /// Find branch by ID
    pub async fn find_by_id(db: &Database, branch_id: &str) -> Result<Option<Self>> {
        let result = sqlx::query_as::<_, Branch>(
            "SELECT * FROM branches WHERE branchId = ?"
        )
        .bind(branch_id)
        .fetch_optional(db.pool())
        .await?;

        Ok(result)
    }

    /// Find all branches for a note
    pub async fn find_by_note_id(db: &Database, note_id: &str) -> Result<Vec<Self>> {
        let branches = sqlx::query_as::<_, Branch>(
            "SELECT * FROM branches WHERE noteId = ? AND isDeleted = 0 ORDER BY notePosition"
        )
        .bind(note_id)
        .fetch_all(db.pool())
        .await?;

        Ok(branches)
    }

    /// Find child branches of a parent note
    pub async fn find_children(db: &Database, parent_note_id: &str) -> Result<Vec<Self>> {
        let branches = sqlx::query_as::<_, Branch>(
            "SELECT * FROM branches WHERE parentNoteId = ? AND isDeleted = 0 ORDER BY notePosition"
        )
        .bind(parent_note_id)
        .fetch_all(db.pool())
        .await?;

        Ok(branches)
    }

    /// Insert a new branch
    pub async fn insert(&self, db: &Database) -> Result<()> {
        sqlx::query(
            "INSERT INTO branches (branchId, noteId, parentNoteId, prefix, notePosition,
             isExpanded, isDeleted, utcDateModified)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&self.branch_id)
        .bind(&self.note_id)
        .bind(&self.parent_note_id)
        .bind(&self.prefix)
        .bind(self.note_position)
        .bind(self.is_expanded)
        .bind(self.is_deleted)
        .bind(&self.utc_date_modified)
        .execute(db.pool())
        .await?;

        Ok(())
    }

    /// Check if branch is deleted
    pub fn is_deleted(&self) -> bool {
        self.is_deleted != 0
    }

    /// Check if branch is expanded in UI
    pub fn is_expanded(&self) -> bool {
        self.is_expanded != 0
    }
}
