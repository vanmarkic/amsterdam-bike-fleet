/**
 * JavaScript Obfuscator Configuration
 *
 * This configuration provides a balance between code protection and runtime performance.
 * The settings are optimized for production Angular applications.
 *
 * Security Level: Medium-High
 * Performance Impact: Moderate (15-30% larger bundle, ~10-20% slower execution)
 *
 * @see https://github.com/javascript-obfuscator/javascript-obfuscator
 */

module.exports = {
  // === COMPACTION ===
  // Removes whitespace and formats code in a single line
  compact: true,

  // === CONTROL FLOW FLATTENING ===
  // Transforms code structure to make it harder to understand
  // Higher threshold = more protection but slower execution
  controlFlowFlattening: true,
  controlFlowFlatteningThreshold: 0.5, // Apply to 50% of code blocks

  // === DEAD CODE INJECTION ===
  // Injects fake code that never executes
  // Makes reverse engineering more difficult
  deadCodeInjection: true,
  deadCodeInjectionThreshold: 0.3, // 30% - balanced for bundle size

  // === DEBUG PROTECTION ===
  // Makes debugging in browser DevTools difficult
  debugProtection: false, // Enable for higher security (may cause issues in some browsers)
  debugProtectionInterval: 0,

  // === CONSOLE DISABLING ===
  // Disables console.* methods - NOT recommended for production debugging
  disableConsoleOutput: false,

  // === IDENTIFIER MANGLING ===
  // Renames variables and function names
  identifierNamesGenerator: 'hexadecimal', // Options: 'dictionary', 'hexadecimal', 'mangled', 'mangled-shuffled'
  identifiersPrefix: '', // Add prefix to avoid collisions
  renameGlobals: false, // Keep false to avoid breaking external dependencies

  // === SELF-DEFENDING ===
  // Code will break if formatted/beautified
  selfDefending: true,

  // === STRING ARRAY ===
  // Extracts strings into an array and replaces them with references
  stringArray: true,
  stringArrayCallsTransform: true,
  stringArrayCallsTransformThreshold: 0.5,
  stringArrayEncoding: ['base64'], // Options: 'none', 'base64', 'rc4'
  stringArrayIndexShift: true,
  stringArrayRotate: true,
  stringArrayShuffle: true,
  stringArrayWrappersCount: 2,
  stringArrayWrappersChainedCalls: true,
  stringArrayWrappersParametersMaxCount: 4,
  stringArrayWrappersType: 'function',
  stringArrayThreshold: 0.75, // 75% of strings will be moved to array

  // === TRANSFORMATIONS ===
  // Additional code transformations
  transformObjectKeys: true,
  numbersToExpressions: true,
  simplify: true,
  splitStrings: true,
  splitStringsChunkLength: 10,

  // === UNICODE ESCAPING ===
  // Escapes unicode characters (increases size but adds protection)
  unicodeEscapeSequence: false, // Keep false for reasonable bundle size

  // === TARGET ===
  // Target environment
  target: 'browser',

  // === SOURCE MAPS ===
  // NEVER enable in production - defeats the purpose of obfuscation
  sourceMap: false,

  // === RESERVED ===
  // Identifiers that should NOT be renamed
  reservedNames: [
    // Angular-specific
    '^ng.*',
    '^_ng.*',
    // Zone.js
    '^Zone$',
    '^__zone_symbol__.*',
    // Webpack
    '^__webpack.*',
    // WASM exports
    '^wasm.*',
    // Common globals
    '^window$',
    '^document$',
    '^console$'
  ],
  reservedStrings: [
    // Keep Angular template bindings readable for debugging
    '\\[.*\\]',
    '\\(.*\\)'
  ],

  // === DOMAIN LOCK ===
  // Uncomment and configure to lock code to specific domains
  // domainLock: ['your-domain.com', '.your-domain.com'],
  // domainLockRedirectUrl: 'about:blank',

  // === SEED ===
  // Use a fixed seed for reproducible builds (useful for debugging)
  // seed: 12345,

  // === EXCLUDE ===
  // Files/patterns to exclude from obfuscation
  // (handled in webpack config, not here)
};
