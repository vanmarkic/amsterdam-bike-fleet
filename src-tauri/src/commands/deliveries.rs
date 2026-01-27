//! Delivery Tauri Commands
//!
//! # Purpose
//! Exposes delivery data to the Angular frontend via Tauri IPC.
//!
//! # Why Tauri Commands instead of HTTP?
//! - No network exposure (localhost isn't needed)
//! - Native binary execution (compiled Rust, not JavaScript)
//! - Built-in serialization with serde
//! - Can be wrapped with encryption layer
//!
//! # Security
//! These commands can be called directly for development.
//! In production, they should be wrapped by `secure_invoke`
//! which encrypts all payloads.

use crate::database::DatabaseError;
use crate::models::Delivery;
use crate::AppState;
use tauri::State;

/// Get all deliveries with optional filtering
///
/// # Arguments
/// - `bike_id`: Filter by deliverer (optional)
/// - `status`: Filter by status: "completed", "ongoing", "upcoming" (optional)
///
/// # Returns
/// Vec<Delivery> - List of deliveries matching filters, sorted by created_at DESC
///
/// # Why optional filters?
/// - Flexibility: UI can show all deliveries or filtered view
/// - Efficiency: Database-level filtering is faster than client-side
#[tauri::command]
pub fn get_deliveries(
    state: State<'_, AppState>,
    bike_id: Option<String>,
    status: Option<String>,
) -> Result<Vec<Delivery>, DatabaseError> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or(DatabaseError::NotInitialized)?;

    db.get_deliveries(
        bike_id.as_deref(),
        status.as_deref(),
    )
}

/// Get a single delivery by ID
///
/// # Returns
/// - Some(Delivery) if found
/// - None if not found (not an error - client should handle)
#[tauri::command]
pub fn get_delivery_by_id(
    state: State<'_, AppState>,
    delivery_id: String,
) -> Result<Option<Delivery>, DatabaseError> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or(DatabaseError::NotInitialized)?;

    db.get_delivery_by_id(&delivery_id)
}

/// Get deliveries for a specific bike (for force graph)
///
/// # Why a dedicated command?
/// - Force graph always needs all deliveries for one bike
/// - Cleaner API than passing filter params
/// - Could be optimized differently in the future
#[tauri::command]
pub fn get_deliveries_for_bike(
    state: State<'_, AppState>,
    bike_id: String,
) -> Result<Vec<Delivery>, DatabaseError> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard
        .as_ref()
        .ok_or(DatabaseError::NotInitialized)?;

    db.get_deliveries_by_bike(&bike_id)
}
