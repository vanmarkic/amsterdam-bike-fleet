# Angular 15 → 18 Migration Guide

This document provides comprehensive guidance for migrating Angular projects from version 15 to 18, based on real-world migration experience and community best practices.

**Sources:**
- [Angular Update Guide](https://angular.dev/update-guide)
- [Angular Migration Schematics](https://angular.dev/reference/migrations)
- [Standalone Migration](https://angular.dev/reference/migrations/standalone)
- [Control Flow Migration](https://angular.dev/reference/migrations/control-flow)

## Quick Start

```bash
# Make the script executable
chmod +x scripts/migrate-to-angular18.sh

# Preview what the migration will do (recommended first step)
./scripts/migrate-to-angular18.sh --dry-run

# Run the full migration
./scripts/migrate-to-angular18.sh

# With optional modernizations (standalone + new control flow)
./scripts/migrate-to-angular18.sh --standalone --control-flow
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

After successful migration, Angular provides schematics to modernize your codebase automatically.

### 1. Convert to Standalone Components

Angular 16+ fully supports standalone components without NgModules. Run the migration in three steps:

```bash
# Step 1: Convert all declarations to standalone
ng g @angular/core:standalone

# Step 2: Remove unnecessary NgModules
ng g @angular/core:standalone

# Step 3: Bootstrap with standalone API
ng g @angular/core:standalone
```

Or use the script option:
```bash
./scripts/migrate-to-angular18.sh --standalone
```

**Before (NgModule):**
```typescript
@NgModule({
  declarations: [AppComponent],
  imports: [BrowserModule],
  bootstrap: [AppComponent]
})
export class AppModule {}
```

**After (Standalone):**
```typescript
// main.ts
bootstrapApplication(AppComponent, {
  providers: [provideRouter(routes)]
});
```

### 2. Migrate to New Control Flow

Angular 17+ introduces `@if`, `@for`, `@switch` to replace structural directives. These will be **deprecated in Angular 20**.

```bash
# Automatic migration
ng g @angular/core:control-flow

# Or use script option
./scripts/migrate-to-angular18.sh --control-flow
```

**Before:**
```html
<div *ngIf="bike; else noBike">{{ bike.name }}</div>
<ng-template #noBike>No bike</ng-template>

<div *ngFor="let bike of bikes; trackBy: trackById">
  {{ bike.name }}
</div>
```

**After:**
```html
@if (bike) {
  <div>{{ bike.name }}</div>
} @else {
  <div>No bike</div>
}

@for (bike of bikes; track bike.id) {
  <div>{{ bike.name }}</div>
}
```

### 3. Use Signal-Based APIs

Angular 18 stabilizes signal inputs, outputs, and queries:

```typescript
// Signal Inputs (replaces @Input)
bikeId = input<string>();                    // optional
bikeId = input.required<string>();           // required
bikeId = input<string>('default');           // with default

// Signal Outputs (replaces @Output)
bikeSelected = output<Bike>();
this.bikeSelected.emit(bike);

// Signal Queries (replaces @ViewChild/@ContentChild)
bikeList = viewChild<BikeListComponent>('bikeList');
allBikes = viewChildren<BikeComponent>(BikeComponent);
```

### 4. Use Typed Reactive Forms

Angular 16+ requires typed forms. Migrate gradually:

```typescript
// Untyped (legacy, still works)
form = new UntypedFormControl('');

// Typed (recommended)
name = new FormControl<string>('', { nonNullable: true });
form = new FormGroup({
  name: new FormControl<string>(''),
  age: new FormControl<number>(0)
});
```

### 5. Use takeUntilDestroyed()

Replace manual destroy$ subjects:

```typescript
// Before
private destroy$ = new Subject<void>();

ngOnInit() {
  this.data$.pipe(takeUntil(this.destroy$)).subscribe(...);
}

ngOnDestroy() {
  this.destroy$.next();
  this.destroy$.complete();
}

// After (Angular 16+)
import { takeUntilDestroyed } from '@angular/core/rxjs-interop';

constructor() {
  this.data$.pipe(takeUntilDestroyed()).subscribe(...);
}
```

### 6. Consider Zoneless (Experimental)

For maximum performance, Angular 18 supports zoneless applications:

```typescript
// app.config.ts
export const appConfig: ApplicationConfig = {
  providers: [
    provideExperimentalZonelessChangeDetection()
  ]
};
```

**Warning**: This requires all change detection to be signal-based or manually triggered with `ChangeDetectorRef.detectChanges()`.

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

## Common Issues and Solutions

### ERESOLVE Peer Dependency Conflicts

**Problem:** npm 7+ treats peer dependency mismatches as errors.

**Solutions:**
1. Use `--legacy-peer-deps` flag:
   ```bash
   npm install --legacy-peer-deps
   ```

2. Set globally:
   ```bash
   npm config set legacy-peer-deps true
   ```

3. Use npm overrides in `package.json`:
   ```json
   {
     "overrides": {
       "@problematic/package": "^18.0.0"
     }
   }
   ```

### Third-Party Library Compatibility

**Problem:** Libraries don't always update in sync with Angular releases.

**Solutions:**
1. Check library's GitHub for Angular 18 support
2. Use `npm ls` to identify dependency tree issues
3. Replace outdated libraries with maintained alternatives
4. Open issues/PRs on library repositories

### Angular Material MDC Migration (v15+)

**Problem:** Material 15 switched to MDC-based components with class name changes.

**Solution:**
- Classes changed from `mat-*` to `mat-mdc-*`
- Run `ng update @angular/material` for automatic migration
- Update custom SCSS that targets Material classes

### Typed Reactive Forms Errors (v16+)

**Problem:** Loosely typed form code throws compile errors.

**Solution:**
- Use `UntypedFormControl` / `UntypedFormGroup` for gradual migration
- Migrate to typed forms module by module
- Example: `new FormControl<string>('', { nonNullable: true })`

### SSR Hydration Mismatches

**Problem:** Server and client rendered content differs.

**Solution:**
- Use `TransferState` to avoid double API fetches
- Ensure stable element IDs in lists
- Set up SSR-focused integration tests

## Resources

- [Angular Update Guide](https://angular.dev/update-guide)
- [Angular Migration Schematics](https://angular.dev/reference/migrations)
- [Angular 16 Release Notes](https://blog.angular.io/angular-v16-is-here-4d7a28ec680d)
- [Angular 17 Release Notes](https://blog.angular.io/introducing-angular-v17-4d7bca9e7ef4)
- [Angular 18 Release Notes](https://blog.angular.io/angular-v18-is-now-available-e79d5ac0affe)
- [Standalone Migration](https://angular.dev/reference/migrations/standalone)
- [Control Flow Migration](https://angular.dev/reference/migrations/control-flow)
- [Typed Reactive Forms](https://angular.dev/guide/forms/typed-forms)
- [Real-World Migration Guide](https://www.mol-tech.us/blog/migrate-angular-15-to-angular-18)
