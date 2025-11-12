use axum::{
    Router,
    routing::{get, post},
    extract::{State, Path},
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use serde_json::json;

use crate::db::Database;
use crate::entities::{Note, Branch, Attribute};

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
        .route("/api/notes/:noteId", get(get_note))
        .route("/api/notes/:noteId/branches", get(get_note_branches))
        .route("/api/notes/:noteId/attributes", get(get_note_attributes))

        // Branches API
        .route("/api/branches/:branchId", get(get_branch))
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
