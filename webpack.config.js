/**
 * Custom Webpack Configuration for Angular 15 + WebAssembly + Code Obfuscation
 *
 * ============================================================================
 * WHY THIS FILE EXISTS
 * ============================================================================
 *
 * Angular's default webpack configuration doesn't support:
 * 1. WebAssembly modules compiled with wasm-pack (Rust → WASM)
 * 2. JavaScript obfuscation for intellectual property protection
 *
 * This file extends Angular's webpack config using @angular-builders/custom-webpack
 * to add these capabilities while preserving all default Angular build behavior.
 *
 * ============================================================================
 * PROBLEM 1: WASM INTEGRATION
 * ============================================================================
 *
 * wasm-pack generates WASM modules in two main formats:
 *
 * 1. "bundler" target: Uses ESM imports like `import * as wasm from "./file.wasm"`
 *    - Requires webpack to understand .wasm imports
 *    - Angular 15's webpack 5 CAN do this, but needs `experiments` enabled
 *    - ISSUE: Angular's default config doesn't enable these experiments
 *
 * 2. "web" target: Loads WASM via fetch() at runtime
 *    - More compatible, works without special webpack config
 *    - WASM binary must be served as a static asset
 *    - We chose this approach for reliability
 *
 * SOLUTION:
 * - Enable webpack 5's `asyncWebAssembly` and `syncWebAssembly` experiments
 * - Add a module rule to handle .wasm files
 * - Create a path alias `@wasm` pointing to the wasm-pack output directory
 * - Copy .wasm files to /assets/wasm/ via angular.json assets config
 *
 * The WasmService then loads WASM like this:
 *   const wasm = await import('@wasm/amsterdam_bike_fleet_wasm');
 *   await wasm.default('/assets/wasm/amsterdam_bike_fleet_wasm_bg.wasm');
 *
 * ============================================================================
 * PROBLEM 2: CODE OBFUSCATION
 * ============================================================================
 *
 * For commercial software, we want to protect business logic from easy reverse
 * engineering. JavaScript obfuscation makes code harder to read/copy.
 *
 * CHALLENGES:
 * - Obfuscating everything breaks the app (Angular internals, Zone.js, etc.)
 * - Obfuscation is slow - shouldn't run in development
 * - Source maps defeat obfuscation - must disable them
 *
 * SOLUTION:
 * - Only run obfuscation in production mode
 * - Split vendor code into separate chunk (not obfuscated)
 * - Exclude critical files that break when obfuscated:
 *   - polyfills.js (Zone.js monkey-patching breaks)
 *   - runtime.js (webpack bootstrap code)
 *   - vendor.js (node_modules - not our code anyway)
 *   - WASM-related JS (binary format already protects it)
 *   - Large libraries (deck.gl, maplibre) - already minified, just slows build
 *
 * ============================================================================
 * ARCHITECTURE DECISION: WEB TARGET VS BUNDLER TARGET
 * ============================================================================
 *
 * We use wasm-pack's "web" target instead of "bundler" because:
 *
 * 1. COMPATIBILITY: Works with Angular's build system without complex config
 * 2. CACHING: Browser can cache the .wasm file separately from JS bundles
 * 3. DEBUGGING: Easier to verify WASM is loading correctly (network tab)
 * 4. TAURI: Works the same in both browser dev mode and Tauri desktop app
 *
 * Trade-off: Slightly more code in WasmService to handle initialization
 *
 * ============================================================================
 * FILES THAT WORK TOGETHER
 * ============================================================================
 *
 * - webpack.config.js (this file) - Webpack customization
 * - angular.json - References this file, copies .wasm to assets
 * - tsconfig.json - Defines @wasm path alias for TypeScript
 * - obfuscator.config.js - Obfuscation settings (what to rename, etc.)
 * - src/app/services/wasm.service.ts - Loads and uses WASM module
 * - wasm-lib/pkg-web/ - wasm-pack output (web target)
 *
 * ============================================================================
 */

const path = require('path');

// ============================================================================
// CONDITIONAL OBFUSCATOR IMPORT
// ============================================================================
// Obfuscator is a devDependency and may not be installed in all environments
// (e.g., CI with --production flag). Gracefully handle missing dependency.

let WebpackObfuscator;
let obfuscatorConfig;
try {
  WebpackObfuscator = require('webpack-obfuscator');
  obfuscatorConfig = require('./obfuscator.config');
} catch (e) {
  // Obfuscator not available - will skip obfuscation
  // This is fine for development builds or minimal installs
}

/**
 * Webpack configuration function
 *
 * Angular's custom-webpack builder calls this function with the default
 * webpack config. We modify and return it.
 *
 * @param {import('webpack').Configuration} config - Angular's default webpack config
 * @param {object} _options - Build options (target, configuration name, etc.)
 * @returns {import('webpack').Configuration} Modified webpack config
 */
module.exports = (config, _options) => {
  // ==========================================================================
  // SECTION 1: WEBASSEMBLY SUPPORT
  // ==========================================================================
  // Webpack 5 has built-in WASM support but it's behind experimental flags.
  // Angular doesn't enable these by default.

  config.experiments = {
    ...config.experiments,
    // asyncWebAssembly: Allow dynamic import() of .wasm files
    // This is the modern, recommended approach
    asyncWebAssembly: true,
    // syncWebAssembly: Allow synchronous WASM instantiation
    // Needed for some wasm-pack output patterns
    syncWebAssembly: true
  };

  // Add explicit rule for .wasm files
  // Without this, webpack may try to parse WASM as JavaScript and fail
  config.module.rules.push({
    test: /\.wasm$/,
    type: 'webassembly/async'
  });

  // ==========================================================================
  // SECTION 2: PATH ALIAS FOR WASM MODULE
  // ==========================================================================
  // Create @wasm alias so TypeScript imports like:
  //   import('@wasm/amsterdam_bike_fleet_wasm')
  // resolve to the wasm-pack output directory.
  //
  // We use pkg-web (web target) not pkg (bundler target) because:
  // - Web target uses fetch() to load WASM - more compatible
  // - Bundler target requires direct .wasm imports - harder to configure

  config.resolve = config.resolve || {};
  config.resolve.alias = {
    ...config.resolve.alias,
    '@wasm': path.resolve(__dirname, 'wasm-lib/pkg-web')
  };

  // ==========================================================================
  // SECTION 3: JAVASCRIPT OBFUSCATION (PRODUCTION ONLY)
  // ==========================================================================

  const isProduction = config.mode === 'production';

  if (isProduction && WebpackObfuscator && obfuscatorConfig) {
    console.log('\n🔒 Applying JavaScript obfuscation for production build...\n');

    // -------------------------------------------------------------------------
    // 3a. VENDOR CHUNK SPLITTING
    // -------------------------------------------------------------------------
    // Separate node_modules code into its own chunk so we can exclude it
    // from obfuscation. Third-party code is already minified and obfuscating
    // it just wastes build time.

    config.optimization = config.optimization || {};
    config.optimization.splitChunks = {
      ...config.optimization.splitChunks,
      cacheGroups: {
        ...((config.optimization.splitChunks || {}).cacheGroups || {}),
        vendor: {
          test: /[\\/]node_modules[\\/]/,
          name: 'vendor',
          chunks: 'all',
          priority: 10
        }
      }
    };

    // -------------------------------------------------------------------------
    // 3b. OBFUSCATOR PLUGIN
    // -------------------------------------------------------------------------
    // Apply obfuscation to bundled JavaScript, EXCEPT files that would break
    // or provide no benefit from obfuscation.

    config.plugins.push(
      new WebpackObfuscator(
        obfuscatorConfig, // Settings from obfuscator.config.js
        // EXCLUSION PATTERNS - Files to skip obfuscation
        [
          // -----------------------------------------------------------------
          // NODE_MODULES: Not our code, already minified, just slows build
          // -----------------------------------------------------------------
          '**/node_modules/**',
          'node_modules/**',

          // -----------------------------------------------------------------
          // WASM FILES: Binary format already protects IP, JS wrapper is
          // minimal and obfuscation can break wasm-bindgen glue code
          // -----------------------------------------------------------------
          '**/*.wasm',
          '**/wasm-lib/**',
          '**/pkg/**',
          '**/pkg-web/**',

          // -----------------------------------------------------------------
          // SOURCE MAPS: Should be disabled anyway, but belt and suspenders
          // -----------------------------------------------------------------
          '**/*.map',

          // -----------------------------------------------------------------
          // POLYFILLS: Contains Zone.js which does tricky monkey-patching
          // of browser APIs. Obfuscation can rename things it patches and
          // break Angular's change detection entirely.
          // -----------------------------------------------------------------
          'polyfills.*.js',
          '**/polyfills*.js',
          '**/zone*.js',

          // -----------------------------------------------------------------
          // VENDOR CHUNK: Third-party code, not our IP to protect
          // Obfuscating it just wastes ~30 seconds of build time
          // -----------------------------------------------------------------
          'vendor.*.js',
          '**/vendor*.js',

          // -----------------------------------------------------------------
          // RUNTIME CHUNK: Webpack's module loading bootstrap code
          // If obfuscated, dynamic imports break
          // -----------------------------------------------------------------
          'runtime.*.js',
          '**/runtime*.js',

          // -----------------------------------------------------------------
          // STYLES: CSS extraction creates JS stubs, no code to protect
          // -----------------------------------------------------------------
          'styles.*.js',
          '**/styles*.js',
          '**/*.css',

          // -----------------------------------------------------------------
          // LARGE VISUALIZATION LIBRARIES: Already heavily minified,
          // obfuscation provides minimal benefit but adds build time
          // -----------------------------------------------------------------
          '**/deck.gl/**',
          '**/maplibre-gl/**',
          '**/@deck.gl/**',
          '**/@math.gl/**'
        ]
      )
    );

    // -------------------------------------------------------------------------
    // 3c. DISABLE SOURCE MAPS
    // -------------------------------------------------------------------------
    // Source maps let you reconstruct original code from minified/obfuscated
    // output. For a production build with obfuscation, this defeats the purpose.

    if (config.devtool) {
      console.log('⚠️  Disabling source maps for obfuscated build...');
      config.devtool = false;
    }
  } else {
    console.log('\n📦 Development build - skipping obfuscation\n');
  }

  return config;
};
