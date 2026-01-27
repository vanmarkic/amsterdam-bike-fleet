//! Secure IPC Command Wrapper
//!
//! # Purpose
//! Provides a single entry point for encrypted IPC communication.
//! All commands are routed through `secure_invoke`, which:
//! 1. Decrypts the incoming payload
//! 2. Routes to the appropriate handler
//! 3. Encrypts the response
//!
//! # Why a Single Entry Point?
//! - Easier to audit security (one place to check encryption)
//! - Uniform error handling
//! - Simpler to add logging/monitoring
//! - Attacker sees only one command name, not the internal API
//!
//! # Wire Format
//! Request: ChaCha20-Poly1305 encrypted bincode
//! Response: ChaCha20-Poly1305 encrypted bincode
//!
//! # Session Initialization
//! Before using secure_invoke:
//! 1. Client calls `init_secure_session` with valid license
//! 2. Server generates session nonce, derives key
//! 3. Server returns session nonce (client derives same key)
//! 4. All subsequent calls use encrypted payloads

use crate::crypto::{SecureCommand, SecureResponse, SessionCrypto};
use crate::database::DatabaseError;
use crate::models::ForceGraphData;
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::State;

/// Session state holding the crypto context
///
/// # Why separate from AppState?
/// - Crypto context is optional (only exists after init_secure_session)
/// - Clear separation of concerns
/// - Can be reset independently (e.g., on license change)
pub struct SecureSessionState {
    pub crypto: Mutex<Option<SessionCrypto>>,
}

/// Response from session initialization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecureSessionInfo {
    /// Session nonce (16 bytes, base64 encoded)
    /// Client uses this + license key to derive the same encryption key
    pub session_nonce_base64: String,

    /// Whether the session was successfully initialized
    pub initialized: bool,
}

/// Initialize a secure session
///
/// # Flow
/// 1. Client provides license key
/// 2. Server validates license
/// 3. Server generates random session nonce
/// 4. Server derives encryption key: HKDF(license_key, session_nonce)
/// 5. Server returns session_nonce to client
/// 6. Client derives same key locally
///
/// # Why derive key on both sides?
/// - License key never sent over IPC after this call
/// - Both sides compute same key from shared secret (license) + public nonce
/// - Nonce ensures unique key per session
#[tauri::command]
pub fn init_secure_session(
    _state: State<'_, AppState>,
    secure_state: State<'_, SecureSessionState>,
    license_key: String,
) -> Result<SecureSessionInfo, String> {
    // Validate license first
    match crate::license::verify_license(&license_key) {
        Ok(_license_info) => {
            // License valid, create session
            let session_nonce = SessionCrypto::generate_session_nonce();

            let crypto = SessionCrypto::from_license(&license_key, &session_nonce)
                .map_err(|e| e.to_string())?;

            // Store crypto context
            let mut crypto_guard = secure_state.crypto.lock().unwrap();
            *crypto_guard = Some(crypto);

            // Return nonce (base64 encoded for JSON transport)
            let nonce_base64 = base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                &session_nonce,
            );

            Ok(SecureSessionInfo {
                session_nonce_base64: nonce_base64,
                initialized: true,
            })
        }
        Err(e) => Err(format!("License validation failed: {}", e)),
    }
}

/// Secure invoke - single entry point for all encrypted commands
///
/// # Arguments
/// - `encrypted_payload`: ChaCha20-Poly1305 encrypted, bincode-serialized SecureCommand
///
/// # Returns
/// - ChaCha20-Poly1305 encrypted, bincode-serialized response
///
/// # Error Handling
/// Errors are also encrypted to prevent leaking information via error messages
#[tauri::command]
pub fn secure_invoke(
    state: State<'_, AppState>,
    secure_state: State<'_, SecureSessionState>,
    encrypted_payload: Vec<u8>,
) -> Result<Vec<u8>, String> {
    // Get crypto context
    let crypto_guard = secure_state.crypto.lock().unwrap();
    let crypto = crypto_guard
        .as_ref()
        .ok_or("Secure session not initialized. Call init_secure_session first.")?;

    // Decrypt request
    let decrypted = crypto
        .decrypt(&encrypted_payload)
        .map_err(|e| format!("Decryption failed: {}", e))?;

    // Deserialize command (bincode)
    let command: SecureCommand = bincode::deserialize(&decrypted)
        .map_err(|e| format!("Invalid command format: {}", e))?;

    // Route and execute command
    let response = execute_secure_command(&state, command);

    // Serialize response (bincode)
    let response_bytes = bincode::serialize(&response)
        .map_err(|e| format!("Response serialization failed: {}", e))?;

    // Encrypt response
    crypto
        .encrypt(&response_bytes)
        .map_err(|e| format!("Response encryption failed: {}", e))
}

/// Route and execute a secure command
fn execute_secure_command(state: &State<'_, AppState>, command: SecureCommand) -> SecureResponse {
    match command {
        SecureCommand::GetDeliveries { bike_id, status } => {
            execute_get_deliveries(state, bike_id, status)
        }
        SecureCommand::GetDeliveryById { delivery_id } => {
            execute_get_delivery_by_id(state, delivery_id)
        }
        SecureCommand::GetIssues {
            bike_id,
            resolved,
            category,
        } => execute_get_issues(state, bike_id, resolved, category),
        SecureCommand::GetIssueById { issue_id } => execute_get_issue_by_id(state, issue_id),
        SecureCommand::GetForceGraphLayout { bike_id } => {
            execute_get_force_graph_layout(state, bike_id)
        }
        SecureCommand::UpdateNodePosition {
            bike_id,
            node_id,
            x,
            y,
        } => execute_update_node_position(state, bike_id, node_id, x, y),
    }
}

// ============================================================================
// Command Handlers
// ============================================================================

fn execute_get_deliveries(
    state: &State<'_, AppState>,
    bike_id: Option<String>,
    status: Option<String>,
) -> SecureResponse {
    let db_guard = state.db.lock().unwrap();
    match db_guard.as_ref() {
        Some(db) => match db.get_deliveries(bike_id.as_deref(), status.as_deref()) {
            Ok(deliveries) => match bincode::serialize(&deliveries) {
                Ok(bytes) => SecureResponse::Success(bytes),
                Err(e) => SecureResponse::Error(e.to_string()),
            },
            Err(e) => SecureResponse::Error(e.to_string()),
        },
        None => SecureResponse::Error("Database not initialized".to_string()),
    }
}

fn execute_get_delivery_by_id(
    state: &State<'_, AppState>,
    delivery_id: String,
) -> SecureResponse {
    let db_guard = state.db.lock().unwrap();
    match db_guard.as_ref() {
        Some(db) => match db.get_delivery_by_id(&delivery_id) {
            Ok(delivery) => match bincode::serialize(&delivery) {
                Ok(bytes) => SecureResponse::Success(bytes),
                Err(e) => SecureResponse::Error(e.to_string()),
            },
            Err(e) => SecureResponse::Error(e.to_string()),
        },
        None => SecureResponse::Error("Database not initialized".to_string()),
    }
}

fn execute_get_issues(
    state: &State<'_, AppState>,
    bike_id: Option<String>,
    resolved: Option<bool>,
    category: Option<String>,
) -> SecureResponse {
    let db_guard = state.db.lock().unwrap();
    match db_guard.as_ref() {
        Some(db) => match db.get_issues(bike_id.as_deref(), resolved, category.as_deref()) {
            Ok(issues) => match bincode::serialize(&issues) {
                Ok(bytes) => SecureResponse::Success(bytes),
                Err(e) => SecureResponse::Error(e.to_string()),
            },
            Err(e) => SecureResponse::Error(e.to_string()),
        },
        None => SecureResponse::Error("Database not initialized".to_string()),
    }
}

fn execute_get_issue_by_id(state: &State<'_, AppState>, issue_id: String) -> SecureResponse {
    let db_guard = state.db.lock().unwrap();
    match db_guard.as_ref() {
        Some(db) => match db.get_issue_by_id(&issue_id) {
            Ok(issue) => match bincode::serialize(&issue) {
                Ok(bytes) => SecureResponse::Success(bytes),
                Err(e) => SecureResponse::Error(e.to_string()),
            },
            Err(e) => SecureResponse::Error(e.to_string()),
        },
        None => SecureResponse::Error("Database not initialized".to_string()),
    }
}

fn execute_get_force_graph_layout(
    state: &State<'_, AppState>,
    bike_id: String,
) -> SecureResponse {
    // Note: This duplicates logic from force_graph.rs but with different error handling
    // In production, you'd want to refactor to share the core logic
    let db_guard = state.db.lock().unwrap();
    match db_guard.as_ref() {
        Some(db) => {
            let result = (|| -> Result<ForceGraphData, DatabaseError> {
                let bike = db
                    .get_bike_by_id(&bike_id)?
                    .ok_or_else(|| {
                        DatabaseError::InvalidData(format!("Bike not found: {}", bike_id))
                    })?;
                let deliveries = db.get_deliveries_by_bike(&bike_id)?;
                let issues = db.get_issues_by_bike(&bike_id)?;

                // Use the force_graph module's logic
                crate::commands::force_graph::get_force_graph_layout_internal(
                    &bike, &deliveries, &issues,
                )
            })();

            match result {
                Ok(layout) => match bincode::serialize(&layout) {
                    Ok(bytes) => SecureResponse::Success(bytes),
                    Err(e) => SecureResponse::Error(e.to_string()),
                },
                Err(e) => SecureResponse::Error(e.to_string()),
            }
        }
        None => SecureResponse::Error("Database not initialized".to_string()),
    }
}

fn execute_update_node_position(
    state: &State<'_, AppState>,
    bike_id: String,
    node_id: String,
    x: f64,
    y: f64,
) -> SecureResponse {
    let db_guard = state.db.lock().unwrap();
    match db_guard.as_ref() {
        Some(db) => {
            let result = (|| -> Result<ForceGraphData, DatabaseError> {
                let bike = db
                    .get_bike_by_id(&bike_id)?
                    .ok_or_else(|| {
                        DatabaseError::InvalidData(format!("Bike not found: {}", bike_id))
                    })?;
                let deliveries = db.get_deliveries_by_bike(&bike_id)?;
                let issues = db.get_issues_by_bike(&bike_id)?;

                crate::commands::force_graph::update_node_position_internal(
                    &bike, &deliveries, &issues, &node_id, x, y,
                )
            })();

            match result {
                Ok(layout) => match bincode::serialize(&layout) {
                    Ok(bytes) => SecureResponse::Success(bytes),
                    Err(e) => SecureResponse::Error(e.to_string()),
                },
                Err(e) => SecureResponse::Error(e.to_string()),
            }
        }
        None => SecureResponse::Error("Database not initialized".to_string()),
    }
}
