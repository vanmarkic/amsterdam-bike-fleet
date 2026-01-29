# Angular 15 → 18 Migration Guide

This document provides comprehensive guidance for migrating the Amsterdam Bike Fleet application from Angular 15 to Angular 18.

## Quick Start

```bash
# Make the script executable
chmod +x scripts/migrate-to-angular18.sh

# Preview what the migration will do (recommended first step)
./scripts/migrate-to-angular18.sh --dry-run

# Run the full migration
./scripts/migrate-to-angular18.sh
```

## Prerequisites

| Requirement | Minimum Version | Recommended |
|-------------|-----------------|-------------|
| Node.js | 18.19.0 | 20.x LTS |
| npm | 8.x | 10.x |
| Git | 2.x | Latest |

**Important**: Angular 18 dropped support for Node.js 16. Ensure you have Node.js 18.19+ or 20.9+ installed.

```bash
# Check your Node.js version
node -v

# If needed, use nvm to switch
nvm install 20
nvm use 20
```

## Migration Path

Angular requires incremental major version upgrades. The script follows this path:

```
Angular 15.2.0  →  Angular 16.x  →  Angular 17.x  →  Angular 18.x
TypeScript 4.9  →  TypeScript 5.0 →  TypeScript 5.2 →  TypeScript 5.4
Zone.js 0.12    →  Zone.js 0.13   →  Zone.js 0.14   →  Zone.js 0.15
```

## Breaking Changes by Version

### Angular 15 → 16

| Change | Impact | This Project |
|--------|--------|--------------|
| Router guards now functional by default | Low | ✅ Already using functional guards |
| `@angular/common/http` standalone APIs | Low | ℹ️ Optional adoption |
| Required inputs with `required: true` | None | New feature |
| Signals introduced (preview) | None | New feature |
| DestroyRef for takeUntilDestroyed() | Low | Consider adopting |

### Angular 16 → 17

| Change | Impact | This Project |
|--------|--------|--------------|
| New control flow syntax (@if, @for) | None | Optional - can migrate incrementally |
| Deferrable views (@defer) | None | New feature for lazy loading |
| Application builder uses esbuild | Medium | Custom webpack may need adjustment |
| Signals become stable | None | Can start adoption |
| SSR hydration improvements | None | Not using SSR |

### Angular 17 → 18

| Change | Impact | This Project |
|--------|--------|--------------|
| Zoneless change detection (experimental) | None | Optional |
| Material 3 design tokens | None | Not using Material |
| Signal inputs/outputs stable | Low | Can modernize components |
| @let template syntax | None | New feature |
| Route redirects with functions | None | New feature |

## Project-Specific Considerations

### 1. Custom Webpack Configuration

The project uses `@angular-builders/custom-webpack` for:
- WebAssembly support
- JavaScript obfuscation

**Migration Action**: Update the builder package alongside Angular:

```bash
npm install @angular-builders/custom-webpack@18 --save-dev
```

The `webpack.config.js` should remain compatible, but verify:
- WASM experiments are still needed (Angular 18 may have better native support)
- Obfuscator plugin works with the new build output

### 2. Standalone Components

The project already uses standalone components for:
- `LicenseDialogComponent`
- `DeliveriesPageComponent`
- `IssuesPageComponent`
- `DelivererGraphPageComponent`
- `ForceGraphComponent`

**Post-Migration Opportunity**: Convert remaining module-based components to standalone for a fully modern architecture.

### 3. Change Detection Strategy

Components using `OnPush` strategy:
- `FleetMapComponent`
- `StatsPanel`
- `BikeListPanel`
- `DeliveriesPageComponent`

These should work without changes. However, Angular 18's signal-based reactivity offers an alternative to manual `markForCheck()` calls.

### 4. RxJS Usage

Current RxJS patterns in use:
- `BehaviorSubject` / `ReplaySubject`
- `takeUntil` with destroy$ subjects
- `switchMap`, `map`, `catchError`, `finalize`

**Post-Migration Opportunity**: Replace `takeUntil` pattern with `takeUntilDestroyed()` from `@angular/core/rxjs-interop`:

```typescript
// Before (current pattern)
private destroy$ = new Subject<void>();

ngOnInit() {
  this.data$.pipe(takeUntil(this.destroy$)).subscribe(...);
}

ngOnDestroy() {
  this.destroy$.next();
  this.destroy$.complete();
}

// After (Angular 16+ pattern)
import { takeUntilDestroyed } from '@angular/core/rxjs-interop';

constructor() {
  this.data$.pipe(takeUntilDestroyed()).subscribe(...);
}
```

### 5. WASM Integration

The Rust/WASM integration via `wasm-pack` should continue working. Verify:

```bash
# Rebuild WASM
npm run wasm:build:web

# Test in development
npm start

# Test protected build
npm run build:protected
```

### 6. Tauri Desktop App

Tauri 2.x should be compatible with Angular 18. After migration:

```bash
# Test development
npm run tauri:dev

# Test production build
npm run tauri:build
```

## Post-Migration Verification Checklist

Run these checks after migration:

### Build Verification

```bash
# Development build
npm run build -- --configuration=development

# Production build
npm run build

# Protected build (with obfuscation)
npm run build:protected
```

### Test Verification

```bash
# Unit tests
npm test

# E2E tests
npm run e2e
```

### Runtime Verification

- [ ] Application starts without console errors
- [ ] Map renders correctly (deck.gl / maplibre-gl)
- [ ] WASM module loads and executes
- [ ] License check works
- [ ] Navigation between routes works
- [ ] Change detection updates UI correctly
- [ ] Animations work (if any)

### Tauri Verification

```bash
# Development mode
npm run tauri:dev

# Production build
npm run tauri:build
```

- [ ] Desktop app starts
- [ ] License dialog appears
- [ ] All features work as in browser

## Optional Modernizations

After successful migration, consider these improvements:

### 1. Convert to Fully Standalone

Replace `AppModule` with standalone bootstrap:

```typescript
// main.ts (new pattern)
import { bootstrapApplication } from '@angular/platform-browser';
import { AppComponent } from './app/app.component';
import { appConfig } from './app/app.config';

bootstrapApplication(AppComponent, appConfig);
```

### 2. Adopt New Control Flow

Replace `*ngIf` and `*ngFor` with new syntax:

```html
<!-- Before -->
<div *ngIf="bike">{{ bike.name }}</div>
<div *ngFor="let bike of bikes">{{ bike.name }}</div>

<!-- After -->
@if (bike) {
  <div>{{ bike.name }}</div>
}
@for (bike of bikes; track bike.id) {
  <div>{{ bike.name }}</div>
}
```

### 3. Use Signal-Based Inputs

```typescript
// Before
@Input() bikeId: string;

// After
bikeId = input<string>();
```

### 4. Use Signal-Based Outputs

```typescript
// Before
@Output() bikeSelected = new EventEmitter<Bike>();

// After
bikeSelected = output<Bike>();
```

### 5. Consider Zoneless (Experimental)

For maximum performance, Angular 18 supports zoneless applications:

```typescript
// app.config.ts
export const appConfig: ApplicationConfig = {
  providers: [
    provideExperimentalZonelessChangeDetection()
  ]
};
```

**Warning**: This requires all change detection to be signal-based or manually triggered.

## Troubleshooting

### Build Fails After Migration

1. Delete `node_modules` and `package-lock.json`
2. Run `npm install`
3. Check for peer dependency warnings

### TypeScript Errors

Angular 18 uses TypeScript 5.4 with stricter type checking:

```bash
# Check for type errors
npx tsc --noEmit
```

### Custom Webpack Issues

If the custom webpack configuration breaks:

1. Check `@angular-builders/custom-webpack` version matches Angular
2. Verify webpack 5 compatibility
3. Check obfuscator plugin compatibility

### WASM Loading Fails

1. Verify WASM files are copied to `dist/assets/wasm/`
2. Check browser console for CORS errors
3. Ensure `asyncWebAssembly` experiment is still enabled

## Rollback

If migration fails, restore from backup branch:

```bash
# List backup branches
git branch | grep pre-angular

# Restore to pre-migration state
git checkout pre-angular-15-original-migration
git checkout -b main-restored

# Reinstall dependencies
rm -rf node_modules package-lock.json
npm install
```

## Resources

- [Angular Update Guide](https://angular.dev/update-guide)
- [Angular 16 Release Notes](https://blog.angular.io/angular-v16-is-here-4d7a28ec680d)
- [Angular 17 Release Notes](https://blog.angular.io/introducing-angular-v17-4d7bca9e7ef4)
- [Angular 18 Release Notes](https://blog.angular.io/angular-v18-is-now-available-e79d5ac0affe)
- [Migration to Standalone](https://angular.dev/reference/migrations/standalone)
- [New Control Flow](https://angular.dev/guide/templates/control-flow)
