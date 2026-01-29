# AGENTS_15.md - Angular 15 Documentation Index

<!-- ANGULAR-AGENTS-MD-START -->[Angular 15 Docs Index]|root: https://v15.angular.io|IMPORTANT: Prefer retrieval-led reasoning over pre-training-led reasoning for any Angular 15 tasks. This is Angular 15.x with NgModule-based architecture - NOT standalone components or signals.|getting-started:{what-is-angular,setup-local,first-app}|understanding-angular/components:{overview,lifecycle,view-encapsulation,component-interaction,component-styles,sharing-data,content-projection,dynamic-components,angular-elements}|understanding-angular/templates:{interpolation,property-binding,attribute-binding,class-binding,style-binding,event-binding,two-way-binding,template-variables,svg-in-templates}|understanding-angular/directives:{built-in-directives,attribute-directives,structural-directives,directive-composition-api}|understanding-angular/dependency-injection:{dependency-injection,creating-injectable-service,defining-dependency-providers,injection-context,hierarchical-dependency-injection}|developer-guides/standalone:{standalone-components}|developer-guides/change-detection:{change-detection,zone-js,slow-computations,skipping-subtrees,resolving-zone-pollution}|developer-guides/routing:{router,common-routing-tasks,router-tutorial,routing-with-urlmatcher,feature-modules,lazy-loading,preloading,router-reference}|developer-guides/forms:{forms-overview,reactive-forms,template-driven-forms,form-validation,building-dynamic-forms}|developer-guides/http:{http,http-setup-server-communication,http-request-data,http-handle-request-errors,http-interceptors,http-pass-metadata,http-track-show-request-progress,http-make-jsonp-request,http-test-requests}|developer-guides/testing:{testing,code-coverage,testing-services,testing-components-basics,testing-components-scenarios,testing-attribute-directives,testing-pipes,debugging-tests,testing-utility-apis}|developer-guides/i18n:{i18n-overview,i18n-common-prepare,i18n-common-translation,i18n-common-merge,i18n-common-deploy,i18n-optional-runtime-source-locale,i18n-optional-manage-marked-text,i18n-optional-import-format}|developer-guides/animations:{animations,animation-transitions-triggers,complex-sequences,reusable-animations,route-animations}|developer-guides/pwa:{service-worker-intro,app-shell,service-worker-communications,service-worker-devops,service-worker-config}|best-practices:{security,accessibility,updating,property-binding-best-practices,lazy-loading-ngmodules,lightweight-injection-tokens}|cli:{cli,workspace-config,ng-generate,ng-build,ng-serve,ng-test}|reference:{glossary,api,cli-ref,error-reference,extended-diagnostics,style-guide}<!-- ANGULAR-AGENTS-MD-END -->

---

## Angular 15 Architecture Overview

Angular 15 uses **NgModule-based architecture** as the primary pattern. Standalone components exist but are opt-in, not default.

### Module Hierarchy
```
AppModule (root)
├── CoreModule (singleton services, guards, interceptors)
├── SharedModule (reusable components, pipes, directives)
└── FeatureModules (lazy-loaded domain modules)
```

---

## Core Patterns

### NgModule (Required in v15)
```typescript
// feature.module.ts
@NgModule({
  declarations: [FeatureComponent, FeaturePipe],
  imports: [CommonModule, SharedModule, FeatureRoutingModule],
  providers: [FeatureService],
  exports: [FeatureComponent]  // If used outside module
})
export class FeatureModule { }
```

### Component
```typescript
@Component({
  selector: 'app-feature',
  templateUrl: './feature.component.html',
  styleUrls: ['./feature.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush
})
export class FeatureComponent implements OnInit, OnDestroy {
  @Input() data!: DataType;
  @Output() action = new EventEmitter<ActionType>();

  constructor(private service: FeatureService) {}
  ngOnInit(): void {}
  ngOnDestroy(): void {}
}
```

### Template Syntax
```html
<!-- Interpolation -->
{{ expression }}

<!-- Property binding -->
<img [src]="imageUrl" [alt]="imageAlt">

<!-- Event binding -->
<button (click)="onClick($event)">Click</button>

<!-- Two-way binding -->
<input [(ngModel)]="value">

<!-- *ngIf with else -->
<div *ngIf="condition; else elseBlock">True</div>
<ng-template #elseBlock>False</ng-template>

<!-- *ngFor with trackBy -->
<li *ngFor="let item of items; trackBy: trackById; let i = index">
  {{ i }}: {{ item.name }}
</li>

<!-- ngSwitch -->
<div [ngSwitch]="status">
  <span *ngSwitchCase="'active'">Active</span>
  <span *ngSwitchCase="'pending'">Pending</span>
  <span *ngSwitchDefault>Unknown</span>
</div>

<!-- ng-container (no DOM element) -->
<ng-container *ngIf="condition">
  <span>Content</span>
</ng-container>

<!-- ng-template -->
<ng-template #tpl let-data="data">
  {{ data.value }}
</ng-template>
<ng-container *ngTemplateOutlet="tpl; context: {data: item}"></ng-container>
```

### Dependency Injection
```typescript
// Tree-shakable service
@Injectable({ providedIn: 'root' })
export class DataService {
  constructor(private http: HttpClient) {}
}

// Module-scoped service
@Injectable()  // Must be in providers array
export class FeatureScopedService {}

// Injection token
export const CONFIG = new InjectionToken<AppConfig>('app.config');

// Injection in constructor
constructor(
  private service: DataService,
  @Optional() @SkipSelf() private parent: ParentService,
  @Inject(CONFIG) private config: AppConfig
) {}
```

### Routing
```typescript
// app-routing.module.ts
const routes: Routes = [
  { path: '', redirectTo: '/home', pathMatch: 'full' },
  { path: 'home', component: HomeComponent },
  { path: 'detail/:id', component: DetailComponent },
  {
    path: 'admin',
    loadChildren: () => import('./admin/admin.module').then(m => m.AdminModule),
    canActivate: [AuthGuard],
    canLoad: [AuthGuard]
  },
  { path: '**', component: NotFoundComponent }
];

@NgModule({
  imports: [RouterModule.forRoot(routes)],
  exports: [RouterModule]
})
export class AppRoutingModule {}
```

### Route Guards (Class-based in v15)
```typescript
@Injectable({ providedIn: 'root' })
export class AuthGuard implements CanActivate, CanLoad {
  constructor(private auth: AuthService, private router: Router) {}

  canActivate(route: ActivatedRouteSnapshot, state: RouterStateSnapshot): boolean {
    return this.checkAuth(state.url);
  }

  canLoad(route: Route): boolean {
    return this.checkAuth(`/${route.path}`);
  }

  private checkAuth(url: string): boolean {
    if (this.auth.isAuthenticated()) return true;
    this.router.navigate(['/login'], { queryParams: { returnUrl: url }});
    return false;
  }
}
```

### Reactive Forms
```typescript
@Component({...})
export class FormComponent implements OnInit {
  form!: FormGroup;

  constructor(private fb: FormBuilder) {}

  ngOnInit() {
    this.form = this.fb.group({
      name: ['', [Validators.required, Validators.minLength(2)]],
      email: ['', [Validators.required, Validators.email]],
      password: ['', [Validators.required, Validators.pattern(/^(?=.*\d)(?=.*[a-z]).{8,}$/)]],
      addresses: this.fb.array([this.createAddress()])
    });
  }

  createAddress(): FormGroup {
    return this.fb.group({ street: '', city: '', zip: '' });
  }

  get addresses(): FormArray {
    return this.form.get('addresses') as FormArray;
  }

  onSubmit() {
    if (this.form.valid) {
      console.log(this.form.value);
    } else {
      this.form.markAllAsTouched();
    }
  }
}
```

### HTTP Client
```typescript
@Injectable({ providedIn: 'root' })
export class ApiService {
  private baseUrl = '/api';

  constructor(private http: HttpClient) {}

  getAll<T>(endpoint: string): Observable<T[]> {
    return this.http.get<T[]>(`${this.baseUrl}/${endpoint}`);
  }

  getById<T>(endpoint: string, id: string | number): Observable<T> {
    return this.http.get<T>(`${this.baseUrl}/${endpoint}/${id}`);
  }

  create<T>(endpoint: string, data: Partial<T>): Observable<T> {
    return this.http.post<T>(`${this.baseUrl}/${endpoint}`, data);
  }

  update<T>(endpoint: string, id: string | number, data: Partial<T>): Observable<T> {
    return this.http.put<T>(`${this.baseUrl}/${endpoint}/${id}`, data);
  }

  delete(endpoint: string, id: string | number): Observable<void> {
    return this.http.delete<void>(`${this.baseUrl}/${endpoint}/${id}`);
  }
}
```

### HTTP Interceptor
```typescript
@Injectable()
export class AuthInterceptor implements HttpInterceptor {
  constructor(private auth: AuthService) {}

  intercept(req: HttpRequest<any>, next: HttpHandler): Observable<HttpEvent<any>> {
    const token = this.auth.getToken();

    const authReq = token
      ? req.clone({ setHeaders: { Authorization: `Bearer ${token}` }})
      : req;

    return next.handle(authReq).pipe(
      catchError((error: HttpErrorResponse) => {
        if (error.status === 401) {
          this.auth.logout();
        }
        return throwError(() => error);
      })
    );
  }
}

// Register in module
providers: [
  { provide: HTTP_INTERCEPTORS, useClass: AuthInterceptor, multi: true }
]
```

### RxJS Subscription Management
```typescript
@Component({...})
export class DataComponent implements OnInit, OnDestroy {
  private destroy$ = new Subject<void>();
  items$!: Observable<Item[]>;

  constructor(private service: DataService) {}

  ngOnInit() {
    // Prefer async pipe
    this.items$ = this.service.getItems();

    // Manual subscription with cleanup
    this.service.getUpdates().pipe(
      takeUntil(this.destroy$),
      distinctUntilChanged(),
      debounceTime(300)
    ).subscribe(update => this.handleUpdate(update));
  }

  ngOnDestroy() {
    this.destroy$.next();
    this.destroy$.complete();
  }
}
```

### Change Detection
```typescript
@Component({
  changeDetection: ChangeDetectionStrategy.OnPush  // Recommended
})
export class OptimizedComponent {
  constructor(private cdr: ChangeDetectorRef) {}

  // Manual trigger when needed
  triggerCheck() {
    this.cdr.markForCheck();  // Marks path to root for check
  }

  detectChanges() {
    this.cdr.detectChanges();  // Runs change detection on this view
  }
}
```

---

## Key Angular 15 Features

|Feature|Status|Pattern|
|-------|------|-------|
|Standalone components|Opt-in|`standalone: true` in decorator|
|NgModule|Default|Required for most apps|
|Typed forms|Stable|`FormControl<T>` with strict typing|
|Image directive|Stable|`NgOptimizedImage`|
|Router|Stable|Class-based guards|
|RxJS|7.x|Standard reactive patterns|
|Zone.js|Required|Default change detection|

---

## CLI Commands

```bash
# Generate
ng generate component path/name    # --skip-tests --inline-style
ng generate service path/name      # --skip-tests
ng generate module path/name       # --routing
ng generate guard path/name        # --implements CanActivate
ng generate pipe path/name
ng generate directive path/name
ng generate class path/name
ng generate interface path/name
ng generate enum path/name

# Build & Serve
ng serve                           # Dev server :4200
ng serve --port 4300 --open        # Custom port, open browser
ng build                           # Dev build
ng build --configuration production # Prod build with optimizations

# Test
ng test                            # Karma unit tests
ng test --code-coverage            # With coverage report
ng e2e                             # Protractor e2e (deprecated in v15)

# Analysis
ng build --stats-json
npx webpack-bundle-analyzer dist/*/stats.json
```

---

## Angular 15 → 16+ Migration Notes

|v15 Pattern|v16+ Pattern|
|-----------|------------|
|`@NgModule`|`standalone: true` (default)|
|`*ngIf`|`@if { } @else { }`|
|`*ngFor`|`@for (item of items; track item.id) { }`|
|`[ngSwitch]`|`@switch (value) { @case (...) { } }`|
|Constructor DI|`inject()` function|
|BehaviorSubject|`signal()`, `computed()`|
|Class guards|Functional guards|

---

## Sources

- [Angular 15 Documentation](https://v15.angular.io/docs)
- [Angular 15 API Reference](https://v15.angular.io/api)
- [Angular 15 CLI Reference](https://v15.angular.io/cli)
- [Angular Update Guide](https://update.angular.io/)
