# Licensing Strategy

## Overview

This document describes the chosen licensing approach for the Amsterdam Bike Fleet desktop application. The strategy is designed for an app that:

- Loads content from **customer's servers** (not a single controlled domain)
- Must work **offline** (no mandatory phone-home)
- Cannot rely on **hardware fingerprinting** (cross-platform, VMs, hardware changes)

---

## Chosen Approach: Two-Phase Implementation

### Phase 1: Signed License Keys (Implement First)

Cryptographically signed license keys that work completely offline.

```
┌─────────────────────────────────────────────────────────────┐
│                    HOW IT WORKS                              │
│                                                              │
│  1. Customer purchases license                               │
│                                                              │
│  2. You generate a signed license key containing:            │
│     {                                                        │
│       "customer": "john@acme.com",                           │
│       "company": "ACME Corp",                                │
│       "product": "amsterdam-bike-fleet",                     │
│       "expires": "2026-12-31",                               │
│       "features": ["premium", "export", "api"]               │
│     }                                                        │
│                                                              │
│  3. Key is signed with YOUR private key (Ed25519)            │
│     → Only you can create valid keys                         │
│                                                              │
│  4. Customer receives license key:                           │
│     ABF-XXXX-XXXX-XXXX-XXXX                                  │
│     (base64 encoded: payload + signature)                    │
│                                                              │
│  5. App verifies with PUBLIC key (compiled in Rust binary)   │
│     → Can't be forged                                        │
│     → Works offline                                          │
│     → Traceable if shared                                    │
└─────────────────────────────────────────────────────────────┘
```

#### Benefits

| Benefit | Description |
|---------|-------------|
| **Offline** | No internet required for verification |
| **Unforgeable** | Ed25519 signatures can't be faked without private key |
| **Traceable** | Customer info embedded - you know who shared it |
| **Flexible** | Encode expiration, features, seat count, etc. |
| **Secure** | Verification in compiled Rust is very hard to bypass |

#### License Key Format

```
ABF-XXXX-XXXX-XXXX-XXXX

Where XXXX-XXXX-XXXX-XXXX is base64 encoding of:
┌────────────────────────────────────────┐
│  Payload (JSON, variable length)       │
│  ────────────────────────────────────  │
│  Signature (64 bytes, Ed25519)         │
└────────────────────────────────────────┘
```

#### Implementation Components

1. **License Generator** (separate CLI tool, keep private!)
   - Inputs: customer info, expiration, features
   - Outputs: signed license key
   - Uses PRIVATE key (never distribute)

2. **License Verifier** (in Tauri Rust backend)
   - Inputs: license key from user
   - Uses PUBLIC key (compiled into binary)
   - Returns: valid/invalid + license info

---

### Phase 2: Domain Verification (Add Later)

Extends Phase 1 to tie licenses to specific customer domains.

```
┌─────────────────────────────────────────────────────────────┐
│                    HOW IT WORKS                              │
│                                                              │
│  1. License key includes authorized domains:                 │
│     {                                                        │
│       "customer": "john@acme.com",                           │
│       "domains": [                                           │
│         "fleet.acme.com",                                    │
│         "staging.acme.com",                                  │
│         "localhost"                                          │
│       ],                                                     │
│       "expires": "2026-12-31"                                │
│     }                                                        │
│                                                              │
│  2. App checks WebView URL on every navigation               │
│                                                              │
│  3. If URL host doesn't match authorized domains:            │
│     → App refuses to load content                            │
│     → Shows "Unauthorized domain" error                      │
│                                                              │
│  4. Result: License only works on CUSTOMER'S servers         │
└─────────────────────────────────────────────────────────────┘
```

#### Benefits

| Benefit | Description |
|---------|-------------|
| **Prevents sharing** | License + app copy won't work on other servers |
| **Per-customer isolation** | Each customer's license tied to their domains |
| **Flexible** | Allow localhost for development, multiple domains |
| **Combines with Phase 1** | Signature + domain = strong protection |

---

## Why Not Hardware Fingerprinting?

| Issue | Impact |
|-------|--------|
| Hardware changes (RAM, disk, GPU) | License breaks, support tickets |
| Virtual machines | Fingerprint changes on VM migration/snapshot |
| Multiple devices per user | Need separate licenses per machine |
| Cross-platform differences | macOS vs Windows generate different fingerprints |
| Privacy concerns | GDPR, user objections to hardware tracking |
| Containerized environments | Docker/K8s have no stable hardware identity |

**Conclusion:** Hardware fingerprinting is fragile and creates poor user experience. Domain-based verification (Phase 2) achieves similar goals with fewer problems.

---

## Protection Stack (Complete)

```
┌─────────────────────────────────────────────────────────────┐
│                 COMPLETE PROTECTION STACK                    │
│                                                              │
│  Layer 6: Domain Verification                    [Phase 2]   │
│  └── License tied to customer's server domains              │
│  └── App verifies WebView URL matches license               │
│                                                              │
│  Layer 5: Signed License Keys                    [Phase 1]   │
│  └── Ed25519 cryptographic signatures                       │
│  └── Offline verification in compiled Rust                  │
│                                                              │
│  Layer 4: Binary Protection                      [Optional]  │
│  └── VMProtect / Themida for anti-debugging                 │
│                                                              │
│  Layer 3: Rust Backend                           [✅ Done]   │
│  └── Business logic compiled to native machine code         │
│  └── SQLite (default) or PostgreSQL (--features postgres)   │
│                                                              │
│  Layer 2: WASM Module                            [✅ Done]   │
│  └── Client-side algorithms in WebAssembly binary           │
│  └── Haversine, validation, statistics                      │
│                                                              │
│  Layer 1: JavaScript Obfuscation                 [✅ Done]   │
│  └── UI code obfuscated (low value anyway)                  │
│  └── npm run build:protected                                │
└─────────────────────────────────────────────────────────────┘
```

---

## Implementation Status

### Phase 1: Signed License Keys ✅ IMPLEMENTED

| Component | Status | Location |
|-----------|--------|----------|
| Ed25519 crates | ✅ Done | `src-tauri/Cargo.toml` |
| Keypair generated | ✅ Done | Public key in `license.rs`, private key kept secret |
| License module | ✅ Done | `src-tauri/src/license.rs` |
| Tauri commands | ✅ Done | `src-tauri/src/commands/license.rs` |
| License generator | ✅ Done | `license-generator/` (separate crate) |
| Angular service | ✅ Done | `src/app/services/license.service.ts` |

### Files Created

```
amsterdam-bike-fleet/
├── src-tauri/src/
│   ├── license.rs              # License verification (Ed25519)
│   └── commands/license.rs     # Tauri IPC commands
├── license-generator/          # Separate CLI tool (keep private!)
│   ├── Cargo.toml
│   └── src/main.rs
└── src/app/services/
    ├── tauri.service.ts        # Extended with license commands
    └── license.service.ts      # Angular license state management
```

### Using the License Generator

```bash
# Navigate to the generator
cd license-generator

# Generate a new keypair (do this ONCE, save securely!)
cargo run -- --generate-keys

# Generate a license key
cargo run -- \
  --private-key="YOUR_PRIVATE_KEY" \
  --customer="customer@example.com" \
  --company="ACME Corp" \
  --expires="2027-12-31" \
  --features="premium,export,api"

# Verify a license key
cargo run -- \
  --verify="ABF-..." \
  --public-key="YOUR_PUBLIC_KEY"
```

### Tauri Commands Available

| Command | Description |
|---------|-------------|
| `activate_license` | Verify and store a license key |
| `get_license_status` | Get current license status |
| `deactivate_license` | Remove stored license |
| `is_feature_licensed` | Check if a feature is licensed |
| `validate_license` | Validate key without storing |

### Angular Service Usage

```typescript
import { LicenseService } from './services/license.service';

@Component({...})
export class MyComponent {
  constructor(public license: LicenseService) {}

  // Template: <div *ngIf="license.isLicensed$ | async">Premium</div>

  async activate() {
    const response = await this.license.activateLicense('ABF-...');
    if (response.success) {
      console.log('Licensed to:', response.status.info?.customer);
    }
  }

  // Check feature sync
  get hasPremium(): boolean {
    return this.license.hasFeature('premium');
  }
}
```

### License Expiration

Licenses include an expiration date that is **enforced at runtime**.

**Generating licenses with expiration:**

```bash
# 1-year license
cargo run -- \
  --private-key="YOUR_KEY" \
  --customer="customer@example.com" \
  --expires="2027-01-27"

# Trial license (7 days)
cargo run -- \
  --private-key="YOUR_KEY" \
  --customer="trial@example.com" \
  --expires="2026-02-03" \
  --features="trial"
```

**Expiration behavior:**

| Scenario | Result |
|----------|--------|
| License not expired | ✅ `valid: true`, app works normally |
| License expired | ❌ `valid: false`, error: "License has expired" |
| Expiring soon (≤30 days) | ⚠️ `isExpiringSoon$` emits `true` for UI warnings |

**Angular observables for expiration:**

```typescript
// In component
license.daysRemaining$    // Observable<number | null> - days until expiry
license.isExpiringSoon$   // Observable<boolean> - true if ≤30 days left
license.isExpired$        // Observable<boolean> - true if expired

// Example: Show warning banner
<div *ngIf="license.isExpiringSoon$ | async" class="warning">
  License expires in {{ license.daysRemaining$ | async }} days
</div>
```

**Date formats supported:**
- `YYYY-MM-DD` (e.g., "2027-12-31") - expires at 23:59:59 UTC
- RFC 3339 (e.g., "2027-12-31T23:59:59Z") - exact timestamp

### Phase 2: Domain Verification (TODO)

Domain verification ties licenses to specific customer server domains, preventing license sharing across organizations.

#### Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     DOMAIN VERIFICATION FLOW                             │
│                                                                          │
│  ┌──────────────┐     ┌──────────────┐     ┌──────────────────────────┐ │
│  │   License    │     │   WebView    │     │   Rust Domain Checker    │ │
│  │   (stored)   │     │   (Angular)  │     │   (navigation handler)   │ │
│  └──────┬───────┘     └──────┬───────┘     └──────────────┬───────────┘ │
│         │                    │                            │              │
│         │  domains: [        │                            │              │
│         │    "fleet.acme.com"│                            │              │
│         │    "localhost"     │                            │              │
│         │  ]                 │                            │              │
│         │                    │                            │              │
│         │                    │  1. Navigate to URL        │              │
│         │                    │─────────────────────────►  │              │
│         │                    │                            │              │
│         │  2. Get domains    │                            │              │
│         │◄────────────────────────────────────────────────│              │
│         │                    │                            │              │
│         │                    │  3. Check: URL host        │              │
│         │                    │     in domains[]?          │              │
│         │                    │                            │              │
│         │                    │  4a. YES → Allow           │              │
│         │                    │◄───────────────────────────│              │
│         │                    │                            │              │
│         │                    │  4b. NO → Block + Error    │              │
│         │                    │◄───────────────────────────│              │
│  └──────┴───────────────────┴────────────────────────────┴──────────────┘│
└─────────────────────────────────────────────────────────────────────────┘
```

#### Why Domain Verification?

| Scenario | Without Domain Lock | With Domain Lock |
|----------|---------------------|------------------|
| Customer A shares license with Customer B | ⚠️ Works (traceable but usable) | ❌ Blocked - wrong domain |
| Competitor copies license key | ⚠️ Works on their server | ❌ Blocked - domain mismatch |
| Employee takes license to new company | ⚠️ Could use on new servers | ❌ Blocked - unauthorized domain |
| Development/staging testing | ✅ Works | ✅ Works if `localhost` included |

#### Implementation Checklist

| Task | File(s) | Complexity | Description |
|------|---------|------------|-------------|
| 1. Extend LicenseInfo | `src-tauri/src/license.rs` | Low | Add `domains: Option<Vec<String>>` field |
| 2. Update license generator | `license-generator/src/main.rs` | Low | Add `--domains` CLI flag |
| 3. Add navigation handler | `src-tauri/src/lib.rs` | Medium | Intercept WebView navigation events |
| 4. Domain matching logic | `src-tauri/src/license.rs` | Medium | Implement hostname + wildcard matching |
| 5. Angular error handling | `src/app/services/license.service.ts` | Low | Handle domain rejection errors |
| 6. Error UI component | `src/app/components/` | Low | Show "Unauthorized domain" message |

#### 1. Extend LicenseInfo (Rust)

```rust
// src-tauri/src/license.rs

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LicenseInfo {
    pub customer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company: Option<String>,
    pub product: String,
    pub expires: String,
    pub features: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seats: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issued: Option<String>,
    pub version: u32,

    // NEW: Phase 2 - Domain verification
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domains: Option<Vec<String>>,
}

impl LicenseInfo {
    /// Check if a domain is authorized by this license
    ///
    /// Supports:
    /// - Exact match: "fleet.acme.com" matches "fleet.acme.com"
    /// - Wildcard: "*.acme.com" matches "fleet.acme.com", "staging.acme.com"
    /// - Localhost: "localhost" matches "localhost:4200", "127.0.0.1"
    pub fn is_domain_authorized(&self, url: &str) -> bool {
        // No domains specified = allow all (Phase 1 compatibility)
        let domains = match &self.domains {
            Some(d) if !d.is_empty() => d,
            _ => return true,
        };

        // Parse the URL to extract host
        let host = match url::Url::parse(url) {
            Ok(parsed) => parsed.host_str().unwrap_or("").to_lowercase(),
            Err(_) => return false,
        };

        for domain in domains {
            let domain = domain.to_lowercase();

            // Handle localhost special case
            if domain == "localhost" {
                if host == "localhost" || host == "127.0.0.1" || host == "::1" {
                    return true;
                }
            }
            // Handle wildcard domains (*.example.com)
            else if domain.starts_with("*.") {
                let suffix = &domain[1..]; // ".example.com"
                if host.ends_with(suffix) || host == &domain[2..] {
                    return true;
                }
            }
            // Exact match
            else if host == domain {
                return true;
            }
        }

        false
    }
}
```

#### 2. Update License Generator

```rust
// license-generator/src/main.rs

#[derive(Parser, Debug)]
struct Args {
    // ... existing fields ...

    /// Comma-separated list of authorized domains
    /// Examples: "fleet.acme.com,staging.acme.com,localhost"
    /// Use "*.acme.com" for wildcard subdomains
    #[arg(long)]
    domains: Option<String>,
}

// In generate_license():
let domains: Option<Vec<String>> = args.domains.map(|d| {
    d.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
});

let info = LicenseInfo {
    // ... existing fields ...
    domains,
};
```

**Usage:**

```bash
# Generate domain-locked license
cargo run -- \
  --private-key="YOUR_KEY" \
  --customer="john@acme.com" \
  --company="ACME Corp" \
  --expires="2027-12-31" \
  --features="premium,export" \
  --domains="fleet.acme.com,staging.acme.com,localhost"

# With wildcard subdomain
cargo run -- \
  --private-key="YOUR_KEY" \
  --customer="enterprise@bigcorp.com" \
  --expires="2027-12-31" \
  --domains="*.bigcorp.com,localhost"
```

#### 3. WebView Navigation Handler (Tauri)

```rust
// src-tauri/src/lib.rs

use tauri::Manager;
use tauri::webview::PageLoadEvent;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // Get the main window
            let window = app.get_webview_window("main").unwrap();

            // Listen for navigation events
            window.on_page_load(|webview, payload| {
                if let PageLoadEvent::Started = payload.event() {
                    let url = payload.url().to_string();

                    // Check domain authorization
                    if let Err(e) = check_domain_authorization(&url) {
                        // Block navigation and show error
                        webview.eval(&format!(
                            "window.__DOMAIN_ERROR__ = '{}'; \
                             window.dispatchEvent(new CustomEvent('domain-error', {{ detail: '{}' }}));",
                            e, e
                        )).ok();

                        // Optionally navigate to error page
                        webview.navigate("tauri://localhost/domain-error.html".parse().unwrap()).ok();
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // ... existing handlers ...
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn check_domain_authorization(url: &str) -> Result<(), String> {
    // Load stored license
    let app_data_dir = /* get app data dir */;
    let storage = LicenseStorage::new(app_data_dir);

    if !storage.exists() {
        return Err("No license found".to_string());
    }

    let license_key = storage.load()
        .map_err(|e| format!("Failed to load license: {}", e))?;

    let status = license::get_license_status(&license_key);

    if !status.valid {
        return Err(status.error.unwrap_or("Invalid license".to_string()));
    }

    if let Some(info) = &status.info {
        if !info.is_domain_authorized(url) {
            return Err(format!(
                "Domain not authorized. URL '{}' is not in licensed domains: {:?}",
                url,
                info.domains
            ));
        }
    }

    Ok(())
}
```

#### 4. Tauri v2 Navigation API

In Tauri v2, use the `on_navigation` event:

```rust
// src-tauri/src/lib.rs

use tauri::webview::WebviewBuilder;

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle().clone();

            // Create webview with navigation handler
            let webview = WebviewBuilder::new("main", tauri::WebviewUrl::App("index.html".into()))
                .on_navigation(move |url| {
                    // Return false to block navigation, true to allow
                    match check_domain_authorization_v2(&handle, url.as_str()) {
                        Ok(()) => true,  // Allow
                        Err(e) => {
                            eprintln!("Domain blocked: {}", e);
                            false  // Block
                        }
                    }
                })
                .build()?;

            Ok(())
        })
        .run(tauri::generate_context!())
}
```

#### 5. Angular Error Handling

```typescript
// src/app/services/license.service.ts

// Listen for domain errors from Tauri
if (typeof window !== 'undefined') {
  window.addEventListener('domain-error', (event: CustomEvent) => {
    this.domainErrorSubject.next(event.detail);
  });
}

// Observable for domain errors
private domainErrorSubject = new BehaviorSubject<string | null>(null);
public domainError$ = this.domainErrorSubject.asObservable();
```

#### 6. Error UI Component

```typescript
// src/app/components/domain-error/domain-error.component.ts

@Component({
  selector: 'app-domain-error',
  template: `
    <div class="domain-error-overlay" *ngIf="licenseService.domainError$ | async as error">
      <div class="domain-error-dialog">
        <h2>⚠️ Unauthorized Domain</h2>
        <p>{{ error }}</p>
        <p>This application is licensed for specific domains only.</p>
        <button (click)="openLicenseDialog()">Update License</button>
      </div>
    </div>
  `
})
export class DomainErrorComponent {
  constructor(public licenseService: LicenseService) {}
}
```

#### Domain Matching Rules

| License Domain | URL | Match? | Reason |
|----------------|-----|--------|--------|
| `fleet.acme.com` | `https://fleet.acme.com/bikes` | ✅ | Exact match |
| `fleet.acme.com` | `https://api.acme.com/v1` | ❌ | Different subdomain |
| `*.acme.com` | `https://fleet.acme.com/bikes` | ✅ | Wildcard matches |
| `*.acme.com` | `https://api.acme.com/v1` | ✅ | Wildcard matches |
| `*.acme.com` | `https://acme.com/home` | ✅ | Root domain included |
| `localhost` | `http://localhost:4200` | ✅ | Localhost special case |
| `localhost` | `http://127.0.0.1:4200` | ✅ | IPv4 localhost |
| `localhost` | `http://192.168.1.100` | ❌ | LAN IP ≠ localhost |

#### Backward Compatibility

- **Licenses without `domains` field**: Treated as "allow all" (Phase 1 behavior)
- **Empty `domains` array**: Same as no field (allow all)
- **At least one domain specified**: Domain checking enforced

#### Security Considerations

| Concern | Mitigation |
|---------|------------|
| User modifies stored license | License is signed; modification invalidates signature |
| User patches domain check in binary | Requires reverse engineering compiled Rust |
| User intercepts IPC calls | Domain check happens in Rust, not JS |
| DNS spoofing | Less relevant for desktop app; TLS validates certificates |

#### Testing Phase 2

```bash
# Generate test license with domains
cargo run -- \
  --private-key="YOUR_KEY" \
  --customer="test@example.com" \
  --expires="2027-12-31" \
  --domains="localhost,127.0.0.1"

# Expected behavior:
# - App loads from localhost:4200 → ✅ Works
# - App tries to fetch from api.external.com → ❌ Blocked
# - Navigate to unauthorized domain → ❌ Error shown
```

---

## Security Considerations

### What This Protects Against

| Attack | Protected? | Notes |
|--------|------------|-------|
| Casual piracy (share .exe) | ✅ Yes | No valid license key |
| License key sharing | ⚠️ Partial | Traceable, domain-locked in Phase 2 |
| Decompilation | ✅ Mostly | Verification in Rust binary is hard to patch |
| Clock manipulation | ⚠️ Partial | Offline expiry can be bypassed |

### What This Does NOT Protect Against

| Attack | Why Not | Mitigation |
|--------|---------|------------|
| Skilled reverse engineers | Binary patching is always possible | Legal protection, frequent updates |
| Key generators (keygens) | Would need private key | Keep private key SECRET |
| Memory inspection | Runtime values visible | VMProtect (Layer 4) |

### Private Key Security

**CRITICAL:** The Ed25519 private key must NEVER be:
- Committed to git
- Included in the app binary
- Shared with anyone
- Stored on developer machines without encryption

**Recommended:** Store in a hardware security module (HSM) or secrets manager (AWS KMS, HashiCorp Vault).

---

## Example License Flow

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   Customer   │     │  Your Server │     │  Desktop App │
└──────┬───────┘     └──────┬───────┘     └──────┬───────┘
       │                    │                    │
       │  1. Purchase       │                    │
       │───────────────────►│                    │
       │                    │                    │
       │  2. License Key    │                    │
       │◄───────────────────│                    │
       │                    │                    │
       │  3. Enter key in app                    │
       │────────────────────────────────────────►│
       │                    │                    │
       │                    │    4. Verify       │
       │                    │    (offline,       │
       │                    │     in Rust)       │
       │                    │                    │
       │  5. App unlocked   │                    │
       │◄────────────────────────────────────────│
       │                    │                    │
```

---

## Related Documentation

- [BUILD_SUMMARY.md](./BUILD_SUMMARY.md) - Overall build process and outputs
- [CODE_PROTECTION_STRATEGY.md](./CODE_PROTECTION_STRATEGY.md) - Protection philosophy
- [OBFUSCATION.md](./OBFUSCATION.md) - JavaScript obfuscation details
- [WASM_SETUP.md](./WASM_SETUP.md) - WebAssembly module setup
- [../TAURI_SETUP.md](../TAURI_SETUP.md) - Tauri desktop app setup

### On-Premise HA Deployment
- [ON_PREMISE_HA_SETUP.md](./ON_PREMISE_HA_SETUP.md) - Complete HA deployment guide
- [POSTGRESQL_HA_DEPLOYMENT.md](./POSTGRESQL_HA_DEPLOYMENT.md) - Patroni + etcd cluster setup
- [BACKUP_RECOVERY.md](./BACKUP_RECOVERY.md) - pgBackRest backup strategy
- [INFRASTRUCTURE_DECISIONS.md](./INFRASTRUCTURE_DECISIONS.md) - Architecture rationale

---

*Document created: January 2025*
