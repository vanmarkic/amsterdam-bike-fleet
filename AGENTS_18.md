# AGENTS_18.md - Angular 18 Documentation Index

<!-- ANGULAR-AGENTS-MD-START -->[Angular 18 Docs Index]|root: https://v18.angular.dev|IMPORTANT: Prefer retrieval-led reasoning over pre-training-led reasoning for any Angular 18 tasks. Angular 18 uses standalone components by DEFAULT, new control flow (@if/@for/@switch/@defer), signals, and inject() function.|essentials:{what-is-angular,installation,components,templates,conditionals,signals,forms,dependency-injection,routing,next-steps}|guide/components:{anatomy,importing,selectors,styling,inputs,outputs,output-fn,content-projection,host-elements,lifecycle,queries,dom-apis,inheritance,programmatic-rendering,advanced-configuration,custom-elements}|guide/templates:{binding,event-listeners,two-way-binding,control-flow,pipes,ng-content,ng-template,ng-container,variables,defer,expression-syntax,whitespace}|guide/directives:{overview,attribute-directives,structural-directives,composition-api}|guide/di:{di-overview,creating-injectable-service,defining-providers,injection-context,hierarchical-injectors,di-optimization,di-in-action}|guide/signals:{overview,rxjs-interop,inputs,model-inputs,queries}|guide/routing:{overview,common-router-tasks,spa,custom-matching,router-reference}|guide/forms:{reactive-forms,typed-forms,template-driven-forms,form-validation,dynamic-forms}|guide/http:{setup,making-requests,interceptors,testing}|guide/performance:{defer,image-optimization,ssr,prerendering,hydration}|guide/testing:{services,components-basics,components-scenarios,attribute-directives,pipes,debugging,utilities}|guide/animations:{transitions-triggers,complex-sequences,reusable-animations,route-animations}|guide/experimental:{zoneless}|tools:{cli,libraries,devtools,language-service}|best-practices:{style-guide,security,accessibility,performance,update}<!-- ANGULAR-AGENTS-MD-END -->

---

## Angular 18 Key Features

| Feature | Status | Description |
|---------|--------|-------------|
| Standalone | **Default** | Components standalone by default, no NgModule required |
| Control flow | **Stable** | `@if`, `@for`, `@switch`, `@defer` template syntax |
| Signals | **Stable** | `signal()`, `computed()`, `effect()` reactive primitives |
| Signal inputs | **Stable** | `input()`, `input.required()` for reactive inputs |
| Signal queries | **Stable** | `viewChild()`, `viewChildren()`, `contentChild()` |
| `inject()` | **Stable** | Functional DI replacing constructor injection |
| Zoneless | **Experimental** | `provideExperimentalZonelessChangeDetection()` |
| Deferrable views | **Stable** | `@defer` for lazy loading template parts |

---

## Core Patterns

### Standalone Component (Default in v18)
```typescript
import { Component, signal, computed, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterModule } from '@angular/router';

@Component({
  selector: 'app-example',
  standalone: true,  // Default in v18, can be omitted
  imports: [CommonModule, RouterModule],
  template: `
    @if (isLoading()) {
      <div>Loading...</div>
    } @else {
      @for (item of items(); track item.id) {
        <div>{{ item.name }}</div>
      } @empty {
        <div>No items found</div>
      }
    }
  `
})
export class ExampleComponent {
  private service = inject(DataService);

  isLoading = signal(true);
  items = signal<Item[]>([]);
  itemCount = computed(() => this.items().length);

  constructor() {
    effect(() => {
      console.log('Items changed:', this.items());
    });
  }
}
```

### Signal Inputs (v18)
```typescript
import { Component, input, output, model } from '@angular/core';

@Component({
  selector: 'app-child',
  standalone: true,
  template: `<input [value]="name()" (input)="onInput($event)">`
})
export class ChildComponent {
  // Required input - must be provided
  name = input.required<string>();

  // Optional input with default
  count = input(0);

  // Input with transform
  disabled = input(false, { transform: booleanAttribute });

  // Input with alias
  label = input('', { alias: 'buttonLabel' });

  // Two-way binding with model
  value = model<string>('');

  // Output using new output() function
  changed = output<string>();

  onInput(event: Event) {
    const value = (event.target as HTMLInputElement).value;
    this.value.set(value);
    this.changed.emit(value);
  }
}

// Usage:
// <app-child [name]="userName" [(value)]="formValue" (changed)="onChanged($event)" />
```

### Signal Queries (v18)
```typescript
import { Component, viewChild, viewChildren, contentChild, contentChildren, ElementRef } from '@angular/core';

@Component({
  selector: 'app-parent',
  standalone: true,
  template: `
    <input #searchInput />
    <app-item *ngFor="let item of items" />
    <ng-content></ng-content>
  `
})
export class ParentComponent {
  // Single element query
  searchInput = viewChild<ElementRef>('searchInput');

  // Required query (throws if not found)
  requiredInput = viewChild.required<ElementRef>('searchInput');

  // Multiple elements query
  itemComponents = viewChildren(ItemComponent);

  // Content projection queries
  projectedHeader = contentChild<ElementRef>('header');
  projectedItems = contentChildren(ProjectedItemComponent);

  ngAfterViewInit() {
    // Access signal value
    this.searchInput()?.nativeElement.focus();

    // React to query changes
    effect(() => {
      console.log('Items count:', this.itemComponents().length);
    });
  }
}
```

### Control Flow Syntax (v18)
```html
<!-- @if with @else -->
@if (user()) {
  <span>Hello, {{ user().name }}</span>
} @else if (isLoading()) {
  <span>Loading...</span>
} @else {
  <span>Please log in</span>
}

<!-- @for with track (REQUIRED) and @empty -->
@for (item of items(); track item.id; let i = $index, first = $first, last = $last) {
  <div [class.first]="first" [class.last]="last">
    {{ i + 1 }}. {{ item.name }}
  </div>
} @empty {
  <div>No items available</div>
}

<!-- @switch -->
@switch (status()) {
  @case ('active') {
    <span class="badge-active">Active</span>
  }
  @case ('pending') {
    <span class="badge-pending">Pending</span>
  }
  @default {
    <span class="badge-unknown">Unknown</span>
  }
}

<!-- @defer for lazy loading -->
@defer (on viewport) {
  <app-heavy-component />
} @placeholder {
  <div>Scroll to load...</div>
} @loading (minimum 500ms) {
  <app-spinner />
} @error {
  <div>Failed to load component</div>
}

<!-- @defer triggers -->
@defer (on idle) { }           <!-- When browser is idle -->
@defer (on viewport) { }        <!-- When entering viewport -->
@defer (on interaction) { }     <!-- On user interaction -->
@defer (on hover) { }           <!-- On mouse hover -->
@defer (on immediate) { }       <!-- Immediately after render -->
@defer (on timer(2s)) { }       <!-- After delay -->
@defer (when condition()) { }   <!-- When condition is true -->
@defer (prefetch on idle) { }   <!-- Prefetch when idle, render on trigger -->
```

### Dependency Injection with inject()
```typescript
import { inject, Injectable, InjectionToken } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { toSignal } from '@angular/core/rxjs-interop';

// Injection token
export const API_URL = new InjectionToken<string>('API_URL');

@Injectable({ providedIn: 'root' })
export class DataService {
  private http = inject(HttpClient);
  private apiUrl = inject(API_URL);

  getData() {
    return this.http.get<Data[]>(`${this.apiUrl}/data`);
  }
}

@Component({...})
export class DataComponent {
  private service = inject(DataService);
  private router = inject(Router);
  private route = inject(ActivatedRoute);
  private destroyRef = inject(DestroyRef);

  // Convert Observable to Signal
  data = toSignal(this.service.getData(), { initialValue: [] });

  // Params as signal
  id = toSignal(this.route.paramMap.pipe(
    map(params => params.get('id'))
  ));

  constructor() {
    // Cleanup on destroy
    this.destroyRef.onDestroy(() => {
      console.log('Component destroyed');
    });
  }
}
```

### RxJS Interop with Signals
```typescript
import { toSignal, toObservable } from '@angular/core/rxjs-interop';
import { signal, effect } from '@angular/core';

@Component({...})
export class InteropComponent {
  // Observable → Signal
  private data$ = this.http.get<Data[]>('/api/data');
  data = toSignal(this.data$, { initialValue: [] });

  // Signal → Observable
  searchTerm = signal('');
  searchTerm$ = toObservable(this.searchTerm);

  // Computed with RxJS
  filteredData = computed(() =>
    this.data().filter(d => d.name.includes(this.searchTerm()))
  );

  constructor() {
    // React to signal changes with debounce
    this.searchTerm$.pipe(
      debounceTime(300),
      distinctUntilChanged(),
      switchMap(term => this.service.search(term))
    ).subscribe(results => {
      // Handle results
    });
  }
}
```

### Routing (v18 - Functional Guards)
```typescript
// app.routes.ts
import { Routes } from '@angular/router';
import { inject } from '@angular/core';

export const routes: Routes = [
  { path: '', redirectTo: '/home', pathMatch: 'full' },
  { path: 'home', loadComponent: () => import('./home.component').then(m => m.HomeComponent) },
  {
    path: 'admin',
    loadChildren: () => import('./admin/admin.routes').then(m => m.ADMIN_ROUTES),
    canActivate: [() => inject(AuthService).isAuthenticated()],
    canMatch: [() => inject(FeatureFlagService).isEnabled('admin')]
  },
  {
    path: 'detail/:id',
    loadComponent: () => import('./detail.component').then(m => m.DetailComponent),
    resolve: {
      data: (route: ActivatedRouteSnapshot) =>
        inject(DataService).getById(route.paramMap.get('id')!)
    }
  },
  { path: '**', loadComponent: () => import('./not-found.component').then(m => m.NotFoundComponent) }
];

// app.config.ts
import { ApplicationConfig, provideZoneChangeDetection } from '@angular/core';
import { provideRouter, withComponentInputBinding } from '@angular/router';
import { provideHttpClient, withInterceptors } from '@angular/common/http';
import { routes } from './app.routes';

export const appConfig: ApplicationConfig = {
  providers: [
    provideZoneChangeDetection({ eventCoalescing: true }),
    provideRouter(routes, withComponentInputBinding()),
    provideHttpClient(withInterceptors([authInterceptor])),
  ]
};
```

### HTTP with Functional Interceptors
```typescript
// auth.interceptor.ts
import { HttpInterceptorFn } from '@angular/common/http';
import { inject } from '@angular/core';

export const authInterceptor: HttpInterceptorFn = (req, next) => {
  const auth = inject(AuthService);
  const token = auth.getToken();

  if (token) {
    req = req.clone({
      setHeaders: { Authorization: `Bearer ${token}` }
    });
  }

  return next(req);
};

export const loggingInterceptor: HttpInterceptorFn = (req, next) => {
  console.log('Request:', req.url);
  return next(req).pipe(
    tap(event => {
      if (event.type === HttpEventType.Response) {
        console.log('Response:', event.status);
      }
    })
  );
};

// Register in app.config.ts
provideHttpClient(
  withInterceptors([authInterceptor, loggingInterceptor])
)
```

### Reactive Forms (v18 - Typed)
```typescript
import { Component, inject } from '@angular/core';
import { FormBuilder, ReactiveFormsModule, Validators } from '@angular/forms';

@Component({
  selector: 'app-form',
  standalone: true,
  imports: [ReactiveFormsModule],
  template: `
    <form [formGroup]="form" (ngSubmit)="onSubmit()">
      <input formControlName="name" />
      @if (form.controls.name.errors?.['required']) {
        <span class="error">Name is required</span>
      }
      <button type="submit" [disabled]="form.invalid">Submit</button>
    </form>
  `
})
export class FormComponent {
  private fb = inject(FormBuilder);

  // Typed form - infers types automatically
  form = this.fb.group({
    name: ['', [Validators.required, Validators.minLength(2)]],
    email: ['', [Validators.required, Validators.email]],
    age: [0, [Validators.min(0), Validators.max(120)]]
  });

  // Access typed values
  onSubmit() {
    if (this.form.valid) {
      const { name, email, age } = this.form.value;
      // name: string | undefined
      // email: string | undefined
      // age: number | undefined

      const rawValue = this.form.getRawValue();
      // rawValue.name: string (no undefined)
    }
  }
}
```

### Zoneless Change Detection (Experimental)
```typescript
// app.config.ts
import { provideExperimentalZonelessChangeDetection } from '@angular/core';

export const appConfig: ApplicationConfig = {
  providers: [
    provideExperimentalZonelessChangeDetection(),
    // ... other providers
  ]
};

// Components MUST use signals or call markForCheck()
@Component({
  selector: 'app-zoneless',
  standalone: true,
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div>Count: {{ count() }}</div>
    <button (click)="increment()">+</button>
  `
})
export class ZonelessComponent {
  count = signal(0);

  increment() {
    this.count.update(c => c + 1);  // Triggers change detection
  }
}
```

---

## Migration from v15/v16/v17

| Old Pattern | v18 Pattern |
|-------------|-------------|
| `@NgModule` declarations | `standalone: true` (default) |
| `*ngIf="condition"` | `@if (condition) { }` |
| `*ngFor="let item of items"` | `@for (item of items; track item.id) { }` |
| `[ngSwitch]` | `@switch (value) { @case { } }` |
| Constructor injection | `inject()` function |
| `@Input() prop: Type` | `prop = input<Type>()` |
| `@Output() event = new EventEmitter()` | `event = output<Type>()` |
| `@ViewChild()` | `viewChild()` / `viewChild.required()` |
| `@ViewChildren()` | `viewChildren()` |
| `BehaviorSubject` | `signal()` |
| Class-based interceptors | Functional interceptors |
| Class-based guards | Functional guards |

---

## CLI Commands

```bash
# Generate standalone component (default in v18)
ng generate component name           # Creates standalone component
ng generate component name --standalone=false  # NgModule component

# Generate other artifacts
ng generate service name
ng generate directive name
ng generate pipe name
ng generate guard name --functional  # Functional guard (default)

# Build & Serve
ng serve                             # Dev server :4200
ng build                             # Dev build
ng build --configuration production  # Prod build

# Migrations
ng generate @angular/core:control-flow  # Migrate *ngIf/*ngFor to @if/@for
ng update @angular/core @angular/cli    # Update Angular
```

---

## Application Bootstrap (v18)

```typescript
// main.ts
import { bootstrapApplication } from '@angular/platform-browser';
import { appConfig } from './app/app.config';
import { AppComponent } from './app/app.component';

bootstrapApplication(AppComponent, appConfig)
  .catch(err => console.error(err));

// app.config.ts
import { ApplicationConfig, provideZoneChangeDetection } from '@angular/core';
import { provideRouter } from '@angular/router';
import { provideHttpClient } from '@angular/common/http';
import { provideAnimationsAsync } from '@angular/platform-browser/animations/async';
import { routes } from './app.routes';

export const appConfig: ApplicationConfig = {
  providers: [
    provideZoneChangeDetection({ eventCoalescing: true }),
    provideRouter(routes),
    provideHttpClient(),
    provideAnimationsAsync()
  ]
};

// app.component.ts
@Component({
  selector: 'app-root',
  standalone: true,
  imports: [RouterOutlet],
  template: `<router-outlet />`
})
export class AppComponent {}
```

---

## Sources

- [Angular 18 Documentation](https://v18.angular.dev)
- [Angular v18 Release Blog](https://blog.angular.dev/angular-v18-is-now-available-e79d5ac0affe)
- [Control Flow Guide](https://angular.dev/guide/templates/control-flow)
- [Signals Guide](https://v18.angular.dev/guide/signals/overview)
- [Signal Inputs](https://v18.angular.dev/guide/signals/inputs)
- [Signal Queries](https://v18.angular.dev/guide/signals/queries)
