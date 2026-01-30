//! PostgreSQL Issue Tauri Commands
//!
//! Async versions of issue commands for PostgreSQL backend.

use crate::database_pg::DatabaseError;
use crate::models::Issue;
use crate::AppState;
use tauri::State;

/// Get all issues with optional filtering
#[tauri::command]
pub async fn get_issues(
    state: State<'_, AppState>,
    bike_id: Option<String>,
    resolved: Option<bool>,
    category: Option<String>,
) -> Result<Vec<Issue>, DatabaseError> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or(DatabaseError::NotInitialized)?;

    db.get_issues(bike_id.as_deref(), resolved, category.as_deref()).await
}

/// Get a single issue by ID
#[tauri::command]
pub async fn get_issue_by_id(
    state: State<'_, AppState>,
    issue_id: String,
) -> Result<Option<Issue>, DatabaseError> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or(DatabaseError::NotInitialized)?;

    db.get_issue_by_id(&issue_id).await
}

/// Get issues for a specific bike (for force graph)
#[tauri::command]
pub async fn get_issues_for_bike(
    state: State<'_, AppState>,
    bike_id: String,
) -> Result<Vec<Issue>, DatabaseError> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or(DatabaseError::NotInitialized)?;

    db.get_issues_by_bike(&bike_id).await
}
