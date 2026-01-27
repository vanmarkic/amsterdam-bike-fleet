# WASM Module Setup Guide

This document explains how to build and use the WebAssembly (WASM) module for the Amsterdam Bike Fleet application. The WASM module provides protected client-side algorithms for fleet statistics, data validation, and geographic calculations.

## Prerequisites

### 1. Install Rust

If you don't have Rust installed:

```bash
# macOS/Linux
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Windows (download and run)
# https://win.rustup.rs/
```

After installation, restart your terminal and verify:

```bash
rustc --version
cargo --version
```

### 2. Install wasm-pack

wasm-pack is the tool that compiles Rust to WebAssembly:

```bash
# Using cargo (recommended)
cargo install wasm-pack

# Or using npm
npm install -g wasm-pack

# Or on macOS with Homebrew
brew install wasm-pack
```

Verify installation:

```bash
wasm-pack --version
```

### 3. Add WASM target to Rust

```bash
rustup target add wasm32-unknown-unknown
```

## Building the WASM Module

### Development Build

For development with debug symbols and faster compilation:

```bash
npm run wasm:build
```

This runs: `wasm-pack build --target bundler --out-dir pkg`

### Production Build

For optimized, smaller output:

```bash
npm run wasm:build:release
```

This runs: `wasm-pack build --target bundler --release --out-dir pkg`

### Web Target Build

For use without a bundler (direct browser loading):

```bash
npm run wasm:build:web
```

Output will be in `wasm-lib/pkg-web/`.

## Project Structure

```
amsterdam-bike-fleet/
├── wasm-lib/                    # WASM module source
│   ├── Cargo.toml               # Rust package configuration
│   ├── src/
│   │   └── lib.rs               # WASM functions implementation
│   └── pkg/                     # Generated output (after build)
│       ├── amsterdam_bike_fleet_wasm.js
│       ├── amsterdam_bike_fleet_wasm.d.ts
│       ├── amsterdam_bike_fleet_wasm_bg.wasm
│       └── package.json
├── src/
│   └── app/
│       └── services/
│           └── wasm.service.ts  # Angular service wrapping WASM
└── docs/
    └── WASM_SETUP.md            # This file
```

## Using WASM in Angular

### 1. Import the Service

```typescript
import { WasmService } from './services/wasm.service';
```

### 2. Initialize in Component

```typescript
import { Component, OnInit } from '@angular/core';
import { WasmService, FleetStatistics } from './services/wasm.service';

@Component({
  selector: 'app-fleet-stats',
  template: `...`
})
export class FleetStatsComponent implements OnInit {
  stats: FleetStatistics | null = null;

  constructor(
    private wasmService: WasmService,
    private fleetApi: FleetApiService
  ) {}

  async ngOnInit() {
    // Initialize WASM module (required once)
    await this.wasmService.initialize();

    // Now you can use WASM functions
    this.fleetApi.getFleetDataStream().subscribe(data => {
      this.stats = this.wasmService.calculateFleetStatistics(data.bikes);
    });
  }
}
```

### 3. Initialize in App Module (Alternative)

For app-wide initialization, use APP_INITIALIZER:

```typescript
import { APP_INITIALIZER, NgModule } from '@angular/core';
import { WasmService } from './services/wasm.service';

function initializeWasm(wasmService: WasmService) {
  return () => wasmService.initialize();
}

@NgModule({
  providers: [
    {
      provide: APP_INITIALIZER,
      useFactory: initializeWasm,
      deps: [WasmService],
      multi: true
    }
  ]
})
export class AppModule {}
```

## Available WASM Functions

The WASM module provides three categories of functions:

1. **Analysis** - Fleet statistics, validation, geographic calculations
2. **Simulation** - Movement physics, status transitions, speed modeling
3. **Optimization** - Fast hashing for change detection

---

### Fleet Statistics

Calculate comprehensive statistics for the bike fleet:

```typescript
const stats = wasmService.calculateFleetStatistics(bikes);
console.log(`Total bikes: ${stats.totalBikes}`);
console.log(`Active: ${stats.activePercentage.toFixed(1)}%`);
console.log(`Average speed: ${stats.averageSpeed.toFixed(1)} km/h`);
```

**Returns:**
- `totalBikes` - Total number of bikes
- `deliveringCount` - Bikes currently delivering
- `idleCount` - Bikes that are idle
- `returningCount` - Bikes returning
- `averageSpeed` - Mean speed in km/h
- `maxSpeed` / `minSpeed` - Speed range
- `activePercentage` - Percentage of active bikes
- `fleetCenterLongitude` / `fleetCenterLatitude` - Geographic centroid

### Data Validation

Validate individual bike data:

```typescript
const result = wasmService.validateBikeData(bike);
if (result.isValid) {
  console.log('Data is valid');
  // Use result.sanitizedData
} else {
  console.error('Errors:', result.errors);
  console.warn('Warnings:', result.warnings);
}
```

Batch validation:

```typescript
const results = wasmService.validateBikeDataBatch(bikes);
const invalidCount = results.filter(r => !r.isValid).length;
```

### Geographic Calculations

Calculate distance between two points:

```typescript
const from = { longitude: 4.9041, latitude: 52.3676 }; // Amsterdam Centraal
const to = { longitude: 4.8932, latitude: 52.3730 };   // Dam Square

const result = wasmService.calculateDistance(from, to);
console.log(`Distance: ${result.distanceKm.toFixed(2)} km`);
console.log(`Bearing: ${result.bearingDegrees.toFixed(1)}°`);
```

Find nearest bike:

```typescript
const target = { longitude: 4.9000, latitude: 52.3700 };
const nearest = wasmService.findNearestBike(bikes, target);
console.log(`Nearest bike: ${nearest.name}`);
```

Find bikes within radius:

```typescript
const center = { longitude: 4.9041, latitude: 52.3676 };
const radiusKm = 2.0;
const nearbyBikes = wasmService.findBikesInRadius(bikes, center, radiusKm);
console.log(`Found ${nearbyBikes.length} bikes within ${radiusKm}km`);
```

---

### Simulation Functions

These functions power the real-time fleet simulation, providing deterministic, reproducible behavior.

#### Complete Simulation Tick (Recommended)

The main entry point that combines all simulation steps in one call:

```typescript
const result = wasmService.simulationTick(bikes, Date.now(), 0.10);

// Result contains:
// - bikes: Updated bike positions
// - statistics: Fleet statistics
// - positionHash: For deck.gl updateTriggers
// - stateHash: Includes status and speed
// - statusTransitions: Count of bikes that changed status
// - boundsCorrections: Count of bikes clamped to bounds
```

This is the most efficient approach as it avoids multiple JS↔WASM boundary crossings.

#### Movement Simulation

Simulate bike movement for one tick with realistic physics:

```typescript
const result = wasmService.simulateBikeMovement(bikes, Date.now());
// Returns: { bikes, movementsApplied, boundCorrections }
```

- **Idle bikes**: Drift slightly (GPS jitter simulation)
- **Active bikes**: Move purposefully in random directions
- **Bounds enforcement**: Positions clamped to Amsterdam operational area

#### Status Transitions

Uses a Markov chain model for realistic status changes:

```typescript
const result = wasmService.transitionBikeStatus('delivering', Math.random());
// Returns: { newStatus, transitionOccurred, probabilityUsed }
```

Transition probabilities:
- **Delivering**: 70% stay → 15% returning → 15% idle
- **Returning**: 65% stay → 25% idle → 10% delivering
- **Idle**: 60% stay → 30% delivering → 10% returning

Batch processing for multiple bikes:

```typescript
const results = wasmService.transitionBikeStatusBatch(
  ['delivering', 'idle', 'returning'],
  [0.5, 0.8, 0.3]
);
```

#### Speed Calculation

Calculate speed based on status and traffic conditions:

```typescript
const result = wasmService.calculateBikeSpeed('delivering', false, 0.5);
// Returns: { speed, baseSpeed, trafficPenalty, statusFactor }
```

Speed ranges:
- **Delivering**: 15-35 km/h
- **Returning**: 10-25 km/h
- **Idle**: 0 km/h
- **Traffic penalty**: 40% reduction in congestion zones

Batch processing:

```typescript
const speeds = wasmService.calculateBikeSpeedBatch(
  ['delivering', 'idle'],
  [false, true],
  [0.5, 0.7]
);
```

---

### Optimization Functions

#### Position Hashing

Fast hash computation for deck.gl `updateTriggers`:

```typescript
const hash = wasmService.hashBikePositions(bikes);
// Use in deck.gl layer:
// updateTriggers: { getPosition: hash }
```

Uses FNV-1a algorithm for O(n) deterministic hashing.

#### State Hashing

More comprehensive hash including status and speed:

```typescript
const hash = wasmService.hashBikeState(bikes);
```

Use when you need to detect any state change, not just position

## TypeScript Configuration

Ensure your `tsconfig.json` allows dynamic imports:

```json
{
  "compilerOptions": {
    "module": "ES2022",
    "moduleResolution": "node",
    "target": "ES2022"
  }
}
```

## Troubleshooting

### "Failed to load WASM module"

1. Make sure you've built the WASM module:
   ```bash
   npm run wasm:build
   ```

2. Check that `wasm-lib/pkg/` exists and contains the generated files.

3. Verify the import path in `wasm.service.ts` matches your project structure.

### "wasm-pack: command not found"

Install wasm-pack:
```bash
cargo install wasm-pack
```

Or add it to your PATH if already installed.

### Build fails with "error[E0463]: can't find crate"

Add the WASM target:
```bash
rustup target add wasm32-unknown-unknown
```

### CORS errors when loading WASM

When running locally without a bundler, the WASM file must be served with proper MIME type. Use Angular's dev server (`ng serve`) which handles this automatically.

### Large bundle size

For production, ensure you're using the release build:
```bash
npm run wasm:build:release
```

The release build uses LTO (Link Time Optimization) and size optimization.

## Why WASM?

1. **Performance** - Rust/WASM can be significantly faster than JavaScript for computational tasks

2. **Code Protection** - WASM binaries are harder to reverse-engineer than JavaScript

3. **Type Safety** - Rust's strong type system catches errors at compile time

4. **Memory Safety** - Rust prevents common memory-related bugs

5. **Consistency** - Same algorithms can run in browser and in Tauri desktop app

## Testing

Run Rust tests:

```bash
cd wasm-lib
cargo test
```

Run WASM tests in browser:

```bash
npm run wasm:test
```

## Extending the Module

To add new WASM functions:

1. Add the function to `wasm-lib/src/lib.rs`:
   ```rust
   #[wasm_bindgen(js_name = myNewFunction)]
   pub fn my_new_function(input: JsValue) -> Result<JsValue, JsValue> {
       // Implementation
   }
   ```

2. Rebuild the WASM module:
   ```bash
   npm run wasm:build
   ```

3. Add TypeScript wrapper in `wasm.service.ts`:
   ```typescript
   myNewFunction(input: InputType): OutputType {
     this.ensureInitialized();
     return this.wasmModule!.myNewFunction(input);
   }
   ```

4. Update the type declaration in `pkg/amsterdam_bike_fleet_wasm.d.ts`.
