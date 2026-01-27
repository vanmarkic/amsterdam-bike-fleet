import { Injectable } from '@angular/core';
import { BehaviorSubject, Observable } from 'rxjs';
import { map } from 'rxjs/operators';
import { TauriService, LicenseStatus, ActivateLicenseResponse } from './tauri.service';

/**
 * License management service for Angular 15
 *
 * Provides reactive state for license status and feature gating.
 * Works with the Rust backend for cryptographic license verification.
 *
 * Usage:
 * ```typescript
 * @Component({...})
 * export class MyComponent {
 *   constructor(public license: LicenseService) {}
 *
 *   // Use in template with async pipe
 *   // <div *ngIf="license.isLicensed$ | async">Premium content</div>
 * }
 * ```
 */
@Injectable({
  providedIn: 'root'
})
export class LicenseService {
  // Reactive state using BehaviorSubject
  private readonly _status = new BehaviorSubject<LicenseStatus | null>(null);
  private readonly _loading = new BehaviorSubject<boolean>(false);
  private readonly _error = new BehaviorSubject<string | null>(null);

  // Public observables
  readonly status$: Observable<LicenseStatus | null> = this._status.asObservable();
  readonly loading$: Observable<boolean> = this._loading.asObservable();
  readonly error$: Observable<string | null> = this._error.asObservable();

  readonly isLicensed$: Observable<boolean> = this._status.pipe(
    map(s => s?.valid ?? false)
  );

  readonly licenseInfo$ = this._status.pipe(
    map(s => s?.info ?? null)
  );

  readonly daysRemaining$ = this._status.pipe(
    map(s => s?.days_remaining ?? null)
  );

  readonly customer$ = this._status.pipe(
    map(s => s?.info?.customer ?? null)
  );

  readonly features$ = this._status.pipe(
    map(s => s?.info?.features ?? [])
  );

  readonly isExpiringSoon$ = this._status.pipe(
    map(s => {
      const days = s?.days_remaining;
      return days !== null && days !== undefined && days <= 30 && days > 0;
    })
  );

  readonly isExpired$ = this._status.pipe(
    map(s => {
      const days = s?.days_remaining;
      return days !== null && days !== undefined && days <= 0;
    })
  );

  constructor(private tauri: TauriService) {
    // Auto-check license on service init if in Tauri
    this.checkLicense();
  }

  /**
   * Get current status value synchronously
   */
  get status(): LicenseStatus | null {
    return this._status.getValue();
  }

  /**
   * Check if currently licensed (sync)
   */
  get isLicensed(): boolean {
    return this._status.getValue()?.valid ?? false;
  }

  /**
   * Get current loading state (sync)
   */
  get loading(): boolean {
    return this._loading.getValue();
  }

  /**
   * Check if running in Tauri (license features available) - sync version
   * Note: May return false during startup. Use isTauriEnvironmentAsync() for guards.
   */
  isTauriEnvironment(): boolean {
    return this.tauri.isTauri();
  }

  /**
   * Check if running in Tauri - async version (waits for init)
   * Use this in guards to ensure Tauri is fully initialized.
   */
  async isTauriEnvironmentAsync(): Promise<boolean> {
    return this.tauri.isTauriAsync();
  }

  /**
   * Check the current license status
   */
  async checkLicense(): Promise<LicenseStatus | null> {
    // Wait for Tauri to be fully initialized
    const isTauri = await this.tauri.isTauriAsync();

    if (!isTauri) {
      // Not in Tauri, return unlicensed status
      const browserStatus: LicenseStatus = {
        valid: false,
        info: null,
        error: 'Running in browser mode (no license check)',
        days_remaining: null
      };
      this._status.next(browserStatus);
      return browserStatus;
    }

    this._loading.next(true);
    this._error.next(null);

    try {
      const status = await this.tauri.getLicenseStatus();
      this._status.next(status);
      return status;
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      this._error.next(errorMsg);
      return null;
    } finally {
      this._loading.next(false);
    }
  }

  /**
   * Activate a license key
   */
  async activateLicense(licenseKey: string): Promise<ActivateLicenseResponse> {
    if (!this.tauri.isTauri()) {
      throw new Error('License activation requires Tauri desktop app');
    }

    this._loading.next(true);
    this._error.next(null);

    try {
      const response = await this.tauri.activateLicense(licenseKey);
      this._status.next(response.status);

      if (!response.success) {
        this._error.next(response.message);
      }

      return response;
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      this._error.next(errorMsg);
      throw err;
    } finally {
      this._loading.next(false);
    }
  }

  /**
   * Validate a license key without activating
   */
  async validateLicense(licenseKey: string): Promise<LicenseStatus> {
    if (!this.tauri.isTauri()) {
      throw new Error('License validation requires Tauri desktop app');
    }

    return this.tauri.validateLicense(licenseKey);
  }

  /**
   * Deactivate the current license
   */
  async deactivateLicense(): Promise<void> {
    if (!this.tauri.isTauri()) {
      throw new Error('License deactivation requires Tauri desktop app');
    }

    this._loading.next(true);
    this._error.next(null);

    try {
      await this.tauri.deactivateLicense();
      this._status.next({
        valid: false,
        info: null,
        error: 'License deactivated',
        days_remaining: null
      });
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : String(err);
      this._error.next(errorMsg);
      throw err;
    } finally {
      this._loading.next(false);
    }
  }

  /**
   * Check if a specific feature is licensed (sync)
   */
  hasFeature(feature: string): boolean {
    const features = this._status.getValue()?.info?.features ?? [];
    return features.includes(feature) || features.includes('*');
  }

  /**
   * Async check if a specific feature is licensed (uses Rust backend)
   */
  async checkFeature(feature: string): Promise<boolean> {
    if (!this.tauri.isTauri()) {
      return false;
    }

    try {
      return await this.tauri.isFeatureLicensed(feature);
    } catch {
      return false;
    }
  }

  /**
   * Format license info for display
   */
  formatLicenseInfo(): string {
    const info = this._status.getValue()?.info;
    if (!info) return 'No license';

    const parts = [info.customer];
    if (info.company) parts.push(`(${info.company})`);
    parts.push(`- Expires: ${info.expires}`);

    return parts.join(' ');
  }
}
