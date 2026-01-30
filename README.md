# Amsterdam Bike Fleet

Real-time bike courier fleet management visualization for Amsterdam, built with Angular 15, deck.gl, and Rust/WebAssembly.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         PRESENTATION                             │
│  Angular 15 + deck.gl + MapLibre GL                              │
│  • Fleet map visualization                                       │
│  • Real-time position updates                                    │
│  • Pollution/traffic zone overlays                               │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      BUSINESS LOGIC                              │
│  Rust → WebAssembly (wasm-lib/)                                  │
│  • Fleet simulation (movement, status, speed)                    │
│  • Geographic calculations (Haversine)                           │
│  • Statistics aggregation                                        │
│  • Fast hashing for change detection                             │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                       DESKTOP SHELL                              │
│  Tauri (src-tauri/)                                              │
│  • Native window management                                      │
│  • License verification                                          │
│  • System integration                                            │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                       DATABASE BACKEND                           │
│  SQLite (default) OR PostgreSQL (--features postgres)            │
│  ┌─────────────────┐  ┌─────────────────────────────────────┐   │
│  │ SQLite          │  │ PostgreSQL HA (On-Premise)          │   │
│  │ • Standalone    │  │ • Patroni + etcd (auto-failover)    │   │
│  │ • Embedded      │  │ • HAProxy (load balancing)          │   │
│  │ • Zero config   │  │ • pgBackRest (backups)              │   │
│  └─────────────────┘  │ • 99.99% uptime target              │   │
│                       └─────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

### Why This Architecture?

**Performance**: Rust/WASM executes compute-intensive simulation 10-100x faster than equivalent JavaScript.

**Code Protection**: Compiled WASM binaries are significantly harder to reverse-engineer than JavaScript source.

**Graceful Degradation**: TypeScript fallbacks ensure the app works even if WASM fails to load.

**Determinism**: Seeded pseudo-random simulation produces reproducible results for testing and debugging.

## Quick Start

```bash
# Install dependencies
npm install

# Build WASM module (requires Rust + wasm-pack)
npm run wasm:build

# Start development server
ng serve
```

Open http://localhost:4200 to see the fleet map.

## Project Structure

```
amsterdam-bike-fleet/
├── src/app/
│   ├── components/
│   │   ├── fleet-map/        # Main map visualization
│   │   └── bike-list-item/   # Bike status display
│   └── services/
│       ├── fleet-api.service.ts   # Data stream + simulation
│       └── wasm.service.ts        # WASM module wrapper
├── wasm-lib/                 # Rust WASM source
│   ├── src/lib.rs            # Core algorithms
│   └── pkg/                  # Compiled WASM output
├── src-tauri/                # Tauri desktop shell
└── docs/                     # Architecture documentation
```

## Key Technologies

| Layer | Technology | Purpose |
|-------|------------|---------|
| Map | MapLibre GL + deck.gl | Hardware-accelerated visualization |
| UI | Angular 15 | Component architecture, change detection |
| Logic | Rust → WASM | Protected, performant algorithms |
| Desktop | Tauri | Native distribution, licensing |
| Database | SQLite / PostgreSQL | Embedded (default) or HA on-premise |

## Documentation

### Core Documentation
- [WASM Setup Guide](docs/WASM_SETUP.md) - Building and using the WebAssembly module
- [Code Protection Strategy](docs/CODE_PROTECTION_STRATEGY.md) - Security architecture rationale
- [Licensing](docs/LICENSING.md) - License key implementation
- [Performance](docs/PERFORMANCE_OPTIMIZATIONS.md) - Optimization techniques

### On-Premise High Availability
- [On-Premise HA Setup](docs/ON_PREMISE_HA_SETUP.md) - Complete on-premise deployment guide
- [PostgreSQL HA Deployment](docs/POSTGRESQL_HA_DEPLOYMENT.md) - Patroni + etcd cluster setup
- [Backup & Recovery](docs/BACKUP_RECOVERY.md) - pgBackRest backup strategy
- [Infrastructure Decisions](docs/INFRASTRUCTURE_DECISIONS.md) - Why not Kubernetes, architecture rationale

## Development

### Prerequisites

- Node.js 18+
- Rust toolchain (`rustup`)
- wasm-pack (`cargo install wasm-pack`)

### Commands

```bash
# Angular
ng serve              # Development server
ng build              # Production build
ng test               # Unit tests

# WASM
npm run wasm:build    # Development build
npm run wasm:build:release  # Optimized build
cd wasm-lib && cargo test   # Rust tests

# Tauri (SQLite - default)
npm run tauri dev     # Desktop development
npm run tauri build   # Desktop release

# Tauri (PostgreSQL - for on-premise HA)
cd src-tauri
cargo build --release --no-default-features --features postgres
```

## On-Premise HA Deployment

For enterprise deployments requiring **99.99% uptime**, the app supports PostgreSQL with automatic failover.

### Quick Start

```bash
# 1. Build with PostgreSQL support
cd src-tauri
cargo build --release --no-default-features --features postgres

# 2. Configure environment (point to HAProxy VIP)
export PG_HOST=10.0.0.100    # HAProxy VIP
export PG_PORT=5432
export PG_USER=fleet_app
export PG_PASSWORD=your_password
export PG_DATABASE=bike_fleet

# 3. Run the app
./amsterdam-bike-fleet
```

### Infrastructure Components

| Component | Purpose | Nodes |
|-----------|---------|-------|
| PostgreSQL + Patroni | Database with auto-failover | 3 |
| etcd | Distributed consensus | 3 |
| HAProxy | Load balancing + routing | 2 |
| pgBackRest | Backup & recovery | 1 |

See [On-Premise HA Setup](docs/ON_PREMISE_HA_SETUP.md) for complete deployment guide.

## Design Decisions

### Simulation in WASM

The fleet simulation (movement physics, status transitions, speed calculation) runs in Rust/WASM rather than TypeScript because:

1. **Determinism** - Seeded random generation ensures reproducible behavior
2. **Performance** - Single `simulationTick()` call processes all bikes efficiently
3. **Protection** - Core algorithms are compiled, not exposed as readable source

### Graceful Fallback Pattern

Every WASM function has a TypeScript fallback:

```typescript
if (this.wasmInitialized) {
  return this.wasmService.calculateFleetStatistics(bikes);
} else {
  // TypeScript implementation
  return { totalBikes: bikes.length, ... };
}
```

This ensures the application works even in environments where WASM is unavailable.

### FNV-1a Hashing

deck.gl's `updateTriggers` require detecting when data changes. Rather than deep comparison, we use fast O(n) hashing:

```typescript
updateTriggers: {
  getPosition: this.hashBikePositions(bikes)
}
```

The hash changes when any bike moves, triggering a re-render.
