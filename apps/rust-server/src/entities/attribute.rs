use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use anyhow::Result;

use crate::db::Database;

/// Represents an attribute (label or relation) attached to a note
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Attribute {
    #[sqlx(rename = "attributeId")]
    #[serde(rename = "attributeId")]
    pub attribute_id: String,

    #[sqlx(rename = "noteId")]
    #[serde(rename = "noteId")]
    pub note_id: String,

    #[sqlx(rename = "type")]
    #[serde(rename = "type")]
    pub attr_type: String,  // "label" or "relation"

    pub name: String,

    pub value: Option<String>,

    #[sqlx(rename = "position")]
    pub position: i32,

    #[sqlx(rename = "isInheritable")]
    #[serde(rename = "isInheritable")]
    pub is_inheritable: i32,  // SQLite boolean

    #[sqlx(rename = "isDeleted")]
    #[serde(rename = "isDeleted")]
    pub is_deleted: i32,

    #[sqlx(rename = "utcDateModified")]
    #[serde(rename = "utcDateModified")]
    pub utc_date_modified: String,
}

impl Attribute {
    /// Find attribute by ID
    pub async fn find_by_id(db: &Database, attribute_id: &str) -> Result<Option<Self>> {
        let result = sqlx::query_as::<_, Attribute>(
            "SELECT * FROM attributes WHERE attributeId = ?"
        )
        .bind(attribute_id)
        .fetch_optional(db.pool())
        .await?;

        Ok(result)
    }

    /// Find all attributes for a note
    pub async fn find_by_note_id(db: &Database, note_id: &str) -> Result<Vec<Self>> {
        let attributes = sqlx::query_as::<_, Attribute>(
            "SELECT * FROM attributes WHERE noteId = ? AND isDeleted = 0 ORDER BY position"
        )
        .bind(note_id)
        .fetch_all(db.pool())
        .await?;

        Ok(attributes)
    }

    /// Find attributes by name
    pub async fn find_by_name(db: &Database, note_id: &str, name: &str) -> Result<Vec<Self>> {
        let attributes = sqlx::query_as::<_, Attribute>(
            "SELECT * FROM attributes WHERE noteId = ? AND name = ? AND isDeleted = 0"
        )
        .bind(note_id)
        .bind(name)
        .fetch_all(db.pool())
        .await?;

        Ok(attributes)
    }

    /// Find label attributes
    pub async fn find_labels(db: &Database, note_id: &str) -> Result<Vec<Self>> {
        let attributes = sqlx::query_as::<_, Attribute>(
            "SELECT * FROM attributes WHERE noteId = ? AND type = 'label' AND isDeleted = 0"
        )
        .bind(note_id)
        .fetch_all(db.pool())
        .await?;

        Ok(attributes)
    }

    /// Find relation attributes
    pub async fn find_relations(db: &Database, note_id: &str) -> Result<Vec<Self>> {
        let attributes = sqlx::query_as::<_, Attribute>(
            "SELECT * FROM attributes WHERE noteId = ? AND type = 'relation' AND isDeleted = 0"
        )
        .bind(note_id)
        .fetch_all(db.pool())
        .await?;

        Ok(attributes)
    }

    /// Insert a new attribute
    pub async fn insert(&self, db: &Database) -> Result<()> {
        sqlx::query(
            "INSERT INTO attributes (attributeId, noteId, type, name, value, position,
             isInheritable, isDeleted, utcDateModified)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&self.attribute_id)
        .bind(&self.note_id)
        .bind(&self.attr_type)
        .bind(&self.name)
        .bind(&self.value)
        .bind(self.position)
        .bind(self.is_inheritable)
        .bind(self.is_deleted)
        .bind(&self.utc_date_modified)
        .execute(db.pool())
        .await?;

        Ok(())
    }

    /// Check if attribute is a label
    pub fn is_label(&self) -> bool {
        self.attr_type == "label"
    }

    /// Check if attribute is a relation
    pub fn is_relation(&self) -> bool {
        self.attr_type == "relation"
    }

    /// Check if attribute is inheritable
    pub fn is_inheritable(&self) -> bool {
        self.is_inheritable != 0
    }

    /// Check if attribute is deleted
    pub fn is_deleted(&self) -> bool {
        self.is_deleted != 0
    }
}
