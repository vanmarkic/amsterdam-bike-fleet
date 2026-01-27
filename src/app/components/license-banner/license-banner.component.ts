import { Component, ChangeDetectionStrategy } from '@angular/core';
import { Observable, combineLatest } from 'rxjs';
import { map } from 'rxjs/operators';
import { LicenseService } from '../../services/license.service';

export type BannerType = 'warning' | 'error' | 'none';

export interface BannerState {
  type: BannerType;
  daysRemaining: number | null;
  message: string;
}

@Component({
  selector: 'app-license-banner',
  templateUrl: './license-banner.component.html',
  styleUrls: ['./license-banner.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush
})
export class LicenseBannerComponent {
  dismissed = false;

  readonly bannerState$: Observable<BannerState>;

  constructor(public licenseService: LicenseService) {
    this.bannerState$ = combineLatest([
      this.licenseService.isExpired$,
      this.licenseService.isExpiringSoon$,
      this.licenseService.daysRemaining$
    ]).pipe(
      map(([isExpired, isExpiringSoon, daysRemaining]) => {
        if (isExpired) {
          return {
            type: 'error' as BannerType,
            daysRemaining,
            message: 'Your license has expired. Please renew to continue using all features.'
          };
        }

        if (isExpiringSoon && daysRemaining !== null) {
          const dayText = daysRemaining === 1 ? 'day' : 'days';
          return {
            type: 'warning' as BannerType,
            daysRemaining,
            message: `Your license expires in ${daysRemaining} ${dayText}. Renew now to avoid service interruption.`
          };
        }

        return {
          type: 'none' as BannerType,
          daysRemaining,
          message: ''
        };
      })
    );
  }

  dismiss(): void {
    this.dismissed = true;
  }

  openLicenseManagement(): void {
    // Navigate to license management or trigger a modal
    // For now, we'll just log and refresh the license check
    console.log('Opening license management...');
    this.licenseService.checkLicense();
  }
}
