#!/bin/bash

# ==============================================================================
# Angular 15 → 18 Migration Script (v2.0)
# ==============================================================================
#
# A portable, reproducible script for migrating Angular projects from v15 to v18.
# Follows Angular's official upgrade path: 15 → 16 → 17 → 18
#
# Based on real-world migration experience and community best practices from:
# - Angular Update Guide (https://angular.dev/update-guide)
# - Angular Migration Schematics (https://angular.dev/reference/migrations)
#
# LESSONS LEARNED (from real-world migrations):
# 1. Third-party Angular builders must be updated BEFORE Angular packages
#    to avoid ERESOLVE peer dependency conflicts
# 2. --legacy-peer-deps is needed during mid-migration states (npm 7+ strictness)
# 3. Clean reinstall (rm -rf node_modules && npm install) resolves phantom issues
# 4. ng update can fail silently - always verify builds after each step
# 5. TypeScript version must match Angular's peer dependency exactly
# 6. Material MDC migration (v15) has significant class name changes
# 7. Typed Reactive Forms (v16+) require gradual migration
#
# PREREQUISITES:
# - Node.js ^18.19.1 || ^20.11.1 || ^22.0.0 (Angular 18 requirement)
# - npm 8+ (or yarn)
# - Git (recommended for backup branches)
#
# USAGE:
#   chmod +x migrate-to-angular18.sh
#   ./migrate-to-angular18.sh [OPTIONS]
#
# OPTIONS:
#   --dry-run          Show what would be done without making changes
#   --skip-install     Skip npm install between versions (for debugging)
#   --force            Continue despite warnings or dirty working directory
#   --from=VERSION     Start migration from specific version (15, 16, 17)
#   --clean            Force clean install at each step (slower but reliable)
#   --standalone       Run standalone components migration after upgrade
#   --control-flow     Run control flow (@if/@for) migration after upgrade
#   --help, -h         Show this help message
#
# EXAMPLES:
#   ./migrate-to-angular18.sh                    # Full migration
#   ./migrate-to-angular18.sh --dry-run          # Preview changes
#   ./migrate-to-angular18.sh --from=16          # Resume from Angular 16
#   ./migrate-to-angular18.sh --clean --force    # Reliable mode
#   ./migrate-to-angular18.sh --standalone       # Also convert to standalone
#
# ==============================================================================

set -euo pipefail

# ==============================================================================
# CONFIGURATION
# ==============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Auto-detect project root (look for package.json)
if [ -f "$SCRIPT_DIR/package.json" ]; then
  PROJECT_ROOT="$SCRIPT_DIR"
elif [ -f "$SCRIPT_DIR/../package.json" ]; then
  PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
else
  PROJECT_ROOT="$(pwd)"
fi

LOG_FILE="$PROJECT_ROOT/angular-migration-$(date +%Y%m%d-%H%M%S).log"

# Colors for output (auto-disable if not a terminal)
if [ -t 1 ]; then
  RED='\033[0;31m'
  GREEN='\033[0;32m'
  YELLOW='\033[1;33m'
  BLUE='\033[0;34m'
  CYAN='\033[0;36m'
  BOLD='\033[1m'
  NC='\033[0m'
else
  RED='' GREEN='' YELLOW='' BLUE='' CYAN='' BOLD='' NC=''
fi

# Flags
DRY_RUN=false
SKIP_INSTALL=false
FORCE=false
CLEAN_INSTALL=false
START_FROM=""
RUN_STANDALONE_MIGRATION=false
RUN_CONTROL_FLOW_MIGRATION=false

# Parse arguments
for arg in "$@"; do
  case $arg in
    --dry-run)
      DRY_RUN=true
      ;;
    --skip-install)
      SKIP_INSTALL=true
      ;;
    --force)
      FORCE=true
      ;;
    --clean)
      CLEAN_INSTALL=true
      ;;
    --from=*)
      START_FROM="${arg#*=}"
      ;;
    --standalone)
      RUN_STANDALONE_MIGRATION=true
      ;;
    --control-flow)
      RUN_CONTROL_FLOW_MIGRATION=true
      ;;
    --help|-h)
      head -55 "$0" | tail -50
      exit 0
      ;;
    *)
      echo "Unknown argument: $arg"
      echo "Use --help for usage information"
      exit 1
      ;;
  esac
done

# ==============================================================================
# UTILITY FUNCTIONS
# ==============================================================================

log() {
  local message="$1"
  local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
  echo -e "${BLUE}[$timestamp]${NC} $message" | tee -a "$LOG_FILE"
}

log_success() {
  echo -e "${GREEN}✓${NC} $1" | tee -a "$LOG_FILE"
}

log_warning() {
  echo -e "${YELLOW}⚠${NC} $1" | tee -a "$LOG_FILE"
}

log_error() {
  echo -e "${RED}✗${NC} $1" | tee -a "$LOG_FILE"
}

log_step() {
  echo -e "\n${CYAN}═══════════════════════════════════════════════════════════════════${NC}" | tee -a "$LOG_FILE"
  echo -e "${CYAN}  $1${NC}" | tee -a "$LOG_FILE"
  echo -e "${CYAN}═══════════════════════════════════════════════════════════════════${NC}\n" | tee -a "$LOG_FILE"
}

log_substep() {
  echo -e "\n${BOLD}--- $1 ---${NC}\n" | tee -a "$LOG_FILE"
}

# Run command with logging and optional failure tolerance
run_cmd() {
  local cmd="$1"
  local description="${2:-Running command}"
  local allow_fail="${3:-false}"

  log "$description"
  log "  → $cmd"

  if [ "$DRY_RUN" = true ]; then
    echo -e "  ${YELLOW}[DRY RUN] Would execute: $cmd${NC}"
    return 0
  fi

  if eval "$cmd" >> "$LOG_FILE" 2>&1; then
    log_success "$description completed"
    return 0
  else
    if [ "$allow_fail" = true ]; then
      log_warning "$description completed with warnings"
      return 0
    else
      log_error "$description failed"
      echo -e "  ${YELLOW}See $LOG_FILE for details${NC}"
      return 1
    fi
  fi
}

# Run command with automatic retries (for network operations)
run_cmd_retry() {
  local cmd="$1"
  local description="${2:-Running command}"
  local max_retries="${3:-3}"
  local retry_delay="${4:-5}"

  for ((i=1; i<=max_retries; i++)); do
    log "$description (attempt $i/$max_retries)"

    if [ "$DRY_RUN" = true ]; then
      echo -e "  ${YELLOW}[DRY RUN] Would execute: $cmd${NC}"
      return 0
    fi

    if eval "$cmd" >> "$LOG_FILE" 2>&1; then
      log_success "$description completed"
      return 0
    else
      if [ $i -lt $max_retries ]; then
        log_warning "Attempt $i failed, retrying in ${retry_delay}s..."
        sleep $retry_delay
      fi
    fi
  done

  log_error "$description failed after $max_retries attempts"
  return 1
}

check_node_version() {
  local required_major=$1
  local node_version=$(node -v 2>/dev/null | cut -d'v' -f2)

  if [ -z "$node_version" ]; then
    log_error "Node.js not found. Install Node.js $required_major+ first."
    exit 1
  fi

  local node_major=$(echo "$node_version" | cut -d'.' -f1)

  if [ "$node_major" -lt "$required_major" ]; then
    log_error "Node.js $required_major+ required for Angular 18, found v$node_version"
    log_warning "Install Node.js 18.19+ or 20.9+ before continuing"
    log_warning "Tip: Use nvm to manage Node versions: nvm install 20 && nvm use 20"
    exit 1
  fi
  log_success "Node.js v$node_version (✓ >= $required_major required)"
}

check_npm_version() {
  local npm_version=$(npm -v 2>/dev/null)

  if [ -z "$npm_version" ]; then
    log_error "npm not found"
    exit 1
  fi

  local npm_major=$(echo "$npm_version" | cut -d'.' -f1)

  if [ "$npm_major" -lt 8 ]; then
    log_warning "npm 8+ recommended, found v$npm_version"
  else
    log_success "npm v$npm_version"
  fi
}

check_git_status() {
  if ! git rev-parse --is-inside-work-tree > /dev/null 2>&1; then
    log_warning "Not a git repository - backup branches will be skipped"
    return 0
  fi

  if [ "$FORCE" = false ]; then
    if ! git diff-index --quiet HEAD -- 2>/dev/null; then
      log_error "Working directory has uncommitted changes"
      log_warning "Options:"
      log_warning "  1. Commit or stash your changes first"
      log_warning "  2. Use --force to continue anyway"
      exit 1
    fi
  fi

  log_success "Git working directory is clean"
}

create_backup_branch() {
  local version=$1
  local branch_name="pre-angular-$version-migration"

  if git rev-parse --is-inside-work-tree > /dev/null 2>&1; then
    if [ "$DRY_RUN" = true ]; then
      echo -e "  ${YELLOW}[DRY RUN] Would create branch: $branch_name${NC}"
    else
      git branch -f "$branch_name" 2>/dev/null || true
      log_success "Created backup branch: $branch_name"
    fi
  fi
}

get_current_angular_version() {
  if [ ! -f "package.json" ]; then
    echo ""
    return
  fi
  local version=$(grep '"@angular/core"' package.json 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | cut -d'.' -f1 | head -1)
  echo "$version"
}

# Detect third-party Angular builders that need updating
detect_angular_builders() {
  local builders=()

  # Common third-party builders with Angular peer dependencies
  if grep -q "@angular-builders/custom-webpack" package.json 2>/dev/null; then
    builders+=("@angular-builders/custom-webpack")
  fi
  if grep -q "@angular-builders/jest" package.json 2>/dev/null; then
    builders+=("@angular-builders/jest")
  fi
  if grep -q "@nguniversal" package.json 2>/dev/null; then
    builders+=("@nguniversal/builders")
  fi
  if grep -q "ngx-build-plus" package.json 2>/dev/null; then
    builders+=("ngx-build-plus")
  fi
  if grep -q "@angular-eslint" package.json 2>/dev/null; then
    builders+=("@angular-eslint/schematics")
  fi
  if grep -q "@ngrx/store" package.json 2>/dev/null; then
    builders+=("@ngrx/store")
  fi

  echo "${builders[*]}"
}

# Detect Angular Material for MDC migration warnings
detect_angular_material() {
  if grep -q "@angular/material" package.json 2>/dev/null; then
    return 0
  fi
  return 1
}

# Check for npm overrides that might help with peer deps
setup_npm_overrides() {
  local target_version=$1

  # Check if overrides section exists
  if ! grep -q '"overrides"' package.json 2>/dev/null; then
    log "Tip: You can add 'overrides' to package.json to force dependency versions"
    log "  Example: \"overrides\": { \"@some/package\": \"$target_version\" }"
  fi
}

# Verify Angular packages are consistent
verify_angular_packages() {
  local expected_major=$1
  log "Verifying Angular package versions..."

  if [ "$DRY_RUN" = true ]; then
    echo -e "  ${YELLOW}[DRY RUN] Would verify packages${NC}"
    return 0
  fi

  local packages=("@angular/core" "@angular/common" "@angular/compiler" "@angular/router")
  local all_match=true

  for pkg in "${packages[@]}"; do
    local version=$(npm list "$pkg" --depth=0 2>/dev/null | grep "$pkg@" | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1 | cut -d'.' -f1)
    if [ -n "$version" ] && [ "$version" != "$expected_major" ]; then
      log_warning "$pkg is at v$version, expected v$expected_major"
      all_match=false
    fi
  done

  if [ "$all_match" = true ]; then
    log_success "All Angular packages at version $expected_major"
  fi
}

# Clean install helper
clean_install() {
  log_substep "Clean Install"
  run_cmd "rm -rf node_modules package-lock.json" "Removing node_modules and lockfile"
  run_cmd_retry "npm install" "Fresh npm install" 3 10
}

# ==============================================================================
# MIGRATION FUNCTIONS
# ==============================================================================

migrate_to_version() {
  local target_version=$1
  local target_typescript=$2
  local target_zonejs=$3
  local breaking_changes=$4

  log_step "MIGRATING TO ANGULAR $target_version"

  create_backup_branch "$target_version"

  # Show breaking changes
  if [ -n "$breaking_changes" ]; then
    log "Key changes in Angular $target_version:"
    echo -e "${YELLOW}$breaking_changes${NC}" | tee -a "$LOG_FILE"
    echo "" | tee -a "$LOG_FILE"
  fi

  # =========================================================================
  # STEP 1: Update third-party Angular builders FIRST
  # =========================================================================
  # These have peer dependencies on @angular/compiler-cli and must be updated
  # before Angular packages to avoid ERESOLVE conflicts.

  log_substep "Step 1: Update Third-Party Builders"

  local builders=$(detect_angular_builders)
  if [ -n "$builders" ]; then
    for builder in $builders; do
      run_cmd "npm install ${builder}@$target_version --save-dev --legacy-peer-deps" \
        "Updating $builder to $target_version" true
    done
  else
    log_success "No third-party Angular builders detected"
  fi

  # =========================================================================
  # STEP 2: Run ng update for Angular core packages
  # =========================================================================

  log_substep "Step 2: Run Angular Update Schematics"

  local ng_update_cmd="npx ng update @angular/core@$target_version @angular/cli@$target_version --force --allow-dirty"

  if [ "$DRY_RUN" = true ]; then
    echo -e "  ${YELLOW}[DRY RUN] Would execute: $ng_update_cmd${NC}"
  else
    log "Running: $ng_update_cmd"
    if eval "$ng_update_cmd" >> "$LOG_FILE" 2>&1; then
      log_success "Angular schematics completed"
    else
      log_warning "ng update completed with warnings (often OK)"
    fi
  fi

  # =========================================================================
  # STEP 3: Update TypeScript
  # =========================================================================

  if [ -n "$target_typescript" ]; then
    log_substep "Step 3: Update TypeScript"
    run_cmd "npm install typescript@$target_typescript --save-dev --legacy-peer-deps" \
      "Updating TypeScript to $target_typescript" true
  fi

  # =========================================================================
  # STEP 4: Update Zone.js
  # =========================================================================

  if [ -n "$target_zonejs" ]; then
    log_substep "Step 4: Update Zone.js"
    run_cmd "npm install zone.js@$target_zonejs --legacy-peer-deps" \
      "Updating Zone.js to $target_zonejs" true
  fi

  # =========================================================================
  # STEP 5: Resolve dependencies
  # =========================================================================

  log_substep "Step 5: Resolve Dependencies"

  if [ "$SKIP_INSTALL" = false ]; then
    if [ "$CLEAN_INSTALL" = true ]; then
      clean_install
    else
      if ! npm install >> "$LOG_FILE" 2>&1; then
        log_warning "npm install failed, trying clean install..."
        clean_install
      else
        log_success "Dependencies resolved"
      fi
    fi
  fi

  # =========================================================================
  # STEP 6: Verify build
  # =========================================================================

  log_substep "Step 6: Verify Build"

  if [ "$DRY_RUN" = true ]; then
    echo -e "  ${YELLOW}[DRY RUN] Would verify build${NC}"
  else
    if npx ng build --configuration=development 2>> "$LOG_FILE"; then
      log_success "Build verification PASSED for Angular $target_version"
    else
      log_error "Build verification FAILED for Angular $target_version"
      log_warning "Check $LOG_FILE for details"
      log_warning "Common fixes:"
      log_warning "  1. Fix TypeScript errors shown in the log"
      log_warning "  2. Update deprecated API usage"
      log_warning "  3. Run: rm -rf node_modules && npm install"

      if [ "$FORCE" = false ]; then
        log_error "Stopping migration. Use --force to continue."
        exit 1
      fi
    fi
  fi

  verify_angular_packages "$target_version"
  log_success "Migration to Angular $target_version complete!"
}

# ==============================================================================
# CODE PATTERN CHECKS
# ==============================================================================

check_deprecated_patterns() {
  log_step "CHECKING FOR DEPRECATED PATTERNS"

  log "Scanning source code for patterns that may need updates..."

  local src_dir="$PROJECT_ROOT/src"
  local issues_found=false

  if [ ! -d "$src_dir" ]; then
    log_warning "src directory not found - skipping code analysis"
    return 0
  fi

  # Class-based guards (deprecated in 15.2, functional guards preferred)
  log "Checking for class-based route guards..."
  if grep -rE "implements\s+(CanActivate|CanDeactivate|CanLoad|Resolve)" --include="*.ts" "$src_dir" 2>/dev/null | head -3; then
    log_warning "Found class-based guards - convert to functional guards"
    log_warning "  See: https://angular.dev/guide/routing/route-guards"
    issues_found=true
  else
    log_success "No class-based guards found"
  fi

  # ComponentFactoryResolver (removed in Ivy)
  log "Checking for ComponentFactoryResolver..."
  if grep -r "ComponentFactoryResolver" --include="*.ts" "$src_dir" 2>/dev/null | head -3; then
    log_warning "Found ComponentFactoryResolver - use ViewContainerRef.createComponent()"
    issues_found=true
  else
    log_success "No ComponentFactoryResolver found"
  fi

  # .toPromise() (deprecated in RxJS 7)
  log "Checking for deprecated RxJS patterns..."
  if grep -rE "\.toPromise\(\)" --include="*.ts" "$src_dir" 2>/dev/null | head -3; then
    log_warning "Found .toPromise() - use firstValueFrom() or lastValueFrom()"
    issues_found=true
  else
    log_success "No deprecated RxJS patterns found"
  fi

  # Untyped FormControl/FormGroup (typed forms in v16+)
  log "Checking for untyped reactive forms..."
  if grep -rE "new FormControl\([^<]|new FormGroup\([^<]" --include="*.ts" "$src_dir" 2>/dev/null | grep -v "UntypedForm" | head -3; then
    log_warning "Found untyped forms - consider using typed FormControl<T> (Angular 16+)"
    log_warning "  Or use UntypedFormControl for gradual migration"
    issues_found=true
  else
    log_success "No untyped forms issues found"
  fi

  # CommonJS requires in app code (problematic for ESM builds)
  log "Checking for CommonJS patterns..."
  if grep -rE "require\(|__dirname|__filename" --include="*.ts" "$src_dir" 2>/dev/null | grep -v "node_modules" | head -3; then
    log_warning "Found CommonJS patterns - convert to ESM imports"
    log_warning "  Angular 17+ uses esbuild which requires ESM compatibility"
    issues_found=true
  else
    log_success "No CommonJS patterns found"
  fi

  # Old *ngIf/*ngFor (deprecated, will be auto-migrated in Angular 20)
  log "Checking for structural directives..."
  local ngif_count=$(grep -r "\*ngIf" --include="*.html" "$src_dir" 2>/dev/null | wc -l | tr -d ' ')
  local ngfor_count=$(grep -r "\*ngFor" --include="*.html" "$src_dir" 2>/dev/null | wc -l | tr -d ' ')
  if [ "$ngif_count" -gt 0 ] || [ "$ngfor_count" -gt 0 ]; then
    log_warning "Found $ngif_count *ngIf and $ngfor_count *ngFor usages"
    log_warning "  Consider migrating to @if/@for with: ng g @angular/core:control-flow"
  else
    log_success "Already using new control flow or no templates found"
  fi

  # Angular Material MDC class changes
  if detect_angular_material; then
    log "Checking for old Material CSS classes..."
    if grep -rE "\.mat-[a-z]+-[a-z]+" --include="*.scss" --include="*.css" "$src_dir" 2>/dev/null | grep -v "mat-mdc-" | head -3; then
      log_warning "Found old Material classes - may need MDC migration"
      log_warning "  Classes changed from mat-* to mat-mdc-* in Material 15+"
      issues_found=true
    else
      log_success "Material CSS classes look compatible"
    fi
  fi

  # Suggestions
  echo "" | tee -a "$LOG_FILE"
  log "Optional Angular 18 modernizations:"
  echo -e "  ${CYAN}•${NC} New control flow: @if/@for instead of *ngIf/*ngFor" | tee -a "$LOG_FILE"
  echo -e "  ${CYAN}•${NC} Signal inputs: input<T>() instead of @Input()" | tee -a "$LOG_FILE"
  echo -e "  ${CYAN}•${NC} Signal outputs: output<T>() instead of @Output()" | tee -a "$LOG_FILE"
  echo -e "  ${CYAN}•${NC} takeUntilDestroyed() from @angular/core/rxjs-interop" | tee -a "$LOG_FILE"
  echo -e "  ${CYAN}•${NC} Standalone components: ng g @angular/core:standalone" | tee -a "$LOG_FILE"
  echo -e "  ${CYAN}•${NC} Control flow migration: ng g @angular/core:control-flow" | tee -a "$LOG_FILE"

  if [ "$issues_found" = false ]; then
    echo "" | tee -a "$LOG_FILE"
    log_success "No critical issues found!"
  fi
}

# ==============================================================================
# OPTIONAL SCHEMATICS MIGRATIONS
# ==============================================================================

run_standalone_migration() {
  log_step "RUNNING STANDALONE COMPONENTS MIGRATION"

  log "This will convert components, directives, and pipes to standalone."
  log "See: https://angular.dev/reference/migrations/standalone"
  echo "" | tee -a "$LOG_FILE"

  if [ "$DRY_RUN" = true ]; then
    echo -e "  ${YELLOW}[DRY RUN] Would run standalone migration schematics${NC}"
    return 0
  fi

  # Step 1: Convert all declarations to standalone
  log_substep "Step 1: Convert declarations to standalone"
  if npx ng generate @angular/core:standalone --mode=convert-to-standalone 2>&1 | tee -a "$LOG_FILE"; then
    log_success "Converted declarations to standalone"
  else
    log_warning "Standalone conversion had issues - check manually"
  fi

  # Step 2: Remove unnecessary NgModules
  log_substep "Step 2: Remove unnecessary NgModules"
  if npx ng generate @angular/core:standalone --mode=prune-modules 2>&1 | tee -a "$LOG_FILE"; then
    log_success "Removed unnecessary NgModules"
  else
    log_warning "Module pruning had issues - check manually"
  fi

  # Step 3: Bootstrap with standalone API
  log_substep "Step 3: Switch to standalone bootstrap"
  if npx ng generate @angular/core:standalone --mode=standalone-bootstrap 2>&1 | tee -a "$LOG_FILE"; then
    log_success "Switched to standalone bootstrap"
  else
    log_warning "Bootstrap migration had issues - check manually"
  fi

  # Verify build after standalone migration
  log_substep "Verifying build after standalone migration"
  if npx ng build --configuration=development 2>> "$LOG_FILE"; then
    log_success "Build passes after standalone migration"
  else
    log_error "Build failed after standalone migration - manual fixes needed"
  fi
}

run_control_flow_migration() {
  log_step "RUNNING CONTROL FLOW MIGRATION (@if, @for, @switch)"

  log "This will convert *ngIf/*ngFor/*ngSwitch to @if/@for/@switch."
  log "See: https://angular.dev/reference/migrations/control-flow"
  echo "" | tee -a "$LOG_FILE"

  if [ "$DRY_RUN" = true ]; then
    echo -e "  ${YELLOW}[DRY RUN] Would run control flow migration schematic${NC}"
    return 0
  fi

  if npx ng generate @angular/core:control-flow 2>&1 | tee -a "$LOG_FILE"; then
    log_success "Control flow migration completed"
  else
    log_warning "Control flow migration had issues - some templates may need manual fixes"
  fi

  # Verify build
  log "Verifying build after control flow migration..."
  if npx ng build --configuration=development 2>> "$LOG_FILE"; then
    log_success "Build passes after control flow migration"
  else
    log_error "Build failed after control flow migration - manual fixes needed"
  fi
}

# ==============================================================================
# FINALIZATION
# ==============================================================================

finalize_migration() {
  log_step "FINALIZING MIGRATION"

  log "Checking for remaining updates..."
  if [ "$DRY_RUN" = false ]; then
    npx ng update 2>&1 | tee -a "$LOG_FILE" || true
  fi

  log_substep "Final Clean Install"
  if [ "$SKIP_INSTALL" = false ]; then
    clean_install
  fi

  log_substep "Production Build Verification"
  if [ "$DRY_RUN" = true ]; then
    echo -e "  ${YELLOW}[DRY RUN] Would verify production build${NC}"
  else
    if npx ng build 2>> "$LOG_FILE"; then
      log_success "Production build PASSED!"
    else
      log_warning "Production build has issues - check $LOG_FILE"
    fi
  fi
}

# ==============================================================================
# MAIN EXECUTION
# ==============================================================================

main() {
  echo ""
  echo "╔══════════════════════════════════════════════════════════════════╗"
  echo "║          ANGULAR 15 → 18 MIGRATION SCRIPT (v2.0)                 ║"
  echo "╠══════════════════════════════════════════════════════════════════╣"
  echo "║  Migrates Angular projects: 15 → 16 → 17 → 18                    ║"
  echo "║                                                                  ║"
  echo "║  Features:                                                       ║"
  echo "║  • Auto-detects third-party builders (custom-webpack, jest...)   ║"
  echo "║  • Handles ERESOLVE peer dependency conflicts                    ║"
  echo "║  • Retries failed network operations (3 attempts)                ║"
  echo "║  • Creates backup branches at each step                          ║"
  echo "║  • Verifies builds after each migration                          ║"
  echo "║  • Checks for deprecated patterns and suggests fixes             ║"
  echo "║  • Optional: --standalone to convert to standalone components    ║"
  echo "║  • Optional: --control-flow to migrate to @if/@for syntax        ║"
  echo "╚══════════════════════════════════════════════════════════════════╝"
  echo ""

  if [ "$DRY_RUN" = true ]; then
    echo -e "${YELLOW}DRY RUN MODE - no changes will be made${NC}\n"
  fi

  cd "$PROJECT_ROOT"

  # Verify we're in an Angular project
  if [ ! -f "package.json" ]; then
    log_error "No package.json found in $PROJECT_ROOT"
    log_error "Run this script from your Angular project directory"
    exit 1
  fi

  if ! grep -q "@angular/core" package.json; then
    log_error "This doesn't appear to be an Angular project"
    exit 1
  fi

  # Initialize log
  echo "Angular Migration Log - $(date)" > "$LOG_FILE"
  echo "Project: $PROJECT_ROOT" >> "$LOG_FILE"
  echo "Options: DRY_RUN=$DRY_RUN FORCE=$FORCE CLEAN=$CLEAN_INSTALL FROM=$START_FROM" >> "$LOG_FILE"
  echo "" >> "$LOG_FILE"

  # Pre-flight checks
  log_step "PRE-FLIGHT CHECKS"

  check_node_version 18
  check_npm_version
  check_git_status

  local current_version=$(get_current_angular_version)

  if [ -z "$current_version" ]; then
    log_error "Could not detect Angular version from package.json"
    exit 1
  fi

  log "Detected Angular version: $current_version"

  # Handle --from flag
  if [ -n "$START_FROM" ]; then
    log "Using --from=$START_FROM to set starting version"
    current_version=$START_FROM
  fi

  # Validate
  if [ "$current_version" = "18" ]; then
    log_success "Already on Angular 18 - nothing to do!"
    exit 0
  fi

  if [[ ! "$current_version" =~ ^(15|16|17)$ ]]; then
    log_warning "Unexpected Angular version: $current_version"
    if [ "$FORCE" = false ]; then
      log_error "Use --force to continue, or --from=VERSION to specify version"
      exit 1
    fi
  fi

  create_backup_branch "$current_version-original"

  # ===========================================================================
  # MIGRATION STEPS
  # ===========================================================================

  if [ "$current_version" = "15" ] || [ "$current_version" -lt "16" ]; then
    migrate_to_version "16" "~5.1.0" "~0.13.0" "
  • Standalone components become first-class
  • Required inputs: required: true
  • Signals introduced (preview)
  • DestroyRef for takeUntilDestroyed()
  • Functional guards by default
  "
  fi

  if [ "$current_version" -lt "17" ]; then
    migrate_to_version "17" "~5.2.0" "~0.14.0" "
  • New control flow: @if, @for, @switch
  • Deferrable views: @defer
  • Signals become stable
  • esbuild-based builder as default
  "
  fi

  if [ "$current_version" -lt "18" ]; then
    migrate_to_version "18" "~5.4.0" "~0.14.0" "
  • Zoneless change detection (experimental)
  • Signal-based inputs, outputs, queries (stable)
  • @let template syntax
  • Fallback content for ng-content
  "
  fi

  # ===========================================================================
  # POST-MIGRATION
  # ===========================================================================

  check_deprecated_patterns

  # Optional: Run standalone migration if requested
  if [ "$RUN_STANDALONE_MIGRATION" = true ]; then
    run_standalone_migration
  fi

  # Optional: Run control flow migration if requested
  if [ "$RUN_CONTROL_FLOW_MIGRATION" = true ]; then
    run_control_flow_migration
  fi

  finalize_migration

  # ===========================================================================
  # SUMMARY
  # ===========================================================================

  log_step "MIGRATION COMPLETE! 🎉"

  local final_version=$(get_current_angular_version)

  echo ""
  echo "╔══════════════════════════════════════════════════════════════════╗"
  echo "║                      MIGRATION SUMMARY                           ║"
  echo "╠══════════════════════════════════════════════════════════════════╣"
  echo "║  ✓ Angular $current_version → Angular $final_version                                     ║"
  echo "║  ✓ Log: ${LOG_FILE##*/}                     ║"
  echo "║                                                                  ║"
  echo "║  VERIFICATION CHECKLIST:                                         ║"
  echo "║  □ npm test           Run unit tests                             ║"
  echo "║  □ npm run e2e        Run E2E tests (if configured)              ║"
  echo "║  □ npm start          Test dev server                            ║"
  echo "║  □ npm run build      Verify production build                    ║"
  echo "║                                                                  ║"
  echo "║  OPTIONAL MODERNIZATIONS:                                        ║"
  echo "║  □ ng g @angular/core:standalone    Convert to standalone        ║"
  echo "║  □ ng g @angular/core:control-flow  Migrate to @if/@for          ║"
  echo "║  □ ng g @angular/core:inject        Use inject() function        ║"
  echo "║                                                                  ║"
  echo "║  ROLLBACK (if needed):                                           ║"
  echo "║    git checkout pre-angular-$current_version-original-migration              ║"
  echo "║    rm -rf node_modules && npm install                            ║"
  echo "╚══════════════════════════════════════════════════════════════════╝"
  echo ""

  log_success "Full log: $LOG_FILE"
}

main
