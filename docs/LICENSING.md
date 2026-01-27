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
│  └── SQLite database operations                             │
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

- [ ] Extend `LicenseInfo` to include `domains: Vec<String>`
- [ ] Add WebView URL interception in Rust
- [ ] Verify URL host against licensed domains
- [ ] Block unauthorized origins with error UI
- [ ] Update license generator to include domains

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

---

*Document created: January 2025*
