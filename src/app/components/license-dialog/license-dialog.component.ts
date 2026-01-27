import {
  Component,
  OnInit,
  OnDestroy,
  Input,
  Output,
  EventEmitter,
  ChangeDetectionStrategy,
  ChangeDetectorRef
} from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { Subject, takeUntil } from 'rxjs';
import { LicenseService } from '../../services/license.service';
import { LicenseStatus, LicenseInfo } from '../../services/tauri.service';

/**
 * License Dialog Component for Angular 15
 *
 * A modal dialog for entering/viewing license keys.
 * Works both as a standalone page/route and as a modal.
 *
 * Usage as modal:
 * ```html
 * <app-license-dialog
 *   [isModal]="true"
 *   [isOpen]="showLicenseDialog"
 *   (closed)="showLicenseDialog = false">
 * </app-license-dialog>
 * ```
 *
 * Usage as standalone page:
 * ```html
 * <app-license-dialog [isModal]="false"></app-license-dialog>
 * ```
 */
@Component({
  selector: 'app-license-dialog',
  standalone: true,
  imports: [CommonModule, FormsModule],
  templateUrl: './license-dialog.component.html',
  styleUrls: ['./license-dialog.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush
})
export class LicenseDialogComponent implements OnInit, OnDestroy {
  /** Whether to display as a modal overlay or inline content.
   *  Default is false so component shows content when used as a routed page.
   *  Set to true when using as a modal dialog. */
  @Input() isModal = false;

  /** Controls modal visibility when isModal is true */
  @Input() isOpen = false;

  /** Emitted when the modal is closed */
  @Output() closed = new EventEmitter<void>();

  /** Emitted when license status changes */
  @Output() licenseChanged = new EventEmitter<LicenseStatus | null>();

  // Component state
  licenseKey = '';
  status: LicenseStatus | null = null;
  loading = false;
  error: string | null = null;
  validationResult: LicenseStatus | null = null;
  isValidating = false;
  activationSuccess = false;
  deactivationSuccess = false;

  private destroy$ = new Subject<void>();

  constructor(
    public licenseService: LicenseService,
    private cdr: ChangeDetectorRef
  ) {}

  ngOnInit(): void {
    // Subscribe to license service state
    this.licenseService.status$
      .pipe(takeUntil(this.destroy$))
      .subscribe(status => {
        this.status = status;
        this.cdr.markForCheck();
      });

    this.licenseService.loading$
      .pipe(takeUntil(this.destroy$))
      .subscribe(loading => {
        this.loading = loading;
        this.cdr.markForCheck();
      });

    this.licenseService.error$
      .pipe(takeUntil(this.destroy$))
      .subscribe(error => {
        this.error = error;
        this.cdr.markForCheck();
      });

    // Initial license check
    this.refreshStatus();
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  /**
   * Get the license info from current status
   */
  get licenseInfo(): LicenseInfo | null {
    return this.status?.info ?? null;
  }

  /**
   * Check if there's a valid license
   */
  get isLicensed(): boolean {
    return this.status?.valid ?? false;
  }

  /**
   * Get days remaining until license expiration
   */
  get daysRemaining(): number | null {
    return this.status?.days_remaining ?? null;
  }

  /**
   * Check if license is expiring soon (within 30 days)
   */
  get isExpiringSoon(): boolean {
    const days = this.daysRemaining;
    return days !== null && days <= 30 && days > 0;
  }

  /**
   * Check if license is expired
   */
  get isExpired(): boolean {
    const days = this.daysRemaining;
    return days !== null && days <= 0;
  }

  /**
   * Check if running in Tauri environment
   */
  get isTauriEnvironment(): boolean {
    return this.licenseService.isTauriEnvironment();
  }

  /**
   * Check if the license key input is valid for submission
   */
  get canActivate(): boolean {
    return this.licenseKey.trim().length > 0 && !this.loading && !this.isValidating;
  }

  /**
   * Check if deactivation is possible
   */
  get canDeactivate(): boolean {
    return this.isLicensed && !this.loading;
  }

  /**
   * Refresh the current license status
   */
  async refreshStatus(): Promise<void> {
    this.clearMessages();
    await this.licenseService.checkLicense();
  }

  /**
   * Validate the entered license key without activating
   */
  async validateKey(): Promise<void> {
    if (!this.licenseKey.trim()) {
      return;
    }

    this.clearMessages();
    this.isValidating = true;
    this.cdr.markForCheck();

    try {
      this.validationResult = await this.licenseService.validateLicense(this.licenseKey.trim());
      this.cdr.markForCheck();
    } catch (err) {
      this.error = err instanceof Error ? err.message : 'Validation failed';
      this.validationResult = null;
      this.cdr.markForCheck();
    } finally {
      this.isValidating = false;
      this.cdr.markForCheck();
    }
  }

  /**
   * Activate the entered license key
   */
  async activateLicense(): Promise<void> {
    if (!this.canActivate) {
      return;
    }

    this.clearMessages();

    try {
      const response = await this.licenseService.activateLicense(this.licenseKey.trim());

      if (response.success) {
        this.activationSuccess = true;
        this.licenseKey = '';
        this.validationResult = null;
        this.licenseChanged.emit(response.status);
      } else {
        this.error = response.message || 'Activation failed';
      }

      this.cdr.markForCheck();
    } catch (err) {
      this.error = err instanceof Error ? err.message : 'Activation failed';
      this.cdr.markForCheck();
    }
  }

  /**
   * Deactivate the current license
   */
  async deactivateLicense(): Promise<void> {
    if (!this.canDeactivate) {
      return;
    }

    this.clearMessages();

    try {
      await this.licenseService.deactivateLicense();
      this.deactivationSuccess = true;
      this.licenseChanged.emit(null);
      this.cdr.markForCheck();
    } catch (err) {
      this.error = err instanceof Error ? err.message : 'Deactivation failed';
      this.cdr.markForCheck();
    }
  }

  /**
   * Clear all status messages
   */
  clearMessages(): void {
    this.error = null;
    this.activationSuccess = false;
    this.deactivationSuccess = false;
    this.validationResult = null;
  }

  /**
   * Close the modal dialog
   */
  closeModal(): void {
    this.clearMessages();
    this.licenseKey = '';
    this.closed.emit();
  }

  /**
   * Handle click on backdrop to close modal
   */
  onBackdropClick(event: MouseEvent): void {
    if ((event.target as HTMLElement).classList.contains('modal-backdrop')) {
      this.closeModal();
    }
  }

  /**
   * Handle keyboard events for modal
   */
  onKeyDown(event: KeyboardEvent): void {
    if (event.key === 'Escape' && this.isModal) {
      this.closeModal();
    }
  }

  /**
   * Format features list for display
   */
  formatFeatures(features: string[]): string {
    if (!features || features.length === 0) {
      return 'None';
    }
    if (features.includes('*')) {
      return 'All features';
    }
    return features.join(', ');
  }

  /**
   * Format date string for display
   */
  formatDate(dateStr: string | undefined): string {
    if (!dateStr) {
      return 'N/A';
    }
    try {
      const date = new Date(dateStr);
      return date.toLocaleDateString('en-US', {
        year: 'numeric',
        month: 'long',
        day: 'numeric'
      });
    } catch {
      return dateStr;
    }
  }

  /**
   * Get status badge class based on license state
   */
  getStatusBadgeClass(): string {
    if (!this.status) {
      return 'badge-unknown';
    }
    if (this.isExpired) {
      return 'badge-expired';
    }
    if (this.isExpiringSoon) {
      return 'badge-warning';
    }
    if (this.isLicensed) {
      return 'badge-valid';
    }
    return 'badge-invalid';
  }

  /**
   * Get status text for display
   */
  getStatusText(): string {
    if (!this.status) {
      return 'Unknown';
    }
    if (this.isExpired) {
      return 'Expired';
    }
    if (this.isExpiringSoon) {
      return `Valid (${this.daysRemaining} days left)`;
    }
    if (this.isLicensed) {
      return 'Valid';
    }
    return 'No License';
  }
}
