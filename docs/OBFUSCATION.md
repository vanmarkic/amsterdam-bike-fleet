# JavaScript Obfuscation Guide

This document explains the JavaScript obfuscation setup for the Amsterdam Bike Fleet application.

## Overview

JavaScript obfuscation transforms readable source code into a form that is difficult to understand and reverse-engineer while maintaining the same functionality. This adds a layer of protection for intellectual property and sensitive business logic.

## What Obfuscation Does

### Code Transformations Applied

| Transformation | Description | Impact |
|----------------|-------------|--------|
| **Control Flow Flattening** | Restructures code execution flow using switch-case constructs | High protection, moderate performance cost |
| **Dead Code Injection** | Adds fake code paths that never execute | Confuses static analysis |
| **String Array** | Extracts strings into an encoded array | Hides readable strings |
| **String Encoding** | Applies Base64 encoding to extracted strings | Additional layer of protection |
| **String Rotation** | Rotates the string array at runtime | Makes static analysis harder |
| **Self-Defending** | Code detects and breaks when beautified | Prevents easy reformatting |
| **Identifier Mangling** | Replaces variable/function names with hexadecimal | Removes semantic meaning |
| **Object Key Transformation** | Transforms object property names | Hides object structure |
| **Numbers to Expressions** | Converts numbers to complex expressions | Obscures numeric values |

### Before Obfuscation
```javascript
function calculateDiscount(price, percentage) {
  const discount = price * (percentage / 100);
  return price - discount;
}
```

### After Obfuscation (simplified example)
```javascript
var _0x1a2b=['price','percentage','discount'];
(function(_0x3c4d,_0x5e6f){var _0x7a8b=function(_0x9c0d){
while(--_0x9c0d){_0x3c4d['push'](_0x3c4d['shift']());}};
_0x7a8b(++_0x5e6f);}(_0x1a2b,0x1e3));
var _0x2b1a=function(_0x3c4d,_0x5e6f){...};
function _0x4d3e(_0x7f8a,_0x9b0c){var _0xde12=_0x7f8a*(_0x9b0c/0x64);
return _0x7f8a-_0xde12;}
```

## Build Commands

### Development Build (No Obfuscation)
```bash
# Fast development server
npm start

# Standard production build (no obfuscation)
npm run build
```

### Protected Production Build
```bash
# Build with obfuscation
npm run build:protected

# Build Tauri app with obfuscation
npm run tauri:build:protected
```

## Configuration Files

### `obfuscator.config.js`
Main configuration file with all obfuscation settings. Located at project root.

### `webpack.config.js`
Custom webpack configuration that applies obfuscation only in production mode.

## Adjusting Security vs Performance

### High Security (Slower Performance)
Edit `obfuscator.config.js`:
```javascript
{
  controlFlowFlatteningThreshold: 0.75,  // Up from 0.5
  deadCodeInjectionThreshold: 0.5,       // Up from 0.3
  stringArrayThreshold: 0.9,             // Up from 0.75
  debugProtection: true,                 // Enable
  disableConsoleOutput: true,            // Enable
  stringArrayEncoding: ['rc4'],          // Stronger encoding
  splitStringsChunkLength: 5,            // Smaller chunks
}
```

### Maximum Performance (Lower Security)
Edit `obfuscator.config.js`:
```javascript
{
  controlFlowFlattening: false,          // Disable
  deadCodeInjection: false,              // Disable
  stringArrayThreshold: 0.5,             // Lower
  selfDefending: false,                  // Disable
  splitStrings: false,                   // Disable
  numbersToExpressions: false,           // Disable
}
```

### Balanced (Default)
The default configuration provides a good balance between protection and performance.

## Verifying Obfuscation

### 1. Build and Compare File Sizes
```bash
# Regular build
npm run build
ls -la dist/amsterdam-bike-fleet/*.js | head -5

# Protected build
npm run build:protected
ls -la dist/amsterdam-bike-fleet/*.js | head -5
```
Obfuscated files will be 20-40% larger.

### 2. Inspect Output Files
```bash
# View first 50 characters of main bundle
head -c 500 dist/amsterdam-bike-fleet/main.*.js
```
Look for:
- Hexadecimal variable names (`_0x1a2b`, `_0x3c4d`)
- String array at the beginning
- No readable function names

### 3. Try Beautifying
Use a code beautifier on the output. If self-defending is enabled, the code should malfunction when formatted.

### 4. Check for Strings
```bash
# Search for specific strings that should be hidden
grep -o "calculateDiscount" dist/amsterdam-bike-fleet/main.*.js
```
Should return nothing if string obfuscation is working.

## Understanding Vendor Chunks

When Angular/Webpack builds your app, it can split the output into multiple files called "chunks":

```
dist/
├── main.abc123.js        ← YOUR code (components, services, business logic)
├── vendor.def456.js      ← THIRD-PARTY code (Angular, RxJS, deck.gl, maplibre)
├── polyfills.ghi789.js   ← Browser compatibility code
├── runtime.jkl012.js     ← Webpack's module loading logic
└── styles.mno345.css     ← Your stylesheets
```

### Why Separate Vendor Chunks?

**1. Better Caching**
```
User visits your app:
  ├── First visit: Downloads vendor.js (2MB) + main.js (300KB)
  │
  └── You deploy a bug fix (only YOUR code changes):
        ├── vendor.js → Same hash, served from browser cache ✅
        └── main.js → New hash, downloads fresh copy (300KB only)
```

Without vendor splitting, users would re-download the entire 2.3MB bundle for every tiny change you make.

**2. Selective Obfuscation (Critical for Protection)**
```
┌─────────────────────────────────────────────────┐
│  vendor.js (NOT obfuscated)                     │
│  • Angular framework                            │
│  • RxJS                                         │
│  • deck.gl, maplibre                            │
│  • Already minified by library authors          │
│  • Obfuscating would break things & add bloat   │
└─────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────┐
│  main.js (OBFUSCATED)                           │
│  • Your components                              │
│  • Your services                                │
│  • Your business logic                          │
│  • THIS is what you want to protect!            │
└─────────────────────────────────────────────────┘
```

**3. Parallel Loading**

The browser can download multiple chunks simultaneously:
```
vendor.js  ████████████████████░░░░░
main.js    ██████████░░░░░░░░░░░░░░░
polyfills  █████░░░░░░░░░░░░░░░░░░░░
                                    → App ready faster!
```

### Your Build Output Example

From a protected build:
```
vendor.082796f6f587394f.js  | 2.08 MB  ← Third-party (NOT obfuscated)
main.eecad73056c58169.js    | 294 KB   ← Your code (OBFUSCATED ✓)
```

The `vendorChunk: true` setting in `angular.json` enables this split:
```json
"configurations": {
  "production": {
    "vendorChunk": true
  }
}
```

## Excluded Files

The following are excluded from obfuscation to prevent breaking the application:

- `node_modules/**` - Third-party libraries (already in vendor chunk)
- `**/*.wasm` - WebAssembly files (binary, already protected)
- `**/wasm-lib/**` - WASM source directory
- `**/polyfills*.js` - Angular polyfills (must remain unobfuscated)
- `**/vendor*.js` - Vendor chunks (third-party code, would break if obfuscated)
- `**/runtime*.js` - Webpack runtime (module loader, must remain functional)
- `**/zone*.js` - Zone.js (Angular change detection, very sensitive to modification)

## Troubleshooting

### Application Breaks After Obfuscation

1. **Check Console Errors**: Look for undefined variable errors
2. **Increase Reserved Names**: Add breaking identifiers to `reservedNames` in config
3. **Reduce Aggressiveness**: Lower threshold values
4. **Exclude Specific Files**: Add to exclusion list in `webpack.config.js`

### Build Fails

1. **Memory Issues**: Obfuscation is memory-intensive
   ```bash
   NODE_OPTIONS="--max-old-space-size=8192" npm run build:protected
   ```

2. **Timeout**: Increase webpack timeout or reduce obfuscation complexity

### Zone.js Issues
If Angular change detection breaks:
- Ensure `zone*.js` is in the exclusion list
- Add Zone-related identifiers to `reservedNames`

## Security Considerations

- **Not Encryption**: Obfuscation is not encryption. Determined attackers can eventually reverse-engineer the code.
- **No Source Maps**: Never deploy source maps with obfuscated production builds.
- **Server-Side Secrets**: Never put API keys or secrets in client-side code, even obfuscated.
- **Defense in Depth**: Use obfuscation as one layer of protection, not the only one.

## Performance Impact

| Metric | Typical Impact |
|--------|----------------|
| Bundle Size | +20-40% |
| Initial Parse Time | +10-20% |
| Runtime Execution | +5-15% slower |
| Memory Usage | +10-20% |

These impacts are for the default balanced configuration. Aggressive settings will increase these numbers.

## References

- [javascript-obfuscator GitHub](https://github.com/javascript-obfuscator/javascript-obfuscator)
- [webpack-obfuscator](https://github.com/javascript-obfuscator/webpack-obfuscator)
- [@angular-builders/custom-webpack](https://github.com/just-jeb/angular-builders/tree/master/packages/custom-webpack)
