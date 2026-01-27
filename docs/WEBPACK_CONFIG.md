# Webpack Configuration Documentation

This document explains the custom webpack configuration used in the Amsterdam Bike Fleet application.

## Why Custom Webpack?

Angular's default webpack configuration doesn't support:
1. **WebAssembly modules** compiled with wasm-pack (Rust → WASM)
2. **JavaScript obfuscation** for intellectual property protection

We use `@angular-builders/custom-webpack` to extend Angular's webpack config while preserving all default build behavior.

---

## Problem 1: WASM Integration

### Background

wasm-pack generates WASM modules in two main formats:

| Target | How it works | Pros | Cons |
|--------|--------------|------|------|
| `bundler` | Uses ESM imports like `import * as wasm from "./file.wasm"` | Cleaner imports | Requires webpack experiments enabled |
| `web` | Loads WASM via `fetch()` at runtime | More compatible | WASM must be served as static asset |

### The Problem

Angular 15 uses webpack 5, which CAN handle WASM imports, but the experimental flags aren't enabled by default. When we tried using the `bundler` target, we got:

```
Module parse failed: Internal failure: parseVec could not cast the value
```

### Our Solution

We chose the **web target** for reliability:

```javascript
// webpack.config.js
config.experiments = {
  asyncWebAssembly: true,  // Allow dynamic import() of .wasm
  syncWebAssembly: true    // Allow synchronous WASM instantiation
};

config.module.rules.push({
  test: /\.wasm$/,
  type: 'webassembly/async'
});

config.resolve.alias = {
  '@wasm': path.resolve(__dirname, 'wasm-lib/pkg-web')
};
```

The WASM binary is copied to `/assets/wasm/` via angular.json:

```json
{
  "glob": "*.wasm",
  "input": "wasm-lib/pkg-web",
  "output": "/assets/wasm"
}
```

The WasmService loads it like this:

```typescript
const wasm = await import('@wasm/amsterdam_bike_fleet_wasm');
await wasm.default('/assets/wasm/amsterdam_bike_fleet_wasm_bg.wasm');
```

---

## Problem 2: Code Obfuscation

### Background

For commercial software, we want to protect business logic from easy reverse engineering. JavaScript obfuscation transforms readable code into functionally equivalent but hard-to-understand code.

### The Problem

Obfuscating everything breaks the app:
- **Zone.js** (in polyfills) does monkey-patching that breaks when renamed
- **Webpack runtime** handles dynamic imports - if obfuscated, lazy loading breaks
- **Third-party code** is already minified - obfuscating wastes build time

### Our Solution

1. **Production only** - Development builds skip obfuscation
2. **Vendor chunk splitting** - Separate node_modules from our code
3. **Selective exclusions** - Skip files that would break or provide no benefit

```javascript
if (isProduction && WebpackObfuscator && obfuscatorConfig) {
  // Split vendor code into separate chunk
  config.optimization.splitChunks.cacheGroups.vendor = {
    test: /[\\/]node_modules[\\/]/,
    name: 'vendor',
    chunks: 'all',
    priority: 10
  };

  // Apply obfuscation with exclusions
  config.plugins.push(new WebpackObfuscator(obfuscatorConfig, [
    '**/node_modules/**',    // Not our code
    '**/*.wasm',             // Binary already protected
    'polyfills.*.js',        // Zone.js breaks
    'vendor.*.js',           // Third-party, already minified
    'runtime.*.js',          // Webpack bootstrap
    // ... more exclusions
  ]));

  // Disable source maps (they defeat obfuscation)
  config.devtool = false;
}
```

---

## Architecture Decision: Web Target vs Bundler Target

We use wasm-pack's **web** target instead of **bundler** because:

| Factor | Web Target | Bundler Target |
|--------|------------|----------------|
| **Compatibility** | Works without complex webpack config | Requires experimental flags |
| **Caching** | Browser caches .wasm separately | Bundled into JS chunk |
| **Debugging** | Easy to verify in Network tab | Harder to inspect |
| **Tauri** | Same behavior in browser and desktop | Same |

**Trade-off**: Slightly more code in WasmService to handle initialization.

---

## Files That Work Together

These files must be kept in sync:

| File | Purpose |
|------|---------|
| `webpack.config.js` | Webpack customization (experiments, aliases, plugins) |
| `angular.json` | References webpack config, copies .wasm to assets |
| `tsconfig.json` | Defines `@wasm` path alias for TypeScript |
| `obfuscator.config.js` | Obfuscation settings (what to rename, etc.) |
| `src/app/services/wasm.service.ts` | Loads and initializes WASM module |
| `wasm-lib/pkg-web/` | wasm-pack output (web target) |

---

## Exclusion Patterns Explained

Each exclusion in the obfuscator has a specific reason:

### node_modules
```javascript
'**/node_modules/**'
```
Not our code. Already minified. Just wastes build time.

### WASM Files
```javascript
'**/*.wasm', '**/pkg/**', '**/pkg-web/**'
```
Binary format already protects IP. The JS wrapper is minimal and obfuscation can break wasm-bindgen glue code.

### Polyfills / Zone.js
```javascript
'polyfills.*.js', '**/zone*.js'
```
Zone.js monkey-patches browser APIs (setTimeout, Promise, etc.). Obfuscation renames internal variables that Zone.js expects to find, breaking Angular's change detection.

### Vendor Chunk
```javascript
'vendor.*.js'
```
Third-party code from node_modules. Not our IP to protect. Obfuscating adds ~30 seconds to build with no benefit.

### Runtime Chunk
```javascript
'runtime.*.js'
```
Webpack's module loading bootstrap. If obfuscated, dynamic imports (`loadComponent`) fail.

### Styles
```javascript
'styles.*.js', '**/*.css'
```
CSS extraction creates JS stubs with no meaningful code to protect.

### Large Libraries
```javascript
'**/deck.gl/**', '**/maplibre-gl/**'
```
Already heavily minified by their authors. Obfuscation provides minimal additional protection but significantly slows the build.

---

## Build Commands

| Command | Description |
|---------|-------------|
| `npm run build` | Production build with obfuscation |
| `npm start` | Development build (no obfuscation) |
| `npm run tauri:build` | Desktop app with obfuscated frontend |
| `npm run build:protected` | Build with obfuscation (explicit) |

---

## Troubleshooting

### "Module parse failed" for WASM
Ensure `experiments.asyncWebAssembly` is enabled and the `.wasm` rule is added.

### App breaks after obfuscation
Check if a critical file is being obfuscated. Add it to the exclusion list.

### Obfuscation not running
- Verify `config.mode === 'production'`
- Check that `webpack-obfuscator` is installed
- Check that `obfuscator.config.js` exists

### Dynamic imports fail
Ensure `runtime.*.js` is excluded from obfuscation.

### Angular change detection broken
Ensure `polyfills.*.js` and `zone*.js` are excluded.
