//! Encrypted IPC Module
//!
//! # Purpose
//! Provides session-based encryption for Tauri IPC communication.
//! All command payloads are encrypted with ChaCha20-Poly1305 to prevent:
//! - Reverse engineering of API structure via IPC inspection
//! - Man-in-the-middle attacks on IPC channel
//! - Payload tampering (AEAD provides integrity)
//!
//! # Key Derivation
//! Session keys are derived from the license key using HKDF-SHA256.
//! This means:
//! - No additional secrets to manage
//! - Each session gets a unique key (via random nonce)
//! - License revocation also revokes encryption capability
//!
//! # Security Model
//! - Session nonce generated at app startup (random 16 bytes)
//! - Key derived using HKDF: license_key + session_nonce → encryption_key
//! - Each message uses incrementing nonce (counter mode)
//! - AEAD tag prevents tampering

use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use hkdf::Hkdf;
use sha2::Sha256;
use std::sync::atomic::{AtomicU64, Ordering};
use thiserror::Error;

/// Nonce size for ChaCha20-Poly1305 (96 bits = 12 bytes)
const NONCE_SIZE: usize = 12;

/// Session nonce size for key derivation (128 bits = 16 bytes)
const SESSION_NONCE_SIZE: usize = 16;

/// HKDF info string - identifies the purpose of derived key
/// Changing this would produce different keys even with same inputs
const HKDF_INFO: &[u8] = b"amsterdam-bike-fleet-ipc-v1";

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("Key derivation failed: {0}")]
    KeyDerivationFailed(String),

    #[error("Invalid nonce length")]
    InvalidNonceLength,

    #[error("Nonce counter overflow")]
    NonceOverflow,
}

impl serde::Serialize for CryptoError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Session-based encryption context
///
/// # Why session-based?
/// - Each app session gets a unique key
/// - Derived from license key (ties encryption to valid license)
/// - Nonce counter ensures unique nonces per message
///
/// # Thread Safety
/// - AtomicU64 for nonce counter enables concurrent encryption
/// - ChaCha20Poly1305 is internally immutable after creation
pub struct SessionCrypto {
    /// The ChaCha20-Poly1305 cipher instance
    cipher: ChaCha20Poly1305,

    /// Monotonically increasing nonce counter
    /// Each encryption increments this to ensure unique nonces
    nonce_counter: AtomicU64,
}

impl SessionCrypto {
    /// Create a new session crypto context from license key
    ///
    /// # Arguments
    /// - `license_key`: The validated license key string
    /// - `session_nonce`: Random 16 bytes generated at session start
    ///
    /// # Key Derivation Process
    /// 1. Use license_key as Input Key Material (IKM)
    /// 2. Use session_nonce as salt (ensures unique keys per session)
    /// 3. HKDF-SHA256 expands to 256-bit key
    /// 4. Info string provides domain separation
    ///
    /// # Why this approach?
    /// - License key alone would produce same key every session
    /// - Random salt ensures attacker can't precompute keys
    /// - HKDF is cryptographically sound key derivation
    pub fn from_license(
        license_key: &str,
        session_nonce: &[u8; SESSION_NONCE_SIZE],
    ) -> Result<Self, CryptoError> {
        // Input Key Material: the license key bytes
        let ikm = license_key.as_bytes();

        // Salt: random session nonce
        let salt = session_nonce;

        // Create HKDF instance
        let hk = Hkdf::<Sha256>::new(Some(salt), ikm);

        // Expand to 256-bit key
        let mut key = [0u8; 32];
        hk.expand(HKDF_INFO, &mut key)
            .map_err(|e| CryptoError::KeyDerivationFailed(e.to_string()))?;

        // Create cipher from derived key
        let cipher = ChaCha20Poly1305::new(&key.into());

        Ok(Self {
            cipher,
            nonce_counter: AtomicU64::new(0),
        })
    }

    /// Encrypt plaintext data
    ///
    /// # Returns
    /// Ciphertext with format: [nonce (12 bytes)][encrypted data + tag]
    ///
    /// # Why prepend nonce?
    /// - Receiver needs nonce to decrypt
    /// - Nonce is not secret, just must be unique
    /// - Prepending is simpler than separate transmission
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        // Get next nonce value
        let counter = self
            .nonce_counter
            .fetch_add(1, Ordering::SeqCst);

        // Build 12-byte nonce from counter
        // First 4 bytes: zeros (could be used for additional entropy)
        // Last 8 bytes: counter value (little-endian)
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        nonce_bytes[4..12].copy_from_slice(&counter.to_le_bytes());
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt with AEAD
        let ciphertext = self
            .cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

        // Prepend nonce to ciphertext
        let mut result = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    /// Decrypt ciphertext data
    ///
    /// # Arguments
    /// - `ciphertext`: Data with format [nonce (12 bytes)][encrypted + tag]
    ///
    /// # Why AEAD?
    /// - Authentication tag ensures data wasn't tampered with
    /// - Decryption fails if tag doesn't match
    /// - Prevents chosen-ciphertext attacks
    pub fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        // Validate minimum length (nonce + at least tag)
        if ciphertext.len() < NONCE_SIZE + 16 {
            // 16 = Poly1305 tag size
            return Err(CryptoError::DecryptionFailed(
                "Ciphertext too short".to_string(),
            ));
        }

        // Extract nonce from first 12 bytes
        let nonce = Nonce::from_slice(&ciphertext[..NONCE_SIZE]);

        // Decrypt remaining bytes
        let plaintext = self
            .cipher
            .decrypt(nonce, &ciphertext[NONCE_SIZE..])
            .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?;

        Ok(plaintext)
    }

    /// Generate a random session nonce
    ///
    /// # Why use rand?
    /// - Cryptographically secure random bytes
    /// - Each session needs unique nonce for key derivation
    pub fn generate_session_nonce() -> [u8; SESSION_NONCE_SIZE] {
        use rand::RngCore;
        let mut nonce = [0u8; SESSION_NONCE_SIZE];
        rand::thread_rng().fill_bytes(&mut nonce);
        nonce
    }
}

// ============================================================================
// Secure Command Protocol
// ============================================================================

use serde::{Deserialize, Serialize};

/// Commands that can be invoked through encrypted IPC
///
/// # Why an enum?
/// - Type-safe command routing
/// - All variants serialized with bincode (binary, not JSON)
/// - Adding new commands requires updating this enum
/// - Compiler enforces handling all variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecureCommand {
    // Delivery commands
    GetDeliveries {
        bike_id: Option<String>,
        status: Option<String>,
    },
    GetDeliveryById {
        delivery_id: String,
    },

    // Issue commands
    GetIssues {
        bike_id: Option<String>,
        resolved: Option<bool>,
        category: Option<String>,
    },
    GetIssueById {
        issue_id: String,
    },

    // Force graph commands
    GetForceGraphLayout {
        bike_id: String,
    },
    UpdateNodePosition {
        bike_id: String,
        node_id: String,
        x: f64,
        y: f64,
    },
}

/// Response wrapper for secure commands
///
/// # Why a wrapper?
/// - Consistent error handling across all commands
/// - Payload is bincode-serialized, then encrypted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecureResponse {
    Success(Vec<u8>), // Bincode-serialized payload
    Error(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let session_nonce = SessionCrypto::generate_session_nonce();
        let crypto =
            SessionCrypto::from_license("test-license-key", &session_nonce).unwrap();

        let plaintext = b"Hello, encrypted world!";
        let ciphertext = crypto.encrypt(plaintext).unwrap();
        let decrypted = crypto.decrypt(&ciphertext).unwrap();

        assert_eq!(plaintext.as_slice(), decrypted.as_slice());
    }

    #[test]
    fn test_different_sessions_different_keys() {
        let nonce1 = SessionCrypto::generate_session_nonce();
        let nonce2 = SessionCrypto::generate_session_nonce();

        let crypto1 = SessionCrypto::from_license("same-key", &nonce1).unwrap();
        let crypto2 = SessionCrypto::from_license("same-key", &nonce2).unwrap();

        let plaintext = b"Test message";
        let ciphertext1 = crypto1.encrypt(plaintext).unwrap();

        // crypto2 should fail to decrypt crypto1's ciphertext
        let result = crypto2.decrypt(&ciphertext1);
        assert!(result.is_err());
    }

    #[test]
    fn test_tampered_ciphertext_fails() {
        let session_nonce = SessionCrypto::generate_session_nonce();
        let crypto =
            SessionCrypto::from_license("test-license-key", &session_nonce).unwrap();

        let plaintext = b"Sensitive data";
        let mut ciphertext = crypto.encrypt(plaintext).unwrap();

        // Tamper with the ciphertext
        if let Some(byte) = ciphertext.get_mut(20) {
            *byte ^= 0xFF;
        }

        // Decryption should fail due to authentication tag mismatch
        let result = crypto.decrypt(&ciphertext);
        assert!(result.is_err());
    }

    #[test]
    fn test_nonce_uniqueness() {
        let session_nonce = SessionCrypto::generate_session_nonce();
        let crypto =
            SessionCrypto::from_license("test-license-key", &session_nonce).unwrap();

        let plaintext = b"Same message";

        // Encrypt same message twice
        let ciphertext1 = crypto.encrypt(plaintext).unwrap();
        let ciphertext2 = crypto.encrypt(plaintext).unwrap();

        // Ciphertexts should be different (different nonces)
        assert_ne!(ciphertext1, ciphertext2);

        // But both should decrypt to same plaintext
        let decrypted1 = crypto.decrypt(&ciphertext1).unwrap();
        let decrypted2 = crypto.decrypt(&ciphertext2).unwrap();
        assert_eq!(decrypted1, decrypted2);
    }

    #[test]
    fn test_bincode_command_serialization() {
        let cmd = SecureCommand::GetForceGraphLayout {
            bike_id: "BIKE-0001".to_string(),
        };

        let serialized = bincode::serialize(&cmd).unwrap();
        let deserialized: SecureCommand = bincode::deserialize(&serialized).unwrap();

        match deserialized {
            SecureCommand::GetForceGraphLayout { bike_id } => {
                assert_eq!(bike_id, "BIKE-0001");
            }
            _ => panic!("Wrong variant"),
        }
    }
}
