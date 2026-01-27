//! License Key Generator for Amsterdam Bike Fleet
//!
//! This tool generates cryptographically signed license keys.
//!
//! SECURITY: Keep this tool and the private key SECRET!
//! Never distribute this tool or the private key with the application.
//!
//! Usage:
//!   # Generate a new keypair (do this ONCE, save the output!)
//!   cargo run -- --generate-keys
//!
//!   # Generate a license key
//!   cargo run -- --private-key <KEY> --customer "john@acme.com" --expires "2026-12-31"
//!
//!   # Generate with all options
//!   cargo run -- --private-key <KEY> \
//!     --customer "john@acme.com" \
//!     --company "ACME Corp" \
//!     --expires "2026-12-31" \
//!     --features "premium,export,api" \
//!     --seats 5

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::Utc;
use clap::Parser;
use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

/// License key prefix
const LICENSE_PREFIX: &str = "ABF-";

#[derive(Parser, Debug)]
#[command(name = "license-generator")]
#[command(about = "Generate signed license keys for Amsterdam Bike Fleet")]
struct Args {
    /// Generate a new Ed25519 keypair
    #[arg(long)]
    generate_keys: bool,

    /// Private key (base64 encoded, 32 bytes)
    #[arg(long)]
    private_key: Option<String>,

    /// Customer email
    #[arg(long)]
    customer: Option<String>,

    /// Company name (optional)
    #[arg(long)]
    company: Option<String>,

    /// Expiration date (YYYY-MM-DD format)
    #[arg(long)]
    expires: Option<String>,

    /// Comma-separated list of features (e.g., "premium,export,api")
    #[arg(long)]
    features: Option<String>,

    /// Number of seats (optional)
    #[arg(long)]
    seats: Option<u32>,

    /// Verify an existing license key
    #[arg(long)]
    verify: Option<String>,

    /// Public key for verification (base64 encoded)
    #[arg(long)]
    public_key: Option<String>,
}

/// License payload structure (must match the app's LicenseInfo)
#[derive(Debug, Serialize, Deserialize)]
struct LicensePayload {
    customer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    company: Option<String>,
    product: String,
    expires: String,
    features: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    seats: Option<u32>,
    issued: String,
    version: u32,
}

fn main() {
    let args = Args::parse();

    if args.generate_keys {
        generate_keypair();
        return;
    }

    if let Some(license_key) = args.verify {
        if let Some(public_key) = args.public_key {
            verify_license(&license_key, &public_key);
        } else {
            eprintln!("Error: --public-key is required for verification");
            std::process::exit(1);
        }
        return;
    }

    // Generate a license key
    let private_key = args.private_key.unwrap_or_else(|| {
        eprintln!("Error: --private-key is required to generate a license");
        eprintln!("Run with --generate-keys to create a new keypair");
        std::process::exit(1);
    });

    let customer = args.customer.unwrap_or_else(|| {
        eprintln!("Error: --customer is required");
        std::process::exit(1);
    });

    let expires = args.expires.unwrap_or_else(|| {
        eprintln!("Error: --expires is required (format: YYYY-MM-DD)");
        std::process::exit(1);
    });

    let features: Vec<String> = args
        .features
        .map(|f| f.split(',').map(|s| s.trim().to_string()).collect())
        .unwrap_or_default();

    generate_license(
        &private_key,
        &customer,
        args.company,
        &expires,
        features,
        args.seats,
    );
}

fn generate_keypair() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║           ED25519 KEYPAIR GENERATION                          ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  SECURITY WARNING:                                            ║");
    println!("║  • SAVE these keys securely!                                  ║");
    println!("║  • NEVER share the private key                                ║");
    println!("║  • NEVER commit the private key to git                        ║");
    println!("║  • Store private key in a password manager or HSM             ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key: VerifyingKey = (&signing_key).into();

    let private_key_b64 = URL_SAFE_NO_PAD.encode(signing_key.to_bytes());
    let public_key_b64 = URL_SAFE_NO_PAD.encode(verifying_key.to_bytes());

    println!("┌─ PRIVATE KEY (KEEP SECRET!) ─────────────────────────────────┐");
    println!("│ {}  │", private_key_b64);
    println!("└─────────────────────────────────────────────────────────────────┘");
    println!();
    println!("┌─ PUBLIC KEY (embed in src-tauri/src/license.rs) ────────────┐");
    println!("│ {}  │", public_key_b64);
    println!("└─────────────────────────────────────────────────────────────────┘");
    println!();
    println!("Next steps:");
    println!("1. Copy the PUBLIC KEY above");
    println!("2. Open src-tauri/src/license.rs");
    println!("3. Replace the placeholder in PUBLIC_KEY_BASE64 constant");
    println!();
    println!("Example:");
    println!("  const PUBLIC_KEY_BASE64: &str = \"{}\";", public_key_b64);
}

fn generate_license(
    private_key_b64: &str,
    customer: &str,
    company: Option<String>,
    expires: &str,
    features: Vec<String>,
    seats: Option<u32>,
) {
    // Decode private key
    let private_key_bytes = match URL_SAFE_NO_PAD.decode(private_key_b64) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("Error: Invalid private key format: {}", e);
            std::process::exit(1);
        }
    };

    if private_key_bytes.len() != 32 {
        eprintln!(
            "Error: Private key must be 32 bytes, got {}",
            private_key_bytes.len()
        );
        std::process::exit(1);
    }

    let signing_key = SigningKey::from_bytes(
        private_key_bytes
            .as_slice()
            .try_into()
            .expect("Invalid key length"),
    );

    // Create license payload
    let payload = LicensePayload {
        customer: customer.to_string(),
        company,
        product: "amsterdam-bike-fleet".to_string(),
        expires: expires.to_string(),
        features,
        seats,
        issued: Utc::now().format("%Y-%m-%d").to_string(),
        version: 1,
    };

    let payload_json = serde_json::to_string(&payload).expect("Failed to serialize payload");
    let payload_bytes = payload_json.as_bytes();

    // Sign the payload
    let signature = signing_key.sign(payload_bytes);

    // Combine payload + signature
    let mut combined = payload_bytes.to_vec();
    combined.extend_from_slice(&signature.to_bytes());

    // Encode as base64
    let encoded = URL_SAFE_NO_PAD.encode(&combined);

    // Format with dashes for readability (groups of 4)
    let formatted = format_license_key(&encoded);

    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║                    LICENSE KEY GENERATED                      ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();
    println!("Customer: {}", payload.customer);
    if let Some(ref company) = payload.company {
        println!("Company:  {}", company);
    }
    println!("Expires:  {}", payload.expires);
    if !payload.features.is_empty() {
        println!("Features: {}", payload.features.join(", "));
    }
    if let Some(seats) = payload.seats {
        println!("Seats:    {}", seats);
    }
    println!();
    println!("┌─ LICENSE KEY ────────────────────────────────────────────────┐");
    println!("│");
    println!("│  {}{}", LICENSE_PREFIX, formatted);
    println!("│");
    println!("└─────────────────────────────────────────────────────────────────┘");
    println!();

    // Also output the raw key for programmatic use
    println!("Raw (single line):");
    println!("{}{}", LICENSE_PREFIX, encoded);
}

fn format_license_key(encoded: &str) -> String {
    encoded
        .chars()
        .collect::<Vec<char>>()
        .chunks(4)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect::<Vec<String>>()
        .join("-")
}

fn verify_license(license_key: &str, public_key_b64: &str) {
    println!();
    println!("Verifying license key...");
    println!();

    // Decode public key
    let public_key_bytes = match URL_SAFE_NO_PAD.decode(public_key_b64) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("Error: Invalid public key format: {}", e);
            std::process::exit(1);
        }
    };

    let verifying_key = match VerifyingKey::from_bytes(
        public_key_bytes
            .as_slice()
            .try_into()
            .expect("Invalid key length"),
    ) {
        Ok(key) => key,
        Err(e) => {
            eprintln!("Error: Invalid public key: {}", e);
            std::process::exit(1);
        }
    };

    // Remove prefix
    let key_data = license_key.strip_prefix(LICENSE_PREFIX).unwrap_or(license_key);

    // Remove dashes
    let key_clean: String = key_data.chars().filter(|c| *c != '-').collect();

    // Decode
    let decoded = match URL_SAFE_NO_PAD.decode(&key_clean) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("Error: Invalid license format: {}", e);
            std::process::exit(1);
        }
    };

    if decoded.len() < 65 {
        eprintln!("Error: License key too short");
        std::process::exit(1);
    }

    // Split payload and signature
    let signature_start = decoded.len() - 64;
    let payload_bytes = &decoded[..signature_start];
    let signature_bytes = &decoded[signature_start..];

    // Verify signature
    let signature = ed25519_dalek::Signature::from_bytes(
        signature_bytes.try_into().expect("Invalid signature length"),
    );

    match verifying_key.verify(payload_bytes, &signature) {
        Ok(()) => {
            println!("✅ Signature VALID");
            println!();

            // Parse and display payload
            match serde_json::from_slice::<LicensePayload>(payload_bytes) {
                Ok(payload) => {
                    println!("License Details:");
                    println!("  Customer: {}", payload.customer);
                    if let Some(company) = payload.company {
                        println!("  Company:  {}", company);
                    }
                    println!("  Product:  {}", payload.product);
                    println!("  Expires:  {}", payload.expires);
                    println!("  Issued:   {}", payload.issued);
                    if !payload.features.is_empty() {
                        println!("  Features: {}", payload.features.join(", "));
                    }
                    if let Some(seats) = payload.seats {
                        println!("  Seats:    {}", seats);
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Could not parse payload: {}", e);
                }
            }
        }
        Err(_) => {
            println!("❌ Signature INVALID");
            std::process::exit(1);
        }
    }
}
