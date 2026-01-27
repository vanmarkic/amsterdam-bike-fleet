import { inject } from '@angular/core';
import { CanActivateFn, Router } from '@angular/router';
import { LicenseService } from '../services/license.service';

/**
 * Guard that requires a valid license to access the route.
 * Redirects to /license page if not licensed.
 */
export const licenseGuard: CanActivateFn = async () => {
  const licenseService = inject(LicenseService);
  const router = inject(Router);

  // Wait for Tauri to be fully initialized before checking
  const isTauri = await licenseService.isTauriEnvironmentAsync();

  // In browser mode (dev), allow access
  if (!isTauri) {
    return true;
  }

  // Check license status (this also waits for Tauri init)
  const status = await licenseService.checkLicense();

  if (status?.valid) {
    return true;
  }

  // Redirect to license page
  return router.createUrlTree(['/license']);
};

/**
 * Guard that requires a specific feature to be licensed.
 * Usage: canActivate: [featureGuard('premium')]
 */
export function featureGuard(requiredFeature: string): CanActivateFn {
  return async () => {
    const licenseService = inject(LicenseService);
    const router = inject(Router);

    // Wait for Tauri to be fully initialized before checking
    const isTauri = await licenseService.isTauriEnvironmentAsync();

    // In browser mode (dev), allow access
    if (!isTauri) {
      return true;
    }

    // Check if feature is licensed
    const hasFeature = await licenseService.checkFeature(requiredFeature);

    if (hasFeature) {
      return true;
    }

    // Redirect to license page with feature info
    return router.createUrlTree(['/license'], {
      queryParams: { requiredFeature }
    });
  };
}
