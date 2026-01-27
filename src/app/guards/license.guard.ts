import { inject } from '@angular/core';
import { CanActivateFn, Router, UrlTree } from '@angular/router';
import { LicenseService } from '../services/license.service';

/**
 * Guard that requires a valid license to access the route.
 * Redirects to /license page if not licensed.
 *
 * Note: Returns a Promise explicitly to ensure Angular properly awaits it.
 */
export const licenseGuard: CanActivateFn = (): Promise<boolean | UrlTree> => {
  console.log('[licenseGuard] Guard invoked!');

  const licenseService = inject(LicenseService);
  const router = inject(Router);

  const checkLicense = async (): Promise<boolean | UrlTree> => {
    console.log('[licenseGuard] Starting async license check...');

    // Wait for Tauri to be fully initialized before checking
    const isTauri = await licenseService.isTauriEnvironmentAsync();
    console.log('[licenseGuard] isTauri:', isTauri);

    // In browser mode (dev), allow access
    if (!isTauri) {
      console.log('[licenseGuard] Not in Tauri, allowing access (dev mode)');
      return true;
    }

    // Check license status (this also waits for Tauri init)
    console.log('[licenseGuard] Checking license status...');
    const status = await licenseService.checkLicense();
    console.log('[licenseGuard] License status:', status);

    if (status?.valid) {
      console.log('[licenseGuard] License valid, allowing access');
      return true;
    }

    // Redirect to license page
    console.log('[licenseGuard] License invalid, redirecting to /license');
    return router.createUrlTree(['/license']);
  };

  return checkLicense();
};

/**
 * Guard that requires a specific feature to be licensed.
 * Usage: canActivate: [featureGuard('premium')]
 */
export function featureGuard(requiredFeature: string): CanActivateFn {
  return (): Promise<boolean | UrlTree> => {
    console.log(`[featureGuard] Guard invoked for feature: ${requiredFeature}`);

    const licenseService = inject(LicenseService);
    const router = inject(Router);

    const checkFeature = async (): Promise<boolean | UrlTree> => {
      console.log(`[featureGuard] Starting async check for feature: ${requiredFeature}`);

      // Wait for Tauri to be fully initialized before checking
      const isTauri = await licenseService.isTauriEnvironmentAsync();
      console.log(`[featureGuard] isTauri: ${isTauri}`);

      // In browser mode (dev), allow access
      if (!isTauri) {
        console.log('[featureGuard] Not in Tauri, allowing access (dev mode)');
        return true;
      }

      // Check if feature is licensed
      console.log(`[featureGuard] Checking if feature '${requiredFeature}' is licensed...`);
      const hasFeature = await licenseService.checkFeature(requiredFeature);
      console.log(`[featureGuard] hasFeature: ${hasFeature}`);

      if (hasFeature) {
        console.log('[featureGuard] Feature licensed, allowing access');
        return true;
      }

      // Redirect to license page with feature info
      console.log(`[featureGuard] Feature not licensed, redirecting to /license`);
      return router.createUrlTree(['/license'], {
        queryParams: { requiredFeature }
      });
    };

    return checkFeature();
  };
}
