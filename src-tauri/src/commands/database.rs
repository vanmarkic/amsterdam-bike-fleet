use crate::database::Database;
use crate::models::DatabaseStats;
use crate::AppState;
use tauri::{AppHandle, Manager, State};

/// Initialize the SQLite database connection
/// Creates the database file in the app data directory if it doesn't exist
#[tauri::command]
pub fn init_database(
    app_handle: AppHandle,
    state: State<AppState>,
) -> Result<String, String> {
    // Get the app data directory using Tauri v2 API
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    // Create the directory if it doesn't exist
    std::fs::create_dir_all(&app_data_dir).map_err(|e| e.to_string())?;

    // Database file path
    let db_path = app_data_dir.join("amsterdam_bike_fleet.db");

    // Initialize the database
    let db = Database::new(db_path.clone()).map_err(|e| e.to_string())?;

    // Store in app state
    let mut db_guard = state.db.lock().map_err(|e| e.to_string())?;
    *db_guard = Some(db);

    Ok(format!(
        "Database initialized successfully at: {}",
        db_path.display()
    ))
}

/// Get database statistics
#[tauri::command]
pub fn get_database_stats(state: State<AppState>) -> Result<DatabaseStats, String> {
    let db_guard = state.db.lock().map_err(|e| e.to_string())?;

    match db_guard.as_ref() {
        Some(db) => db.get_stats().map_err(|e| e.to_string()),
        None => Err("Database not initialized. Call init_database first.".to_string()),
    }
}

/// Check if database is initialized
#[tauri::command]
pub fn is_database_initialized(state: State<AppState>) -> Result<bool, String> {
    let db_guard = state.db.lock().map_err(|e| e.to_string())?;
    Ok(db_guard.is_some())
}
