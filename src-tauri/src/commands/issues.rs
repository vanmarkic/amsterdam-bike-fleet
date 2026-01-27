//! Issue Tauri Commands
//!
//! # Purpose
//! Exposes issue/problem tracking data to the Angular frontend via Tauri IPC.
//!
//! # Data Flow
//! 1. Angular calls `invoke('get_issues', { bikeId: '...', resolved: false })`
//! 2. Tauri routes to this command
//! 3. Command queries SQLite via database.rs
//! 4. Returns serialized Vec<Issue>
//!
//! # Force Graph Integration
//! Issues are secondary nodes in the force graph:
//! - Linked to a delivery (if delivery_id is present)
//! - Or directly to the deliverer (if standalone issue)

use crate::database::DatabaseError;
use crate::models::Issue;
use crate::AppState;
use tauri::State;

/// Get all issues with optional filtering
///
/// # Arguments
/// - `bike_id`: Filter by deliverer (optional)
/// - `resolved`: Filter by resolution status (optional)
/// - `category`: Filter by issue category (optional)
///
/// # Returns
/// Vec<Issue> - List of issues matching filters, sorted by created_at DESC
#[tauri::command]
pub fn get_issues(
    state: State<'_, AppState>,
    bike_id: Option<String>,
    resolved: Option<bool>,
    category: Option<String>,
) -> Result<Vec<Issue>, DatabaseError> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or(DatabaseError::NotInitialized)?;

    db.get_issues(
        bike_id.as_deref(),
        resolved,
        category.as_deref(),
    )
}

/// Get a single issue by ID
#[tauri::command]
pub fn get_issue_by_id(
    state: State<'_, AppState>,
    issue_id: String,
) -> Result<Option<Issue>, DatabaseError> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or(DatabaseError::NotInitialized)?;

    db.get_issue_by_id(&issue_id)
}

/// Get issues for a specific bike (for force graph)
///
/// # Force Graph Usage
/// This command is called when building the force graph for a deliverer.
/// All issues for that bike become nodes, linked either:
/// - To a delivery node (if issue.delivery_id is Some)
/// - Directly to the center deliverer node (if standalone)
#[tauri::command]
pub fn get_issues_for_bike(
    state: State<'_, AppState>,
    bike_id: String,
) -> Result<Vec<Issue>, DatabaseError> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or(DatabaseError::NotInitialized)?;

    db.get_issues_by_bike(&bike_id)
}
