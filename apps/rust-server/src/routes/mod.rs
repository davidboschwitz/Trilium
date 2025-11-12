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

use crate::db::Database;
use crate::entities::{Note, Branch, Attribute, Blob};

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
        .route("/api/notes/:noteId/branches", get(get_note_branches))
        .route("/api/notes/:noteId/attributes", get(get_note_attributes))
        .route("/api/notes/:noteId/blob", get(get_note_blob).put(update_note_blob))
        .route("/api/notes/:noteId/data", get(get_note_blob).put(update_note_blob))

        // Tree API
        .route("/api/tree", get(get_tree).post(load_tree))
        .route("/api/refresh-note-ordering/:parentNoteId", post(refresh_note_ordering))

        // Branches API
        .route("/api/branches/:branchId", get(get_branch).put(update_branch).delete(delete_branch))
        .route("/api/branches/parent/:parentNoteId", get(get_child_branches))

        // Attributes API
        .route("/api/attributes/:attributeId", get(get_attribute))

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
