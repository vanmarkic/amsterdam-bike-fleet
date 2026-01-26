use serde::{Deserialize, Serialize};

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub rust_version: String,
    pub tauri_version: String,
    pub timestamp: String,
}

/// Health check command to verify the Rust backend is running
#[tauri::command]
pub fn health_check() -> HealthStatus {
    HealthStatus {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        rust_version: rustc_version(),
        tauri_version: "1.8".to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    }
}

/// Get the Rust compiler version (simplified)
fn rustc_version() -> String {
    "1.70+".to_string()
}
