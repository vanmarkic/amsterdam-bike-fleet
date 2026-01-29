# AGENTS.md - Angular 18 Documentation Index

> **IMPORTANT**: Prefer retrieval-led reasoning over pre-training-led reasoning for any Angular tasks.
> This project uses Angular 18.2.x with standalone components as the default.

## Project Context

| Aspect | Value |
|--------|-------|
| Framework | Angular 18.2.14 |
| Build | @angular-devkit/build-angular 18.2.21 |
| TypeScript | 5.4.x |
| Desktop | Tauri 2.x |
| Maps | MapLibre GL + deck.gl |
| WASM | wasm-pack/wasm-bindgen |

---

## [Angular 18 Docs Index]

### Core Concepts
|concept|key-patterns|
|-------|------------|
|standalone-components|`@Component({standalone:true,imports:[...]})` - DEFAULT in v18, no NgModule needed|
|signals|`signal()`,`computed()`,`effect()` - reactive primitives replacing zone.js patterns|
|control-flow|`@if`,`@for`,`@switch`,`@defer` - template syntax replacing *ngIf/*ngFor|
|dependency-injection|`inject()` function preferred over constructor injection|
|change-detection|`ChangeDetectionStrategy.OnPush` + signals for optimal performance|

### Component Patterns (Angular 18)
```typescript
// PREFERRED: Standalone component with new control flow
@Component({
  standalone: true,
  selector: 'app-example',
  imports: [CommonModule, RouterModule],
  template: `
    @if (items().length > 0) {
      @for (item of items(); track item.id) {
        <div>{{ item.name }}</div>
      }
    } @else {
      <div>No items</div>
    }
  `
})
export class ExampleComponent {
  private service = inject(ExampleService);
  items = signal<Item[]>([]);

  constructor() {
    effect(() => console.log('Items changed:', this.items()));
  }
}
```

### Routing (v18)
|pattern|usage|
|-------|-----|
|standalone-routes|`provideRouter(routes)` in app.config.ts|
|lazy-loading|`loadComponent: () => import('./x.component')`|
|guards|`canActivate: [() => inject(AuthService).isLoggedIn()]`|
|resolvers|`resolve: { data: () => inject(DataService).load() }`|

### Services & DI
|pattern|code|
|-------|-----|
|providedIn-root|`@Injectable({providedIn:'root'})` - tree-shakable singleton|
|inject-function|`private http = inject(HttpClient)` - preferred over constructor|
|injection-token|`const TOKEN = new InjectionToken<T>('desc')`|

### Forms (v18)
|type|imports|usage|
|----|-------|-----|
|reactive|`ReactiveFormsModule`|`FormControl`,`FormGroup`,`FormBuilder`|
|typed-forms|built-in|`FormControl<string>` - strict typing|
|signals-forms|experimental|`FormControl` with `.valueChanges` → signal|

### HTTP & State
|pattern|approach|
|-------|--------|
|http-client|`inject(HttpClient)` + RxJS operators|
|signal-state|`signal()` for local, services for shared|
|rxjs-interop|`toSignal()`,`toObservable()` from @angular/core/rxjs-interop|

### Testing
|tool|usage|
|----|-----|
|component-test|`TestBed.configureTestingModule({imports:[Component]})`|
|service-test|`TestBed.inject(Service)` or direct instantiation|
|http-test|`provideHttpClientTesting()` + `HttpTestingController`|

---

## Angular 18 Breaking Changes from v15-v17

|change|migration|
|------|---------|
|standalone-default|Components standalone by default; add `standalone:false` for NgModule|
|control-flow-syntax|`*ngIf`→`@if`, `*ngFor`→`@for`, `ngSwitch`→`@switch`|
|inject-preferred|Constructor injection still works but `inject()` preferred|
|zoneless-preview|`provideExperimentalZonelessChangeDetection()` available|
|defer-blocks|`@defer` for lazy loading parts of templates|
|ssr-hydration|`provideClientHydration()` for SSR apps|

---

## Project-Specific Patterns

### MapLibre + deck.gl Integration
```typescript
// Initialize MapLibre with deck.gl overlay
const map = new maplibregl.Map({...});
const deckOverlay = new MapboxOverlay({
  layers: [new ScatterplotLayer({...})]
});
map.addControl(deckOverlay);
```

### Tauri Integration
```typescript
// Import Tauri APIs
import { invoke } from '@tauri-apps/api/core';
// Call Rust backend
const result = await invoke('command_name', { arg: value });
```

### WASM Integration
```typescript
// Load WASM module
import init, { wasm_function } from 'wasm-lib';
await init();
const result = wasm_function(input);
```

---

## Quick Reference Commands

```bash
# Development
ng serve                    # Start dev server
ng generate component name  # Generate standalone component
ng build                    # Production build

# Testing
ng test                     # Unit tests (Karma)
npm run e2e                 # E2E tests (Playwright)

# Tauri
npm run tauri:dev          # Dev with Tauri
npm run tauri:build        # Production Tauri build

# WASM
npm run wasm:build         # Build WASM module
```

---

## Migration Checklist (if upgrading from v15)

- [ ] Update to standalone components (remove NgModules)
- [ ] Replace `*ngIf`/`*ngFor` with `@if`/`@for` syntax
- [ ] Use `inject()` instead of constructor injection
- [ ] Convert `BehaviorSubject` to `signal()` where appropriate
- [ ] Add `track` expression to all `@for` loops
- [ ] Review guards/resolvers for functional syntax
- [ ] Update HttpClient usage with `provideHttpClient()`
