use axum::{
    Router,
    routing::{get, post, put, delete as axum_delete},
    extract::{State, Path},
    body::Bytes,
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::FromRow;

use crate::db::Database;
use crate::entities::{Note, Branch, Attribute, Blob, TriliumOption};

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
}

/// Create the main application router
pub fn create_router(db: Arc<Database>) -> Router {
    let state = AppState { db };

    Router::new()
        // Health check
        .route("/health", get(health_check))

        // Notes API
        .route("/api/notes", get(get_notes))
        .route("/api/notes/:noteId", get(get_note).put(update_note).delete(delete_note))
        .route("/api/notes/:noteId/title", put(update_note_title))
        .route("/api/notes/:noteId/type", put(update_note_type))
        .route("/api/notes/:noteId/branches", get(get_note_branches))
        .route("/api/notes/:noteId/attributes", get(get_note_attributes).put(update_note_attributes))
        .route("/api/notes/:noteId/blob", get(get_note_blob).put(update_note_blob))
        .route("/api/notes/:noteId/data", get(get_note_blob).put(update_note_blob))
        .route("/api/notes/:parentNoteId/children", post(create_note))

        // Tree API
        .route("/api/tree", get(get_tree).post(load_tree))
        .route("/api/refresh-note-ordering/:parentNoteId", post(refresh_note_ordering))

        // Branches API
        .route("/api/branches/:branchId", get(get_branch).put(update_branch).delete(delete_branch))
        .route("/api/branches/parent/:parentNoteId", get(get_child_branches))

        // Attributes API
        .route("/api/attributes/:attributeId", get(get_attribute).put(update_attribute).delete(delete_attribute))
        .route("/api/attributes", post(create_attribute))

        // Search API
        .route("/api/search/:searchString", get(search_notes))
        .route("/api/search-notes", get(search_notes_query))

        // Recent Changes API
        .route("/api/recent-changes", get(get_recent_changes))

        // Options API
        .route("/api/options", get(get_options))
        .route("/api/options/:name", get(get_option).put(set_option))

        .with_state(state)
}

/// Health check endpoint
async fn health_check() -> &'static str {
    "OK"
}

/// Get all notes (limited to 1000)
async fn get_notes(
    State(state): State<AppState>,
) -> Result<Json<Vec<Note>>, AppError> {
    let notes = Note::find_all(&state.db).await?;
    Ok(Json(notes))
}

/// Get a single note by ID
async fn get_note(
    State(state): State<AppState>,
    Path(note_id): Path<String>,
) -> Result<Json<Note>, AppError> {
    let note = Note::find_by_id(&state.db, &note_id)
        .await?
        .ok_or(AppError::NotFound(format!("Note {} not found", note_id)))?;

    Ok(Json(note))
}

/// Get branches for a note
async fn get_note_branches(
    State(state): State<AppState>,
    Path(note_id): Path<String>,
) -> Result<Json<Vec<Branch>>, AppError> {
    let branches = Branch::find_by_note_id(&state.db, &note_id).await?;
    Ok(Json(branches))
}

/// Get attributes for a note
async fn get_note_attributes(
    State(state): State<AppState>,
    Path(note_id): Path<String>,
) -> Result<Json<Vec<Attribute>>, AppError> {
    let attributes = Attribute::find_by_note_id(&state.db, &note_id).await?;
    Ok(Json(attributes))
}

/// Get a single branch by ID
async fn get_branch(
    State(state): State<AppState>,
    Path(branch_id): Path<String>,
) -> Result<Json<Branch>, AppError> {
    let branch = Branch::find_by_id(&state.db, &branch_id)
        .await?
        .ok_or(AppError::NotFound(format!("Branch {} not found", branch_id)))?;

    Ok(Json(branch))
}

/// Get child branches of a parent note
async fn get_child_branches(
    State(state): State<AppState>,
    Path(parent_note_id): Path<String>,
) -> Result<Json<Vec<Branch>>, AppError> {
    let branches = Branch::find_children(&state.db, &parent_note_id).await?;
    Ok(Json(branches))
}

/// Get a single attribute by ID
async fn get_attribute(
    State(state): State<AppState>,
    Path(attribute_id): Path<String>,
) -> Result<Json<Attribute>, AppError> {
    let attribute = Attribute::find_by_id(&state.db, &attribute_id)
        .await?
        .ok_or(AppError::NotFound(format!("Attribute {} not found", attribute_id)))?;

    Ok(Json(attribute))
}

/// Update note
#[derive(Debug, Deserialize)]
struct UpdateNoteRequest {
    title: Option<String>,
    #[serde(rename = "type")]
    note_type: Option<String>,
    mime: Option<String>,
}

async fn update_note(
    State(state): State<AppState>,
    Path(note_id): Path<String>,
    Json(payload): Json<UpdateNoteRequest>,
) -> Result<Json<Note>, AppError> {
    let mut note = Note::find_by_id(&state.db, &note_id)
        .await?
        .ok_or(AppError::NotFound(format!("Note {} not found", note_id)))?;

    if let Some(title) = payload.title {
        note.title = title;
    }
    if let Some(note_type) = payload.note_type {
        note.note_type = note_type;
    }
    if let Some(mime) = payload.mime {
        note.mime = mime;
    }

    // Update timestamps
    let now = chrono::Local::now();
    note.date_modified = now.format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    note.utc_date_modified = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();

    note.update(&state.db).await?;

    Ok(Json(note))
}

/// Delete note
async fn delete_note(
    State(state): State<AppState>,
    Path(note_id): Path<String>,
) -> Result<StatusCode, AppError> {
    Note::delete(&state.db, &note_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Get note blob content
async fn get_note_blob(
    State(state): State<AppState>,
    Path(note_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let note = Note::find_by_id(&state.db, &note_id)
        .await?
        .ok_or(AppError::NotFound(format!("Note {} not found", note_id)))?;

    if let Some(blob_id) = &note.blob_id {
        let content = Blob::get_content(&state.db, blob_id)
            .await?
            .ok_or(AppError::NotFound(format!("Blob {} not found", blob_id)))?;

        Ok((StatusCode::OK, content))
    } else {
        Ok((StatusCode::OK, Vec::new()))
    }
}

/// Update note blob content
async fn update_note_blob(
    State(state): State<AppState>,
    Path(note_id): Path<String>,
    body: Bytes,
) -> Result<StatusCode, AppError> {
    let note = Note::find_by_id(&state.db, &note_id)
        .await?
        .ok_or(AppError::NotFound(format!("Note {} not found", note_id)))?;

    let now = chrono::Local::now();
    let utc_now = chrono::Utc::now();
    let date_modified = now.format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    let utc_date_modified = utc_now.format("%Y-%m-%d %H:%M:%S%.3f").to_string();

    if let Some(blob_id) = &note.blob_id {
        // Update existing blob
        Blob::update_content(&state.db, blob_id, &body, &date_modified, &utc_date_modified).await?;
    } else {
        // Create new blob
        let blob_id = uuid::Uuid::new_v4().to_string();
        Blob::insert(&state.db, &blob_id, &body, &date_modified, &utc_date_modified).await?;

        // Update note to reference the blob
        let mut updated_note = note;
        updated_note.blob_id = Some(blob_id);
        updated_note.date_modified = date_modified;
        updated_note.utc_date_modified = utc_date_modified;
        updated_note.update(&state.db).await?;
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Tree node structure
#[derive(Debug, Serialize)]
struct TreeNode {
    #[serde(rename = "noteId")]
    note_id: String,
    #[serde(rename = "branchId")]
    branch_id: String,
    title: String,
    #[serde(rename = "type")]
    note_type: String,
    #[serde(rename = "isProtected")]
    is_protected: bool,
    prefix: Option<String>,
    #[serde(rename = "notePosition")]
    note_position: i32,
    #[serde(rename = "isExpanded")]
    is_expanded: bool,
    children: Vec<TreeNode>,
}

/// Get tree structure
async fn get_tree(
    State(state): State<AppState>,
) -> Result<Json<Vec<TreeNode>>, AppError> {
    // Get root note
    let root_branches = Branch::find_children(&state.db, "root").await?;
    let mut tree_nodes = Vec::new();

    for branch in root_branches {
        if let Some(node) = build_tree_node(&state.db, &branch).await? {
            tree_nodes.push(node);
        }
    }

    Ok(Json(tree_nodes))
}

/// Load tree (POST request with body)
#[derive(Debug, Deserialize)]
struct LoadTreeRequest {
    #[serde(rename = "noteId")]
    note_id: Option<String>,
}

async fn load_tree(
    State(state): State<AppState>,
    Json(payload): Json<LoadTreeRequest>,
) -> Result<Json<Vec<TreeNode>>, AppError> {
    let parent_id = payload.note_id.as_deref().unwrap_or("root");
    let branches = Branch::find_children(&state.db, parent_id).await?;
    let mut tree_nodes = Vec::new();

    for branch in branches {
        if let Some(node) = build_tree_node(&state.db, &branch).await? {
            tree_nodes.push(node);
        }
    }

    Ok(Json(tree_nodes))
}

/// Build tree node recursively
fn build_tree_node<'a>(
    db: &'a Database,
    branch: &'a Branch,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Option<TreeNode>, AppError>> + 'a + Send>> {
    Box::pin(async move {
        if branch.is_deleted() {
            return Ok(None);
        }

        let note = match Note::find_by_id(db, &branch.note_id).await? {
            Some(n) => n,
            None => return Ok(None),
        };

        // Get children recursively
        let child_branches = Branch::find_children(db, &branch.note_id).await?;
        let mut children = Vec::new();

        for child_branch in child_branches {
            if let Some(child_node) = build_tree_node(db, &child_branch).await? {
                children.push(child_node);
            }
        }

        Ok(Some(TreeNode {
            note_id: note.note_id.clone(),
            branch_id: branch.branch_id.clone(),
            title: note.title.clone(),
            note_type: note.note_type.clone(),
            is_protected: note.is_protected(),
            prefix: branch.prefix.clone(),
            note_position: branch.note_position,
            is_expanded: branch.is_expanded(),
            children,
        }))
    })
}

/// Refresh note ordering
async fn refresh_note_ordering(
    State(state): State<AppState>,
    Path(parent_note_id): Path<String>,
) -> Result<StatusCode, AppError> {
    // Get all child branches
    let mut branches = Branch::find_children(&state.db, &parent_note_id).await?;

    // Sort by current position
    branches.sort_by_key(|b| b.note_position);

    // Re-number positions
    for (idx, branch) in branches.iter().enumerate() {
        sqlx::query(
            "UPDATE branches SET notePosition = ? WHERE branchId = ?"
        )
        .bind(idx as i32 * 10)
        .bind(&branch.branch_id)
        .execute(state.db.pool())
        .await?;
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Update branch
#[derive(Debug, Deserialize)]
struct UpdateBranchRequest {
    prefix: Option<String>,
    #[serde(rename = "notePosition")]
    note_position: Option<i32>,
    #[serde(rename = "isExpanded")]
    is_expanded: Option<bool>,
}

async fn update_branch(
    State(state): State<AppState>,
    Path(branch_id): Path<String>,
    Json(payload): Json<UpdateBranchRequest>,
) -> Result<Json<Branch>, AppError> {
    let branch = Branch::find_by_id(&state.db, &branch_id)
        .await?
        .ok_or(AppError::NotFound(format!("Branch {} not found", branch_id)))?;

    let prefix = payload.prefix.or(branch.prefix);
    let note_position = payload.note_position.unwrap_or(branch.note_position);
    let is_expanded = payload.is_expanded.map(|v| if v { 1 } else { 0 }).unwrap_or(branch.is_expanded);

    let utc_now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();

    sqlx::query(
        "UPDATE branches SET prefix = ?, notePosition = ?, isExpanded = ?, utcDateModified = ?
         WHERE branchId = ?"
    )
    .bind(&prefix)
    .bind(note_position)
    .bind(is_expanded)
    .bind(&utc_now)
    .bind(&branch_id)
    .execute(state.db.pool())
    .await?;

    // Fetch updated branch
    let updated_branch = Branch::find_by_id(&state.db, &branch_id)
        .await?
        .ok_or(AppError::NotFound(format!("Branch {} not found", branch_id)))?;

    Ok(Json(updated_branch))
}

/// Delete branch
async fn delete_branch(
    State(state): State<AppState>,
    Path(branch_id): Path<String>,
) -> Result<StatusCode, AppError> {
    let utc_now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();

    sqlx::query(
        "UPDATE branches SET isDeleted = 1, utcDateModified = ? WHERE branchId = ?"
    )
    .bind(&utc_now)
    .bind(&branch_id)
    .execute(state.db.pool())
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Update note title
#[derive(Debug, Deserialize)]
struct UpdateNoteTitleRequest {
    title: String,
}

async fn update_note_title(
    State(state): State<AppState>,
    Path(note_id): Path<String>,
    Json(payload): Json<UpdateNoteTitleRequest>,
) -> Result<Json<Note>, AppError> {
    let mut note = Note::find_by_id(&state.db, &note_id)
        .await?
        .ok_or(AppError::NotFound(format!("Note {} not found", note_id)))?;

    note.title = payload.title;

    let now = chrono::Local::now();
    note.date_modified = now.format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    note.utc_date_modified = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();

    note.update(&state.db).await?;

    Ok(Json(note))
}

/// Update note type
#[derive(Debug, Deserialize)]
struct UpdateNoteTypeRequest {
    #[serde(rename = "type")]
    note_type: String,
    mime: String,
}

async fn update_note_type(
    State(state): State<AppState>,
    Path(note_id): Path<String>,
    Json(payload): Json<UpdateNoteTypeRequest>,
) -> Result<StatusCode, AppError> {
    let mut note = Note::find_by_id(&state.db, &note_id)
        .await?
        .ok_or(AppError::NotFound(format!("Note {} not found", note_id)))?;

    note.note_type = payload.note_type;
    note.mime = payload.mime;

    let now = chrono::Local::now();
    note.date_modified = now.format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    note.utc_date_modified = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();

    note.update(&state.db).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Create note
#[derive(Debug, Deserialize)]
struct CreateNoteRequest {
    title: String,
    #[serde(rename = "type")]
    note_type: Option<String>,
    mime: Option<String>,
    content: Option<String>,
}

async fn create_note(
    State(state): State<AppState>,
    Path(parent_note_id): Path<String>,
    Json(payload): Json<CreateNoteRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Verify parent exists
    Note::find_by_id(&state.db, &parent_note_id)
        .await?
        .ok_or(AppError::NotFound(format!("Parent note {} not found", parent_note_id)))?;

    // Generate IDs
    let note_id = uuid::Uuid::new_v4().to_string();
    let branch_id = uuid::Uuid::new_v4().to_string();

    let now = chrono::Local::now();
    let utc_now = chrono::Utc::now();
    let date_str = now.format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    let utc_date_str = utc_now.format("%Y-%m-%d %H:%M:%S%.3f").to_string();

    let note_type = payload.note_type.unwrap_or_else(|| "text".to_string());
    let mime = payload.mime.unwrap_or_else(|| "text/html".to_string());

    // Create blob if content provided
    let blob_id = if let Some(content) = payload.content {
        let blob_id = uuid::Uuid::new_v4().to_string();
        Blob::insert(&state.db, &blob_id, content.as_bytes(), &date_str, &utc_date_str).await?;
        Some(blob_id)
    } else {
        None
    };

    // Create note
    sqlx::query(
        "INSERT INTO notes (noteId, title, type, mime, isProtected, blobId, isDeleted,
         dateCreated, dateModified, utcDateCreated, utcDateModified)
         VALUES (?, ?, ?, ?, 0, ?, 0, ?, ?, ?, ?)"
    )
    .bind(&note_id)
    .bind(&payload.title)
    .bind(&note_type)
    .bind(&mime)
    .bind(&blob_id)
    .bind(&date_str)
    .bind(&date_str)
    .bind(&utc_date_str)
    .bind(&utc_date_str)
    .execute(state.db.pool())
    .await?;

    // Get max position for parent
    let max_position: Option<i32> = sqlx::query_scalar(
        "SELECT MAX(notePosition) FROM branches WHERE parentNoteId = ? AND isDeleted = 0"
    )
    .bind(&parent_note_id)
    .fetch_optional(state.db.pool())
    .await?
    .flatten();

    let note_position = max_position.unwrap_or(0) + 10;

    // Create branch
    sqlx::query(
        "INSERT INTO branches (branchId, noteId, parentNoteId, notePosition, prefix,
         isExpanded, isDeleted, utcDateModified)
         VALUES (?, ?, ?, ?, NULL, 0, 0, ?)"
    )
    .bind(&branch_id)
    .bind(&note_id)
    .bind(&parent_note_id)
    .bind(note_position)
    .bind(&utc_date_str)
    .execute(state.db.pool())
    .await?;

    // Fetch created note and branch
    let note = Note::find_by_id(&state.db, &note_id).await?.unwrap();
    let branch = Branch::find_by_id(&state.db, &branch_id).await?.unwrap();

    Ok(Json(json!({
        "note": note,
        "branch": branch
    })))
}

/// Update note attributes (bulk replace)
#[derive(Debug, Deserialize)]
struct AttributeInput {
    #[serde(rename = "type")]
    attr_type: String,
    name: String,
    value: Option<String>,
    #[serde(rename = "isInheritable")]
    is_inheritable: Option<bool>,
}

async fn update_note_attributes(
    State(state): State<AppState>,
    Path(note_id): Path<String>,
    Json(attributes): Json<Vec<AttributeInput>>,
) -> Result<StatusCode, AppError> {
    // Verify note exists
    Note::find_by_id(&state.db, &note_id)
        .await?
        .ok_or(AppError::NotFound(format!("Note {} not found", note_id)))?;

    let utc_now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();

    // Soft delete all existing attributes for this note
    sqlx::query(
        "UPDATE attributes SET isDeleted = 1, utcDateModified = ? WHERE noteId = ? AND isDeleted = 0"
    )
    .bind(&utc_now)
    .bind(&note_id)
    .execute(state.db.pool())
    .await?;

    // Insert new attributes
    for (idx, attr) in attributes.iter().enumerate() {
        let attr_id = uuid::Uuid::new_v4().to_string();
        let is_inheritable = attr.is_inheritable.unwrap_or(false);

        sqlx::query(
            "INSERT INTO attributes (attributeId, noteId, type, name, value, position, isInheritable, isDeleted, utcDateModified)
             VALUES (?, ?, ?, ?, ?, ?, ?, 0, ?)"
        )
        .bind(&attr_id)
        .bind(&note_id)
        .bind(&attr.attr_type)
        .bind(&attr.name)
        .bind(&attr.value)
        .bind(idx as i32 * 10)
        .bind(if is_inheritable { 1 } else { 0 })
        .bind(&utc_now)
        .execute(state.db.pool())
        .await?;
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Create attribute
#[derive(Debug, Deserialize)]
struct CreateAttributeRequest {
    #[serde(rename = "noteId")]
    note_id: String,
    #[serde(rename = "type")]
    attr_type: String,
    name: String,
    value: Option<String>,
    #[serde(rename = "isInheritable")]
    is_inheritable: Option<bool>,
}

async fn create_attribute(
    State(state): State<AppState>,
    Json(payload): Json<CreateAttributeRequest>,
) -> Result<Json<Attribute>, AppError> {
    // Verify note exists
    Note::find_by_id(&state.db, &payload.note_id)
        .await?
        .ok_or(AppError::NotFound(format!("Note {} not found", payload.note_id)))?;

    let attr_id = uuid::Uuid::new_v4().to_string();
    let utc_now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    let is_inheritable = payload.is_inheritable.unwrap_or(false);

    // Get max position
    let max_position: Option<i32> = sqlx::query_scalar(
        "SELECT MAX(position) FROM attributes WHERE noteId = ? AND isDeleted = 0"
    )
    .bind(&payload.note_id)
    .fetch_optional(state.db.pool())
    .await?
    .flatten();

    let position = max_position.unwrap_or(0) + 10;

    sqlx::query(
        "INSERT INTO attributes (attributeId, noteId, type, name, value, position, isInheritable, isDeleted, utcDateModified)
         VALUES (?, ?, ?, ?, ?, ?, ?, 0, ?)"
    )
    .bind(&attr_id)
    .bind(&payload.note_id)
    .bind(&payload.attr_type)
    .bind(&payload.name)
    .bind(&payload.value)
    .bind(position)
    .bind(if is_inheritable { 1 } else { 0 })
    .bind(&utc_now)
    .execute(state.db.pool())
    .await?;

    let attribute = Attribute::find_by_id(&state.db, &attr_id)
        .await?
        .ok_or(AppError::NotFound(format!("Attribute {} not found", attr_id)))?;

    Ok(Json(attribute))
}

/// Update attribute
#[derive(Debug, Deserialize)]
struct UpdateAttributeRequest {
    value: Option<String>,
    #[serde(rename = "isInheritable")]
    is_inheritable: Option<bool>,
}

async fn update_attribute(
    State(state): State<AppState>,
    Path(attribute_id): Path<String>,
    Json(payload): Json<UpdateAttributeRequest>,
) -> Result<Json<Attribute>, AppError> {
    let attribute = Attribute::find_by_id(&state.db, &attribute_id)
        .await?
        .ok_or(AppError::NotFound(format!("Attribute {} not found", attribute_id)))?;

    let value = payload.value.or(attribute.value.clone());
    let is_inheritable = payload.is_inheritable
        .map(|v| if v { 1 } else { 0 })
        .unwrap_or(attribute.is_inheritable);

    let utc_now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();

    sqlx::query(
        "UPDATE attributes SET value = ?, isInheritable = ?, utcDateModified = ?
         WHERE attributeId = ?"
    )
    .bind(&value)
    .bind(is_inheritable)
    .bind(&utc_now)
    .bind(&attribute_id)
    .execute(state.db.pool())
    .await?;

    let updated = Attribute::find_by_id(&state.db, &attribute_id)
        .await?
        .ok_or(AppError::NotFound(format!("Attribute {} not found", attribute_id)))?;

    Ok(Json(updated))
}

/// Delete attribute
async fn delete_attribute(
    State(state): State<AppState>,
    Path(attribute_id): Path<String>,
) -> Result<StatusCode, AppError> {
    let utc_now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();

    sqlx::query(
        "UPDATE attributes SET isDeleted = 1, utcDateModified = ? WHERE attributeId = ?"
    )
    .bind(&utc_now)
    .bind(&attribute_id)
    .execute(state.db.pool())
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Search notes by title (basic search)
async fn search_notes(
    State(state): State<AppState>,
    Path(search_string): Path<String>,
) -> Result<Json<Vec<Note>>, AppError> {
    let search_pattern = format!("%{}%", search_string);

    let notes = sqlx::query_as::<_, Note>(
        "SELECT * FROM notes
         WHERE title LIKE ? AND isDeleted = 0
         ORDER BY title
         LIMIT 100"
    )
    .bind(&search_pattern)
    .fetch_all(state.db.pool())
    .await?;

    Ok(Json(notes))
}

/// Search notes by query parameter
#[derive(Debug, Deserialize)]
struct SearchQuery {
    search: String,
}

async fn search_notes_query(
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<SearchQuery>,
) -> Result<Json<Vec<Note>>, AppError> {
    let search_pattern = format!("%{}%", query.search);

    let notes = sqlx::query_as::<_, Note>(
        "SELECT * FROM notes
         WHERE title LIKE ? AND isDeleted = 0
         ORDER BY title
         LIMIT 100"
    )
    .bind(&search_pattern)
    .fetch_all(state.db.pool())
    .await?;

    Ok(Json(notes))
}

/// Get recent changes
#[derive(Debug, Serialize, FromRow)]
struct RecentChange {
    #[sqlx(rename = "noteId")]
    #[serde(rename = "noteId")]
    note_id: String,
    title: String,
    #[sqlx(rename = "dateModified")]
    #[serde(rename = "dateModified")]
    date_modified: String,
    #[sqlx(rename = "utcDateModified")]
    #[serde(rename = "utcDateModified")]
    utc_date_modified: String,
}

async fn get_recent_changes(
    State(state): State<AppState>,
) -> Result<Json<Vec<RecentChange>>, AppError> {
    let notes = sqlx::query_as::<_, RecentChange>(
        "SELECT noteId, title, dateModified, utcDateModified
         FROM notes
         WHERE isDeleted = 0
         ORDER BY utcDateModified DESC
         LIMIT 50"
    )
    .fetch_all(state.db.pool())
    .await?;

    Ok(Json(notes))
}

/// Get all options
async fn get_options(
    State(state): State<AppState>,
) -> Result<Json<Vec<TriliumOption>>, AppError> {
    let options = TriliumOption::get_all(&state.db).await?;
    Ok(Json(options))
}

/// Get single option
async fn get_option(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<TriliumOption>, AppError> {
    let option = TriliumOption::get(&state.db, &name)
        .await?
        .ok_or(AppError::NotFound(format!("Option {} not found", name)))?;

    Ok(Json(option))
}

/// Set option
#[derive(Debug, Deserialize)]
struct SetOptionRequest {
    value: String,
}

async fn set_option(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(payload): Json<SetOptionRequest>,
) -> Result<StatusCode, AppError> {
    TriliumOption::set(&state.db, &name, &payload.value, false).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Application error types
#[derive(Debug)]
pub enum AppError {
    Database(sqlx::Error),
    NotFound(String),
    Internal(anyhow::Error),
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err)
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Internal(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::Database(err) => {
                tracing::error!("Database error: {}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string())
            }
            AppError::NotFound(msg) => {
                (StatusCode::NOT_FOUND, msg)
            }
            AppError::Internal(err) => {
                tracing::error!("Internal error: {}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            }
        };

        let body = Json(json!({
            "error": message
        }));

        (status, body).into_response()
    }
}
