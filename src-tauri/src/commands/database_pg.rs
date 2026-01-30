//! PostgreSQL Database Commands for Tauri
//!
//! This module provides async Tauri commands for PostgreSQL operations.
//! Used when the application is built with --features postgres.

use crate::database_pg::{create_shared_database, DatabaseConfig};
use crate::models::DatabaseStats;
use crate::AppState;
use tauri::State;

/// Initialize the PostgreSQL database connection pool
///
/// Reads configuration from environment variables:
/// - PG_HOST: PostgreSQL host (default: localhost, use HAProxy VIP for HA)
/// - PG_PORT: PostgreSQL port (default: 5432)
/// - PG_USER: Database user (default: fleet_app)
/// - PG_PASSWORD: Database password (required)
/// - PG_DATABASE: Database name (default: bike_fleet)
/// - PG_POOL_SIZE: Connection pool size (default: 16)
///
/// # Example
/// ```bash
/// export PG_HOST=10.0.0.100  # HAProxy VIP
/// export PG_PASSWORD=your_secure_password
/// ./amsterdam-bike-fleet
/// ```
#[tauri::command]
pub async fn init_database(state: State<'_, AppState>) -> Result<String, String> {
    // Get configuration from environment
    let config = DatabaseConfig::from_env().map_err(|e| e.to_string())?;

    let host = config.host.clone();
    let port = config.port;
    let dbname = config.dbname.clone();

    // Create connection pool
    let db = create_shared_database(config)
        .await
        .map_err(|e| format!("Failed to connect to PostgreSQL: {}", e))?;

    // Store in app state
    let mut db_guard = state.db.lock().map_err(|e| e.to_string())?;
    *db_guard = Some(db);

    Ok(format!(
        "PostgreSQL database initialized successfully at: {}:{}/{}",
        host, port, dbname
    ))
}

/// Get database statistics
#[tauri::command]
pub async fn get_database_stats(state: State<'_, AppState>) -> Result<DatabaseStats, String> {
    let db_guard = state.db.lock().map_err(|e| e.to_string())?;

    match db_guard.as_ref() {
        Some(db) => db.get_stats().await.map_err(|e| e.to_string()),
        None => Err("Database not initialized. Call init_database first.".to_string()),
    }
}

/// Check if database is initialized
#[tauri::command]
pub fn is_database_initialized(state: State<AppState>) -> Result<bool, String> {
    let db_guard = state.db.lock().map_err(|e| e.to_string())?;
    Ok(db_guard.is_some())
}

/// Check database health and connectivity
///
/// Returns:
/// - `primary`: Connected to primary (read-write)
/// - `replica`: Connected to replica (read-only)
/// - Error if connection failed
///
/// This is useful for monitoring and alerting on database status.
#[tauri::command]
pub async fn database_health_check(state: State<'_, AppState>) -> Result<String, String> {
    let db_guard = state.db.lock().map_err(|e| e.to_string())?;

    match db_guard.as_ref() {
        Some(db) => {
            let is_primary = db.health_check().await.map_err(|e| e.to_string())?;
            if is_primary {
                Ok("primary".to_string())
            } else {
                Ok("replica".to_string())
            }
        }
        None => Err("Database not initialized".to_string()),
    }
}
