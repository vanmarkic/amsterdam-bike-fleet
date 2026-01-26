/**
 * Custom Webpack Configuration for JavaScript Obfuscation
 *
 * This configuration applies JavaScript obfuscation to production builds only.
 * Development builds remain fast and debuggable.
 */

const WebpackObfuscator = require('webpack-obfuscator');
const obfuscatorConfig = require('./obfuscator.config');

/**
 * Custom webpack configuration
 * @param {object} config - The default Angular webpack configuration
 * @param {object} options - Build options from Angular CLI
 * @returns {object} Modified webpack configuration
 */
module.exports = (config, _options) => {
  // Only apply obfuscation in production mode
  const isProduction = config.mode === 'production';

  if (isProduction) {
    console.log('\n🔒 Applying JavaScript obfuscation for production build...\n');

    // Configure vendor chunk splitting to separate app code from third-party
    // This allows us to only obfuscate our application code
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

    // Add WebpackObfuscator plugin
    config.plugins.push(
      new WebpackObfuscator(
        obfuscatorConfig,
        // Files to exclude from obfuscation (glob patterns)
        [
          // Exclude all node_modules content
          '**/node_modules/**',
          'node_modules/**',

          // Exclude WASM files and related JavaScript
          '**/*.wasm',
          '**/wasm-lib/**',
          '**/pkg/**',
          '**/pkg-web/**',

          // Exclude source maps
          '**/*.map',

          // Exclude polyfills (may break Zone.js)
          'polyfills.*.js',
          '**/polyfills*.js',

          // Exclude vendor chunks (third-party code) - CRITICAL
          'vendor.*.js',
          '**/vendor*.js',

          // Exclude runtime chunk (webpack runtime)
          'runtime.*.js',
          '**/runtime*.js',

          // Exclude styles
          'styles.*.js',
          '**/styles*.js',
          '**/*.css',

          // Exclude zone.js specifically
          '**/zone*.js',

          // Exclude deck.gl, maplibre, and other large libraries
          '**/deck.gl/**',
          '**/maplibre-gl/**',
          '**/@deck.gl/**',
          '**/@math.gl/**'
        ]
      )
    );

    // Ensure we're not generating source maps in production with obfuscation
    // Source maps would defeat the purpose of obfuscation
    if (config.devtool) {
      console.log('⚠️  Disabling source maps for obfuscated build...');
      config.devtool = false;
    }
  } else {
    console.log('\n📦 Development build - skipping obfuscation\n');
  }

  return config;
};
