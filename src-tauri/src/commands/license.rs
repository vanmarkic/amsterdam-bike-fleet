//! Tauri commands for license management

use crate::license::{self, LicenseStatus, LicenseStorage};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize, Deserialize)]
pub struct ActivateLicenseResponse {
    pub success: bool,
    pub status: LicenseStatus,
    pub message: String,
}

/// Activate a license key
///
/// Verifies the license and stores it if valid.
#[tauri::command]
pub async fn activate_license(
    app: AppHandle,
    license_key: String,
) -> Result<ActivateLicenseResponse, String> {
    // Get app data directory for license storage
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    let storage = LicenseStorage::new(app_data_dir);

    // Verify the license
    let status = license::get_license_status(&license_key);

    if status.valid {
        // Store the license
        storage
            .save(&license_key)
            .map_err(|e| format!("Failed to save license: {}", e))?;

        Ok(ActivateLicenseResponse {
            success: true,
            status,
            message: "License activated successfully".to_string(),
        })
    } else {
        let error_msg = status
            .error
            .clone()
            .unwrap_or_else(|| "Unknown error".to_string());
        Ok(ActivateLicenseResponse {
            success: false,
            status,
            message: format!("License verification failed: {}", error_msg),
        })
    }
}

/// Get current license status
///
/// Loads the stored license (if any) and returns its status.
#[tauri::command]
pub async fn get_license_status(app: AppHandle) -> Result<LicenseStatus, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    let storage = LicenseStorage::new(app_data_dir);

    if !storage.exists() {
        return Ok(LicenseStatus {
            valid: false,
            info: None,
            error: Some("No license found".to_string()),
            days_remaining: None,
        });
    }

    match storage.load() {
        Ok(license_key) => Ok(license::get_license_status(&license_key)),
        Err(e) => Ok(LicenseStatus {
            valid: false,
            info: None,
            error: Some(format!("Failed to load license: {}", e)),
            days_remaining: None,
        }),
    }
}

/// Deactivate (remove) the current license
#[tauri::command]
pub async fn deactivate_license(app: AppHandle) -> Result<String, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    let storage = LicenseStorage::new(app_data_dir);

    storage
        .remove()
        .map_err(|e| format!("Failed to remove license: {}", e))?;

    Ok("License deactivated".to_string())
}

/// Check if a feature is licensed
///
/// Returns true if the current license includes the specified feature.
#[tauri::command]
pub async fn is_feature_licensed(app: AppHandle, feature: String) -> Result<bool, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    let storage = LicenseStorage::new(app_data_dir);

    if !storage.exists() {
        return Ok(false);
    }

    match storage.load() {
        Ok(license_key) => Ok(license::is_feature_licensed(&license_key, &feature)),
        Err(_) => Ok(false),
    }
}

/// Validate a license key without storing it
///
/// Use this to check if a key is valid before activating.
#[tauri::command]
pub async fn validate_license(license_key: String) -> Result<LicenseStatus, String> {
    Ok(license::get_license_status(&license_key))
}
