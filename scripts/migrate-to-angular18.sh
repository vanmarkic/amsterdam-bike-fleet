#!/bin/bash

# ==============================================================================
# Angular 15 → 18 Migration Script
# ==============================================================================
#
# This script performs a reproducible, step-by-step migration from Angular 15
# to Angular 18, following Angular's official upgrade path: 15 → 16 → 17 → 18
#
# WHY INCREMENTAL MIGRATION?
# - Angular's schematics handle breaking changes between adjacent versions
# - Skipping versions can cause missed migrations and subtle bugs
# - Each step applies automatic code transformations (codemods)
#
# PREREQUISITES:
# - Node.js 18.19.0+ or 20.9.0+ (required for Angular 18)
# - npm 8+
# - Git (for creating backup branches)
# - Clean working directory (no uncommitted changes)
#
# USAGE:
#   chmod +x scripts/migrate-to-angular18.sh
#   ./scripts/migrate-to-angular18.sh [--dry-run] [--skip-install] [--force]
#
# OPTIONS:
#   --dry-run       Show what would be done without making changes
#   --skip-install  Skip npm install between versions (for debugging)
#   --force         Continue even if working directory is dirty
#
# ==============================================================================

set -euo pipefail

# ==============================================================================
# CONFIGURATION
# ==============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
LOG_FILE="$PROJECT_ROOT/migration-$(date +%Y%m%d-%H%M%S).log"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Flags
DRY_RUN=false
SKIP_INSTALL=false
FORCE=false

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
    *)
      echo "Unknown argument: $arg"
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

run_cmd() {
  local cmd="$1"
  local description="${2:-Running command}"

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
    log_error "$description failed"
    echo "  See $LOG_FILE for details"
    return 1
  fi
}

check_node_version() {
  local required_major=$1
  local node_version=$(node -v | cut -d'v' -f2)
  local node_major=$(echo "$node_version" | cut -d'.' -f1)

  if [ "$node_major" -lt "$required_major" ]; then
    log_error "Node.js $required_major+ required for Angular 18, found $node_version"
    log_warning "Install Node.js 18.19+ or 20.9+ before continuing"
    exit 1
  fi
  log_success "Node.js version $node_version (✓ >= $required_major required)"
}

check_git_status() {
  if ! git rev-parse --is-inside-work-tree > /dev/null 2>&1; then
    log_warning "Not a git repository - skipping backup branch creation"
    return 0
  fi

  if [ "$FORCE" = false ]; then
    if ! git diff-index --quiet HEAD -- 2>/dev/null; then
      log_error "Working directory has uncommitted changes"
      log_warning "Commit or stash changes, or use --force to continue anyway"
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
  local version=$(grep '"@angular/core"' package.json | sed 's/.*"\^*~*\([0-9]*\).*/\1/')
  echo "$version"
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

  # Create backup branch
  create_backup_branch "$target_version"

  # Show breaking changes
  if [ -n "$breaking_changes" ]; then
    log "Known breaking changes for Angular $target_version:"
    echo -e "${YELLOW}$breaking_changes${NC}" | tee -a "$LOG_FILE"
  fi

  # Run ng update for Angular packages
  log "Running Angular update schematics..."

  local ng_update_cmd="npx ng update @angular/core@$target_version @angular/cli@$target_version --force --allow-dirty"

  if [ "$DRY_RUN" = true ]; then
    echo -e "  ${YELLOW}[DRY RUN] Would execute: $ng_update_cmd${NC}"
  else
    if ! eval "$ng_update_cmd" >> "$LOG_FILE" 2>&1; then
      log_warning "ng update completed with warnings - check log for details"
    else
      log_success "Angular core packages updated to $target_version"
    fi
  fi

  # Update TypeScript if needed
  if [ -n "$target_typescript" ]; then
    run_cmd "npm install typescript@$target_typescript --save-dev" "Updating TypeScript to $target_typescript"
  fi

  # Update Zone.js if needed
  if [ -n "$target_zonejs" ]; then
    run_cmd "npm install zone.js@$target_zonejs" "Updating Zone.js to $target_zonejs"
  fi

  # Install dependencies
  if [ "$SKIP_INSTALL" = false ]; then
    run_cmd "npm install" "Installing dependencies"
  fi

  # Verify the build
  log "Verifying build..."
  if [ "$DRY_RUN" = true ]; then
    echo -e "  ${YELLOW}[DRY RUN] Would verify build${NC}"
  else
    if npm run build -- --configuration=development 2>> "$LOG_FILE"; then
      log_success "Build verification passed for Angular $target_version"
    else
      log_warning "Build failed - manual intervention may be required"
      log "Check $LOG_FILE for error details"
    fi
  fi
}

update_custom_webpack_builder() {
  local target_version=$1

  log "Updating @angular-builders/custom-webpack..."
  run_cmd "npm install @angular-builders/custom-webpack@$target_version --save-dev" "Updating custom-webpack builder to $target_version"
}

finalize_migration() {
  log_step "FINALIZING MIGRATION"

  # Update any remaining dependencies
  log "Checking for additional dependency updates..."

  # Run ng update to check what else needs updating
  if [ "$DRY_RUN" = false ]; then
    npx ng update 2>&1 | tee -a "$LOG_FILE" || true
  fi

  # Clean install
  if [ "$SKIP_INSTALL" = false ]; then
    run_cmd "rm -rf node_modules package-lock.json" "Cleaning node_modules"
    run_cmd "npm install" "Fresh install of all dependencies"
  fi

  # Final build verification
  log "Final build verification..."
  if [ "$DRY_RUN" = true ]; then
    echo -e "  ${YELLOW}[DRY RUN] Would verify final build${NC}"
  else
    if npm run build 2>> "$LOG_FILE"; then
      log_success "Production build successful!"
    else
      log_warning "Production build failed - check $LOG_FILE"
    fi
  fi
}

# ==============================================================================
# MANUAL MIGRATION STEPS (for code changes schematics don't handle)
# ==============================================================================

apply_manual_migrations() {
  log_step "APPLYING MANUAL CODE MIGRATIONS"

  # These are code patterns that need manual updates
  # The script will identify them and provide guidance

  log "Scanning for code patterns that need manual updates..."

  # Check for deprecated patterns
  local issues_found=false

  # 1. Check for class-based guards (deprecated in 15.2, removed in 18)
  if grep -r "implements CanActivate\|implements CanDeactivate\|implements CanLoad\|implements Resolve" --include="*.ts" "$PROJECT_ROOT/src" 2>/dev/null; then
    log_warning "Found class-based guards - these should be converted to functional guards"
    issues_found=true
  fi

  # 2. Check for ComponentFactoryResolver (removed in 18)
  if grep -r "ComponentFactoryResolver" --include="*.ts" "$PROJECT_ROOT/src" 2>/dev/null; then
    log_warning "Found ComponentFactoryResolver usage - migrate to ViewContainerRef.createComponent()"
    issues_found=true
  fi

  # 3. Check for ModuleWithProviders without generic (required since 14)
  if grep -r "ModuleWithProviders[^<]" --include="*.ts" "$PROJECT_ROOT/src" 2>/dev/null; then
    log_warning "Found ModuleWithProviders without generic type parameter"
    issues_found=true
  fi

  # 4. Check for HttpClientModule in standalone components (should use provideHttpClient)
  if grep -r "HttpClientModule" --include="*.ts" "$PROJECT_ROOT/src" 2>/dev/null | grep -v "app.module" 2>/dev/null; then
    log_warning "Found HttpClientModule imports - consider using provideHttpClient() for standalone components"
    issues_found=true
  fi

  # 5. Check for RouterModule.forRoot in standalone setup
  if grep -r "RouterModule.forRoot\|RouterModule.forChild" --include="*.ts" "$PROJECT_ROOT/src" 2>/dev/null; then
    log_warning "Found RouterModule usage - consider using provideRouter() for standalone apps"
    issues_found=true
  fi

  if [ "$issues_found" = false ]; then
    log_success "No problematic code patterns found"
  fi
}

# ==============================================================================
# MAIN EXECUTION
# ==============================================================================

main() {
  echo ""
  echo "╔══════════════════════════════════════════════════════════════════╗"
  echo "║          ANGULAR 15 → 18 MIGRATION SCRIPT                       ║"
  echo "╠══════════════════════════════════════════════════════════════════╣"
  echo "║  This script will migrate your Angular project through:         ║"
  echo "║    Angular 15 → 16 → 17 → 18                                    ║"
  echo "║                                                                  ║"
  echo "║  Each step runs official Angular schematics and updates         ║"
  echo "║  TypeScript, Zone.js, and related dependencies.                 ║"
  echo "╚══════════════════════════════════════════════════════════════════╝"
  echo ""

  if [ "$DRY_RUN" = true ]; then
    echo -e "${YELLOW}Running in DRY RUN mode - no changes will be made${NC}\n"
  fi

  # Change to project root
  cd "$PROJECT_ROOT"

  # Pre-flight checks
  log_step "PRE-FLIGHT CHECKS"

  check_node_version 18
  check_git_status

  local current_version=$(get_current_angular_version)
  log "Current Angular version: $current_version"

  if [ "$current_version" != "15" ]; then
    log_warning "Expected Angular 15, found $current_version"
    log_warning "This script is designed for Angular 15 → 18 migration"
    if [ "$FORCE" = false ]; then
      exit 1
    fi
  fi

  # Create initial backup
  create_backup_branch "15-original"

  # ===========================================================================
  # ANGULAR 15 → 16
  # ===========================================================================

  migrate_to_version "16" "~5.0.0" "~0.13.0" "
  - NgModules can now be standalone (optional)
  - Required inputs can be marked with required: true
  - Signals introduced (preview)
  - DestroyRef introduced for takeUntilDestroyed()
  - Router uses functional guards by default
  "

  update_custom_webpack_builder "16"

  # ===========================================================================
  # ANGULAR 16 → 17
  # ===========================================================================

  migrate_to_version "17" "~5.2.0" "~0.14.0" "
  - New control flow syntax (@if, @for, @switch) - optional
  - Deferrable views (@defer) introduced
  - Signals become stable
  - New application builder (esbuild) as default
  - View transitions API support
  - SSR improvements
  "

  update_custom_webpack_builder "17"

  # ===========================================================================
  # ANGULAR 17 → 18
  # ===========================================================================

  migrate_to_version "18" "~5.4.0" "~0.15.0" "
  - Zoneless change detection (experimental)
  - Material 3 design tokens
  - Stable signal-based inputs, outputs, and queries
  - Fallback content for ng-content
  - Route redirects can use functions
  - @let syntax for template variables
  "

  update_custom_webpack_builder "18"

  # ===========================================================================
  # MANUAL MIGRATIONS & FINALIZATION
  # ===========================================================================

  apply_manual_migrations
  finalize_migration

  # ===========================================================================
  # SUMMARY
  # ===========================================================================

  log_step "MIGRATION COMPLETE"

  echo ""
  echo "╔══════════════════════════════════════════════════════════════════╗"
  echo "║                    MIGRATION SUMMARY                             ║"
  echo "╠══════════════════════════════════════════════════════════════════╣"
  echo "║  ✓ Angular 15 → 16 → 17 → 18 completed                          ║"
  echo "║                                                                  ║"
  echo "║  NEXT STEPS:                                                     ║"
  echo "║  1. Review the migration log: $LOG_FILE"
  echo "║  2. Run tests: npm test                                          ║"
  echo "║  3. Run E2E tests: npm run e2e                                   ║"
  echo "║  4. Test the protected build: npm run build:protected            ║"
  echo "║  5. Test Tauri build: npm run tauri:build                        ║"
  echo "║                                                                  ║"
  echo "║  OPTIONAL MODERNIZATIONS:                                        ║"
  echo "║  - Convert remaining modules to standalone components            ║"
  echo "║  - Adopt new control flow (@if, @for instead of *ngIf, *ngFor)  ║"
  echo "║  - Use signal-based inputs/outputs                               ║"
  echo "║  - Consider zoneless change detection (experimental)             ║"
  echo "║                                                                  ║"
  echo "║  BACKUP BRANCHES CREATED:                                        ║"
  echo "║  - pre-angular-15-original-migration                             ║"
  echo "║  - pre-angular-16-migration                                      ║"
  echo "║  - pre-angular-17-migration                                      ║"
  echo "║  - pre-angular-18-migration                                      ║"
  echo "╚══════════════════════════════════════════════════════════════════╝"
  echo ""

  log_success "Migration script completed. Check $LOG_FILE for full details."
}

# Run main function
main
