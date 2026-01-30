//! PostgreSQL Delivery Tauri Commands
//!
//! Async versions of delivery commands for PostgreSQL backend.

use crate::database_pg::DatabaseError;
use crate::models::Delivery;
use crate::AppState;
use tauri::State;

/// Get all deliveries with optional filtering
#[tauri::command]
pub async fn get_deliveries(
    state: State<'_, AppState>,
    bike_id: Option<String>,
    status: Option<String>,
) -> Result<Vec<Delivery>, DatabaseError> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or(DatabaseError::NotInitialized)?;

    db.get_deliveries(bike_id.as_deref(), status.as_deref()).await
}

/// Get a single delivery by ID
#[tauri::command]
pub async fn get_delivery_by_id(
    state: State<'_, AppState>,
    delivery_id: String,
) -> Result<Option<Delivery>, DatabaseError> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or(DatabaseError::NotInitialized)?;

    db.get_delivery_by_id(&delivery_id).await
}

/// Get deliveries for a specific bike (for force graph)
#[tauri::command]
pub async fn get_deliveries_for_bike(
    state: State<'_, AppState>,
    bike_id: String,
) -> Result<Vec<Delivery>, DatabaseError> {
    let db_guard = state.db.lock().unwrap();
    let db = db_guard.as_ref().ok_or(DatabaseError::NotInitialized)?;

    db.get_deliveries_by_bike(&bike_id).await
}
