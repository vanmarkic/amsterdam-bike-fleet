//! Amsterdam Bike Fleet - Tauri Backend
//!
//! # Architecture
//! This is the native Rust backend for the fleet management application.
//! It provides:
//! - SQLite database for persistent storage
//! - Encrypted IPC for secure communication with Angular frontend
//! - Force graph layout computation using Fjädra
//! - License verification
//!
//! # Security Model
//! - All business logic runs here (compiled native binary)
//! - IPC payloads are encrypted with ChaCha20-Poly1305
//! - Session keys derived from license key (HKDF)
//! - No algorithms exposed to browser

mod commands;
pub mod crypto;
mod database;
pub mod license;
mod models;

use commands::secure::SecureSessionState;
use database::Database;
use std::sync::Mutex;

/// Application state holding the database connection
pub struct AppState {
    pub db: Mutex<Option<Database>>,
}

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
