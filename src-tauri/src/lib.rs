//! Amsterdam Bike Fleet - Tauri Backend
//!
//! # Architecture
//! This is the native Rust backend for the fleet management application.
//! It provides:
//! - Database backend (SQLite or PostgreSQL based on feature flags)
//! - Encrypted IPC for secure communication with Angular frontend
//! - Force graph layout computation using Fjädra
//! - License verification
//!
//! # Database Backends
//! - **SQLite** (default): Embedded database for standalone desktop use
//! - **PostgreSQL**: For on-premise HA deployments with Patroni cluster
//!
//! Build with PostgreSQL:
//! ```bash
//! cargo build --no-default-features --features postgres
//! ```
//!
//! # Security Model
//! - All business logic runs here (compiled native binary)
//! - IPC payloads are encrypted with ChaCha20-Poly1305
//! - Session keys derived from license key (HKDF)
//! - No algorithms exposed to browser

mod commands;
pub mod crypto;
pub mod license;
mod models;

// Database backend selection via feature flags
#[cfg(feature = "sqlite")]
mod database;
#[cfg(feature = "sqlite")]
pub use database::Database;

#[cfg(feature = "postgres")]
mod database_pg;
#[cfg(feature = "postgres")]
pub use database_pg::{Database, DatabaseConfig, SharedDatabase};

use commands::secure::SecureSessionState;
use std::sync::Mutex;

// ============================================================================
// Application State
// ============================================================================

/// Application state for SQLite backend (synchronous)
#[cfg(feature = "sqlite")]
pub struct AppState {
    pub db: Mutex<Option<database::Database>>,
}

/// Application state for PostgreSQL backend (async with connection pool)
#[cfg(feature = "postgres")]
pub struct AppState {
    pub db: Mutex<Option<database_pg::SharedDatabase>>,
}

// ============================================================================
// Tauri Entry Point
// ============================================================================

#[cfg(feature = "sqlite")]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        // Core application state
        .manage(AppState {
            db: Mutex::new(None),
        })
        // Secure session state (holds encryption context)
        .manage(SecureSessionState {
            crypto: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            // Database initialization
            commands::database::init_database,
            commands::database::get_database_stats,
            commands::database::is_database_initialized,

            // Health check
            commands::health::health_check,

            // License management (Phase 1)
            commands::license::activate_license,
            commands::license::get_license_status,
            commands::license::deactivate_license,
            commands::license::is_feature_licensed,
            commands::license::validate_license,

            // Fleet data (legacy - direct commands)
            commands::fleet::get_fleet_data,
            commands::fleet::get_bike_by_id,
            commands::fleet::add_bike,
            commands::fleet::update_bike_status,
            commands::fleet::get_fleet_stats,

            // Delivery commands (direct, for development)
            commands::deliveries::get_deliveries,
            commands::deliveries::get_delivery_by_id,
            commands::deliveries::get_deliveries_for_bike,

            // Issue commands (direct, for development)
            commands::issues::get_issues,
            commands::issues::get_issue_by_id,
            commands::issues::get_issues_for_bike,

            // Force graph commands (direct, for development)
            commands::force_graph::get_force_graph_layout,
            commands::force_graph::update_node_position,

            // Secure IPC (encrypted commands - production use)
            commands::secure::init_secure_session,
            commands::secure::secure_invoke,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(feature = "postgres")]
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // PostgreSQL version uses async setup
    use tokio::runtime::Runtime;

    let rt = Runtime::new().expect("Failed to create Tokio runtime");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        // Core application state (will be initialized by init_database command)
        .manage(AppState {
            db: Mutex::new(None),
        })
        // Secure session state (holds encryption context)
        .manage(SecureSessionState {
            crypto: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            // Database initialization (PostgreSQL version)
            commands::database_pg::init_database,
            commands::database_pg::get_database_stats,
            commands::database_pg::is_database_initialized,
            commands::database_pg::database_health_check,

            // Health check
            commands::health::health_check,

            // License management (Phase 1)
            commands::license::activate_license,
            commands::license::get_license_status,
            commands::license::deactivate_license,
            commands::license::is_feature_licensed,
            commands::license::validate_license,

            // Fleet data (PostgreSQL async versions)
            commands::fleet_pg::get_fleet_data,
            commands::fleet_pg::get_bike_by_id,
            commands::fleet_pg::add_bike,
            commands::fleet_pg::update_bike_status,
            commands::fleet_pg::get_fleet_stats,

            // Delivery commands (PostgreSQL async versions)
            commands::deliveries_pg::get_deliveries,
            commands::deliveries_pg::get_delivery_by_id,
            commands::deliveries_pg::get_deliveries_for_bike,

            // Issue commands (PostgreSQL async versions)
            commands::issues_pg::get_issues,
            commands::issues_pg::get_issue_by_id,
            commands::issues_pg::get_issues_for_bike,

            // Force graph commands (PostgreSQL async versions)
            commands::force_graph_pg::get_force_graph_layout,
            commands::force_graph_pg::update_node_position,

            // Secure IPC (encrypted commands - production use)
            commands::secure::init_secure_session,
            commands::secure::secure_invoke,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
