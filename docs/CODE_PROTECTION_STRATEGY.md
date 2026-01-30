# Code Protection Strategy for Amsterdam Bike Fleet

## Executive Summary

This document outlines the recommended approach for transforming the Amsterdam Bike Fleet Angular web application into a difficult-to-reverse-engineer Windows executable. The key insight is that **architecture matters more than framework choice** — splitting code between a protected backend (Rust/WASM) and a "dumb" UI layer provides far stronger protection than any GUI framework or obfuscation tool alone.

---

## The Myth of GUI Framework Protection

### Why Qt, WinUI, or Any Native GUI Framework Won't Protect Your Code

A common misconception is that choosing a "native" GUI framework (Qt, WinUI, GTK, etc.) provides inherent code protection. **This is false.**

| Framework | Compilation Target | Reverse Engineering Difficulty |
|-----------|-------------------|-------------------------------|
| Qt (C++) | Native x86/x64 | ⭐⭐⭐ Medium - IDA Pro, Ghidra can decompile |
| WinUI (C#) | IL/.NET bytecode | ⭐⭐ Easy - dnSpy, ILSpy decompile to near-source |
| Electron (JS) | Plain JavaScript | ⭐ Trivial - ASAR extract, source readable |
| Flutter (Dart) | Dart AOT | ⭐⭐⭐ Medium - Dart decompilers exist |
| Tauri (JS+Rust) | JS + Native | ⭐⭐⭐⭐⭐ Hard - if logic is in Rust |

**Key Insight:** The GUI framework only determines how the UI is rendered. The protection level depends entirely on **where your business logic lives** and **how it's compiled**.

### The Problem with "Native = Secure" Thinking

```
❌ WRONG ASSUMPTION:
   "If I rewrite in Qt/C++, my code is protected"

   Reality: C++ compiles to machine code, but:
   - Function signatures remain visible
   - String literals are extractable
   - Control flow is recoverable with tools like IDA Pro
   - A skilled reverser can reconstruct logic in days/weeks
```

```
✅ CORRECT APPROACH:
   "I'll minimize what's in the client and protect what remains"

   Strategy:
   - Move sensitive logic to compiled Rust backend
   - Compile remaining algorithms to WASM
   - UI becomes a "dumb renderer" with nothing to steal
```

---

## The Right Architecture: Code Splitting

### Protection Through Separation

The strongest protection comes from **architectural decisions**, not tool choices:

```
┌─────────────────────────────────────────────────────────────────┐
│                    PROTECTION ARCHITECTURE                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   ┌─────────────────────────────────────────────────────────┐   │
│   │                    LAYER 1: UI (Angular)                 │   │
│   │                                                          │   │
│   │   Contains:                    Protection:               │   │
│   │   • Component templates        • Obfuscation (JScrambler)│   │
│   │   • Styling/CSS                • Minification            │   │
│   │   • User event handlers        • Dead code elimination   │   │
│   │   • Display logic only                                   │   │
│   │                                                          │   │
│   │   Value if stolen: LOW (it's just UI scaffolding)        │   │
│   └─────────────────────────────────────────────────────────┘   │
│                              │                                   │
│                              ▼                                   │
│   ┌─────────────────────────────────────────────────────────┐   │
│   │              LAYER 2: WASM BRIDGE (Rust → WASM)          │   │
│   │                                                          │   │
│   │   Contains:                    Protection:               │   │
│   │   • Data transformations       • Compiled to WASM binary │   │
│   │   • Calculations               • No source maps          │   │
│   │   • Validation algorithms      • Stripped symbols        │   │
│   │   • Client-side processing                               │   │
│   │                                                          │   │
│   │   Reverse difficulty: HIGH (binary format, no reflection)│   │
│   └─────────────────────────────────────────────────────────┘   │
│                              │                                   │
│                              ▼                                   │
│   ┌─────────────────────────────────────────────────────────┐   │
│   │              LAYER 3: RUST BACKEND (Compiled)            │   │
│   │                                                          │   │
│   │   Contains:                    Protection:               │   │
│   │   • ALL business logic         • Native machine code     │   │
│   │   • Database operations        • Aggressive optimizations│   │
│   │   • API handlers               • No runtime/reflection   │   │
│   │   • Licensing/DRM              • Optional: VMProtect     │   │
│   │   • Secrets/API keys                                     │   │
│   │                                                          │   │
│   │   Reverse difficulty: EXTREME                            │   │
│   └─────────────────────────────────────────────────────────┘   │
│                              │                                   │
│                              ▼                                   │
│   ┌─────────────────────────────────────────────────────────┐   │
│   │              LAYER 4: DATABASE (SQLite or PostgreSQL)    │   │
│   │                                                          │   │
│   │   SQLite (default - standalone desktop):                 │   │
│   │   • SQLCipher encryption                                 │   │
│   │   • Key compiled into Rust binary                        │   │
│   │   • Schema hidden in compiled code                       │   │
│   │                                                          │   │
│   │   PostgreSQL (--features postgres - on-premise HA):      │   │
│   │   • Patroni + etcd for automatic failover                │   │
│   │   • Connection pooling via deadpool-postgres             │   │
│   │   • 99.99% uptime with 3-node cluster                    │   │
│   └─────────────────────────────────────────────────────────┘   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Why This Works

| Attack Vector | Traditional App | Our Architecture |
|--------------|-----------------|------------------|
| Extract source code | ✅ Full source available | ❌ Only UI templates |
| Decompile algorithms | ✅ Readable logic | ❌ Compiled WASM/Rust |
| Extract database schema | ✅ Visible in code | ❌ In Rust binary |
| Steal API keys | ✅ In JS bundle | ❌ In Rust binary |
| Clone business logic | ✅ Copy-paste | ❌ Must reverse-engineer binary |

---

## Recommended Technology Stack

### Tauri Framework

We recommend **Tauri** over Electron for several reasons:

| Aspect | Electron | Tauri |
|--------|----------|-------|
| Bundle size | ~150MB | ~5-10MB |
| Memory usage | High (Chromium) | Low (System WebView) |
| Backend language | Node.js (JavaScript) | Rust (compiled) |
| Code protection | Poor | Excellent (Rust portions) |
| Startup time | Slow | Fast |

### Database: SQLite or PostgreSQL

**SQLite + SQLCipher (default - standalone desktop):**
- **Embedded**: No external database server required
- **Encrypted**: SQLCipher provides AES-256 encryption
- **Fast**: Single-file database with excellent performance
- **Portable**: Database file stored in user's AppData

**PostgreSQL (--features postgres - on-premise HA deployments):**
- **High Availability**: Patroni + etcd for automatic failover (99.99% uptime)
- **Connection Pooling**: deadpool-postgres for efficient connections
- **Backup**: pgBackRest for point-in-time recovery
- **Scaling**: Suitable for multi-user enterprise deployments

Build with: `cargo build --release --no-default-features --features postgres`

### Frontend Protection: Multi-Layer

1. **javascript-obfuscator**: Open-source, configurable obfuscation
2. **WASM compilation**: Critical algorithms compiled to WebAssembly
3. **Code splitting**: Sensitive code never reaches the frontend

---

## Implementation Plan

### Phase 1: Tauri Setup
- Initialize Tauri project structure
- Configure Rust backend with Axum/SQLite
- Set up Tauri commands for IPC
- Integrate existing Angular frontend

### Phase 2: WASM Bridge
- Set up wasm-pack toolchain
- Identify algorithms to move to WASM
- Create Rust library for WASM compilation
- Integrate WASM modules into Angular

### Phase 3: JavaScript Obfuscation
- Configure javascript-obfuscator
- Integrate with Angular build pipeline
- Set appropriate obfuscation levels
- Test performance impact

### Phase 4: Backend Migration
- Move FleetApiService logic to Rust
- Move DeliveryService logic to Rust
- Move IssueService logic to Rust
- Implement SQLite data layer

### Phase 5: Final Hardening
- Apply VMProtect/Themida (optional)
- Code signing for distribution
- Installer creation with NSIS/WiX

---

## Security Comparison Matrix

### Before (Angular Web App)

```
┌─────────────────────────────────────────┐
│           ANGULAR WEB APP               │
│                                         │
│  • All code visible in browser DevTools │
│  • Business logic in JavaScript         │
│  • API keys in environment files        │
│  • Algorithms easily copied             │
│  • Zero protection                      │
│                                         │
│  Reverse Engineering Time: ~1 hour      │
└─────────────────────────────────────────┘
```

### After (Tauri + Rust + WASM)

```
┌─────────────────────────────────────────┐
│        TAURI PROTECTED APP              │
│                                         │
│  • UI code obfuscated (low value)       │
│  • Business logic in compiled Rust      │
│  • API keys in Rust binary              │
│  • Algorithms in WASM/Rust              │
│  • Database encrypted                   │
│                                         │
│  Reverse Engineering Time: Weeks/Months │
│  (if even possible)                     │
└─────────────────────────────────────────┘
```

---

## Cost Analysis

| Item | One-Time Cost | Monthly Cost | Notes |
|------|--------------|--------------|-------|
| Tauri | Free | Free | Open source |
| Rust toolchain | Free | Free | Open source |
| wasm-pack | Free | Free | Open source |
| javascript-obfuscator | Free | Free | Open source |
| SQLite + SQLCipher | Free | Free | Open source |
| VMProtect (optional) | ~$200 | - | Adds anti-debug/VM |
| JScrambler (optional) | - | ~$500+ | Enterprise obfuscation |
| Code signing cert | ~$200/year | - | Required for distribution |

**Minimum viable cost: $0** (using all open-source tools)
**Recommended cost: ~$400** (VMProtect + code signing)

---

## Conclusion

**The key takeaway is this:** Choosing Qt, WinUI, or any other GUI framework does not inherently protect your code. What protects your code is:

1. **Moving sensitive logic out of the frontend** into compiled Rust
2. **Compiling algorithms to WASM** for client-side operations that must remain local
3. **Encrypting data at rest** with SQLCipher
4. **Obfuscating remaining UI code** to raise the barrier

The Tauri framework enables this architecture elegantly by providing:
- A Rust backend (compiled, hard to reverse)
- A lightweight WebView frontend (keeps your Angular investment)
- Excellent IPC for communication between layers
- Small bundle sizes and fast startup

This approach provides commercial-grade protection at minimal cost while preserving your existing Angular codebase.

---

## Next Steps

1. Review this document and confirm approach
2. Set up development environment (Rust, wasm-pack, Tauri CLI)
3. Begin Phase 1: Tauri project initialization
4. Migrate services incrementally to Rust backend

---

*Document prepared for the Amsterdam Bike Fleet project*
*Date: January 2025*
