// Library entry point for Tauri v2

mod commands;
mod database;
mod models;

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
        .manage(AppState {
            db: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            commands::fleet::get_fleet_data,
            commands::fleet::get_bike_by_id,
            commands::fleet::add_bike,
            commands::fleet::update_bike_status,
            commands::fleet::get_fleet_stats,
            commands::database::init_database,
            commands::database::get_database_stats,
            commands::database::is_database_initialized,
            commands::health::health_check,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
