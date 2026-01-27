//! License verification module using Ed25519 signatures
//!
//! License keys are cryptographically signed payloads that can be verified offline.
//! The private key is kept secret (in the license generator tool).
//! Only the public key is embedded in this binary.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{DateTime, Utc};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use thiserror::Error;

/// The Ed25519 public key for license verification (32 bytes, base64 encoded)
/// IMPORTANT: This is the PUBLIC key - safe to embed in the binary.
/// The PRIVATE key must NEVER be included here - it stays in the license generator.
///
/// Generated with: cargo run --bin license-generator -- --generate-keys
const PUBLIC_KEY_BASE64: &str = "9-olA_QuQjwR-cPw9ZmN_QnFSdJCUf4iTBhXXsNqbI0";

/// License key prefix for easy identification
const LICENSE_PREFIX: &str = "ABF-";

#[derive(Error, Debug)]
pub enum LicenseError {
    #[error("Invalid license key format")]
    InvalidFormat,

    #[error("Invalid base64 encoding: {0}")]
    Base64Error(#[from] base64::DecodeError),

    #[error("Invalid JSON payload: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("License has expired (expired: {0})")]
    Expired(String),

    #[error("License is for a different product: {0}")]
    WrongProduct(String),

    #[error("Feature not included in license: {0}")]
    FeatureNotLicensed(String),

    #[error("Public key not configured")]
    PublicKeyNotConfigured,

    #[error("Invalid public key: {0}")]
    InvalidPublicKey(String),

    #[error("License file error: {0}")]
    FileError(String),
}

/// Information embedded in a license key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseInfo {
    /// Customer email (for tracking if shared)
    pub customer: String,

    /// Company name (optional)
    #[serde(default)]
    pub company: Option<String>,

    /// Product identifier (must match our product)
    pub product: String,

    /// Expiration date (ISO 8601 format)
    pub expires: String,

    /// Licensed features
    #[serde(default)]
    pub features: Vec<String>,

    /// Maximum number of seats (optional)
    #[serde(default)]
    pub seats: Option<u32>,

    /// License issued date
    #[serde(default)]
    pub issued: Option<String>,

    /// License version (for future format changes)
    #[serde(default = "default_version")]
    pub version: u32,
}

fn default_version() -> u32 {
    1
}

impl LicenseInfo {
    /// Check if the license has expired
    pub fn is_expired(&self) -> bool {
        match DateTime::parse_from_rfc3339(&self.expires) {
            Ok(expires) => Utc::now() > expires,
            Err(_) => {
                // Try parsing as date only (YYYY-MM-DD)
                match chrono::NaiveDate::parse_from_str(&self.expires, "%Y-%m-%d") {
                    Ok(date) => {
                        let expires = date
                            .and_hms_opt(23, 59, 59)
                            .unwrap()
                            .and_utc();
                        Utc::now() > expires
                    }
                    Err(_) => true, // Invalid date format = expired
                }
            }
        }
    }

    /// Check if a feature is licensed
    pub fn has_feature(&self, feature: &str) -> bool {
        self.features.iter().any(|f| f == feature || f == "*")
    }

    /// Get days until expiration (negative if expired)
    pub fn days_until_expiry(&self) -> i64 {
        let expires = match DateTime::parse_from_rfc3339(&self.expires) {
            Ok(dt) => dt.with_timezone(&Utc),
            Err(_) => {
                match chrono::NaiveDate::parse_from_str(&self.expires, "%Y-%m-%d") {
                    Ok(date) => date.and_hms_opt(23, 59, 59).unwrap().and_utc(),
                    Err(_) => return -9999,
                }
            }
        };
        (expires - Utc::now()).num_days()
    }
}

/// License verification result returned to the frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseStatus {
    pub valid: bool,
    pub info: Option<LicenseInfo>,
    pub error: Option<String>,
    pub days_remaining: Option<i64>,
}

/// Verify a license key and extract its information
///
/// License key format: ABF-<base64(payload_json + signature_64bytes)>
pub fn verify_license(license_key: &str) -> Result<LicenseInfo, LicenseError> {
    // Check placeholder hasn't been replaced
    if PUBLIC_KEY_BASE64 == "REPLACE_WITH_YOUR_PUBLIC_KEY_BASE64_HERE" {
        return Err(LicenseError::PublicKeyNotConfigured);
    }

    // Parse the public key
    let public_key_bytes = URL_SAFE_NO_PAD
        .decode(PUBLIC_KEY_BASE64)
        .map_err(|e| LicenseError::InvalidPublicKey(e.to_string()))?;

    if public_key_bytes.len() != 32 {
        return Err(LicenseError::InvalidPublicKey(format!(
            "Expected 32 bytes, got {}",
            public_key_bytes.len()
        )));
    }

    let public_key = VerifyingKey::from_bytes(
        public_key_bytes
            .as_slice()
            .try_into()
            .map_err(|_| LicenseError::InvalidPublicKey("Invalid key bytes".to_string()))?,
    )
    .map_err(|e| LicenseError::InvalidPublicKey(e.to_string()))?;

    // Remove prefix and validate format
    let key_data = license_key
        .strip_prefix(LICENSE_PREFIX)
        .ok_or(LicenseError::InvalidFormat)?;

    // Remove any dashes used for readability (ABF-XXXX-XXXX-XXXX-XXXX format)
    let key_clean: String = key_data.chars().filter(|c| *c != '-').collect();

    // Decode base64
    let decoded = URL_SAFE_NO_PAD.decode(&key_clean)?;

    // Must have at least 64 bytes for signature + some payload
    if decoded.len() < 65 {
        return Err(LicenseError::InvalidFormat);
    }

    // Split into payload and signature
    // Format: [payload_bytes...][signature_64_bytes]
    let signature_start = decoded.len() - 64;
    let payload_bytes = &decoded[..signature_start];
    let signature_bytes = &decoded[signature_start..];

    // Parse signature
    let signature = Signature::from_bytes(
        signature_bytes
            .try_into()
            .map_err(|_| LicenseError::InvalidSignature)?,
    );

    // Verify signature
    public_key
        .verify(payload_bytes, &signature)
        .map_err(|_| LicenseError::InvalidSignature)?;

    // Parse JSON payload
    let info: LicenseInfo = serde_json::from_slice(payload_bytes)?;

    // Validate product
    if info.product != "amsterdam-bike-fleet" && info.product != "*" {
        return Err(LicenseError::WrongProduct(info.product.clone()));
    }

    // Check expiration
    if info.is_expired() {
        return Err(LicenseError::Expired(info.expires.clone()));
    }

    Ok(info)
}

/// Get the status of a license key (for UI display)
pub fn get_license_status(license_key: &str) -> LicenseStatus {
    match verify_license(license_key) {
        Ok(info) => {
            let days = info.days_until_expiry();
            LicenseStatus {
                valid: true,
                info: Some(info),
                error: None,
                days_remaining: Some(days),
            }
        }
        Err(e) => LicenseStatus {
            valid: false,
            info: None,
            error: Some(e.to_string()),
            days_remaining: None,
        },
    }
}

/// Check if a specific feature is licensed
pub fn is_feature_licensed(license_key: &str, feature: &str) -> bool {
    match verify_license(license_key) {
        Ok(info) => info.has_feature(feature),
        Err(_) => false,
    }
}

/// License storage manager - handles persisting license to disk
pub struct LicenseStorage {
    storage_path: PathBuf,
}

impl LicenseStorage {
    pub fn new(app_data_dir: PathBuf) -> Self {
        Self {
            storage_path: app_data_dir.join("license.key"),
        }
    }

    /// Save license key to disk
    pub fn save(&self, license_key: &str) -> Result<(), LicenseError> {
        fs::create_dir_all(self.storage_path.parent().unwrap())
            .map_err(|e| LicenseError::FileError(e.to_string()))?;

        fs::write(&self.storage_path, license_key)
            .map_err(|e| LicenseError::FileError(e.to_string()))?;

        Ok(())
    }

    /// Load license key from disk
    pub fn load(&self) -> Result<String, LicenseError> {
        fs::read_to_string(&self.storage_path)
            .map(|s| s.trim().to_string())
            .map_err(|e| LicenseError::FileError(e.to_string()))
    }

    /// Remove stored license
    pub fn remove(&self) -> Result<(), LicenseError> {
        if self.storage_path.exists() {
            fs::remove_file(&self.storage_path)
                .map_err(|e| LicenseError::FileError(e.to_string()))?;
        }
        Ok(())
    }

    /// Check if a license is stored
    pub fn exists(&self) -> bool {
        self.storage_path.exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_license_info_expiry() {
        let info = LicenseInfo {
            customer: "test@example.com".to_string(),
            company: None,
            product: "amsterdam-bike-fleet".to_string(),
            expires: "2099-12-31".to_string(),
            features: vec!["premium".to_string()],
            seats: None,
            issued: None,
            version: 1,
        };

        assert!(!info.is_expired());
        assert!(info.has_feature("premium"));
        assert!(!info.has_feature("enterprise"));
    }

    #[test]
    fn test_expired_license() {
        let info = LicenseInfo {
            customer: "test@example.com".to_string(),
            company: None,
            product: "amsterdam-bike-fleet".to_string(),
            expires: "2020-01-01".to_string(),
            features: vec![],
            seats: None,
            issued: None,
            version: 1,
        };

        assert!(info.is_expired());
    }

    #[test]
    fn test_wildcard_feature() {
        let info = LicenseInfo {
            customer: "test@example.com".to_string(),
            company: None,
            product: "amsterdam-bike-fleet".to_string(),
            expires: "2099-12-31".to_string(),
            features: vec!["*".to_string()],
            seats: None,
            issued: None,
            version: 1,
        };

        assert!(info.has_feature("anything"));
        assert!(info.has_feature("premium"));
        assert!(info.has_feature("enterprise"));
    }
}
