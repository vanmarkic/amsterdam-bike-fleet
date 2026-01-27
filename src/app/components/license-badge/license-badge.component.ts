import { Component, ChangeDetectionStrategy, Output, EventEmitter } from '@angular/core';
import { Observable, combineLatest } from 'rxjs';
import { map } from 'rxjs/operators';
import { LicenseService } from '../../services/license.service';
import { LicenseStatus } from '../../services/tauri.service';

/**
 * License status types for badge display
 */
export type LicenseBadgeStatus = 'licensed' | 'trial' | 'unlicensed' | 'expired';

/**
 * Badge display data computed from license status
 */
export interface BadgeDisplayData {
  status: LicenseBadgeStatus;
  label: string;
  tooltip: string;
}

/**
 * License status badge component for header/toolbar placement
 *
 * Displays a small badge indicating the current license status:
 * - Licensed (green): Valid, active license
 * - Trial (yellow): Valid trial license or expiring soon
 * - Unlicensed (gray): No license present
 * - Expired (red): License has expired
 *
 * Clicking the badge emits an event to open the license dialog.
 *
 * Usage:
 * ```html
 * <app-license-badge (openLicenseDialog)="showLicenseDialog()"></app-license-badge>
 * ```
 */
@Component({
  selector: 'app-license-badge',
  templateUrl: './license-badge.component.html',
  styleUrls: ['./license-badge.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush
})
export class LicenseBadgeComponent {
  /**
   * Emitted when the badge is clicked to request opening the license dialog
   */
  @Output() openLicenseDialog = new EventEmitter<void>();

  /**
   * Observable of computed badge display data
   */
  readonly badgeData$: Observable<BadgeDisplayData>;

  /**
   * Observable for loading state
   */
  readonly loading$: Observable<boolean>;

  constructor(private licenseService: LicenseService) {
    this.loading$ = this.licenseService.loading$;

    // Combine status observables to compute badge display data
    this.badgeData$ = combineLatest([
      this.licenseService.status$,
      this.licenseService.isExpired$,
      this.licenseService.isExpiringSoon$
    ]).pipe(
      map(([status, isExpired, isExpiringSoon]) =>
        this.computeBadgeData(status, isExpired, isExpiringSoon)
      )
    );
  }

  /**
   * Handle badge click - emit event to open license dialog
   */
  onBadgeClick(): void {
    this.openLicenseDialog.emit();
  }

  /**
   * Compute badge display data from license status
   */
  private computeBadgeData(
    status: LicenseStatus | null,
    isExpired: boolean,
    isExpiringSoon: boolean
  ): BadgeDisplayData {
    // No status yet or error
    if (!status) {
      return {
        status: 'unlicensed',
        label: 'Unlicensed',
        tooltip: 'License status unknown'
      };
    }

    // Expired license
    if (isExpired) {
      return {
        status: 'expired',
        label: 'Expired',
        tooltip: status.info
          ? `License for ${status.info.customer} has expired`
          : 'License has expired'
      };
    }

    // Valid license
    if (status.valid && status.info) {
      // Check if it's a trial (has 'trial' feature or is expiring soon)
      const isTrial = status.info.features.includes('trial') || isExpiringSoon;

      if (isTrial) {
        const daysText = status.days_remaining !== null
          ? `${status.days_remaining} days remaining`
          : 'Expiring soon';

        return {
          status: 'trial',
          label: status.info.customer,
          tooltip: `Trial license - ${daysText}`
        };
      }

      // Full license
      return {
        status: 'licensed',
        label: status.info.customer,
        tooltip: status.info.company
          ? `Licensed to ${status.info.customer} (${status.info.company})`
          : `Licensed to ${status.info.customer}`
      };
    }

    // Not valid / no info - unlicensed
    return {
      status: 'unlicensed',
      label: 'Unlicensed',
      tooltip: status.error || 'No valid license found'
    };
  }
}
