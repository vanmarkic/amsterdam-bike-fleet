import { Injectable } from '@angular/core';

/**
 * Bike status enum matching Rust backend
 */
export type BikeStatus = 'available' | 'in_use' | 'maintenance' | 'charging' | 'offline';

/**
 * Bike model matching Rust backend
 */
export interface Bike {
  id: string;
  name: string;
  status: BikeStatus;
  latitude: number;
  longitude: number;
  battery_level: number | null;
  last_maintenance: string | null;
  total_trips: number;
  total_distance_km: number;
  created_at: string;
  updated_at: string;
}

/**
 * Fleet statistics model
 */
export interface FleetStats {
  total_bikes: number;
  available_bikes: number;
  bikes_in_use: number;
  bikes_in_maintenance: number;
  bikes_charging: number;
  bikes_offline: number;
  average_battery: number;
  total_trips_today: number;
}

/**
 * Database statistics model
 */
export interface DatabaseStats {
  total_bikes: number;
  total_trips: number;
  database_size_bytes: number;
  last_sync: string | null;
}

/**
 * Health check response
 */
export interface HealthStatus {
  status: string;
  version: string;
  rust_version: string;
  tauri_version: string;
  timestamp: string;
}

/**
 * Request to add a new bike
 */
export interface AddBikeRequest {
  name: string;
  latitude: number;
  longitude: number;
  battery_level?: number;
}

/**
 * Request to update bike status
 */
export interface UpdateBikeStatusRequest {
  bike_id: string;
  status: BikeStatus;
  latitude?: number;
  longitude?: number;
  battery_level?: number;
}

/**
 * License information from the Rust backend
 */
export interface LicenseInfo {
  customer: string;
  company?: string;
  product: string;
  expires: string;
  features: string[];
  seats?: number;
  issued?: string;
  version: number;
}

/**
 * License status response
 */
export interface LicenseStatus {
  valid: boolean;
  info: LicenseInfo | null;
  error: string | null;
  days_remaining: number | null;
}

/**
 * License activation response
 */
export interface ActivateLicenseResponse {
  success: boolean;
  status: LicenseStatus;
  message: string;
}

/**
 * Service for communicating with the Tauri Rust backend via IPC
 *
 * Usage:
 * - Inject this service into components that need to interact with native features
 * - All methods return Promises that resolve when the Rust command completes
 * - Falls back gracefully when running in browser (non-Tauri) environment
 */
@Injectable({
  providedIn: 'root'
})
export class TauriService {
  private invoke: ((cmd: string, args?: Record<string, unknown>) => Promise<unknown>) | null = null;

  constructor() {
    this.initializeTauri();
  }

  /**
   * Initialize Tauri invoke function
   * Sets up the bridge to Rust backend
   */
  private async initializeTauri(): Promise<void> {
    if (typeof window !== 'undefined' && '__TAURI__' in window) {
      try {
        // Tauri v2 uses window.__TAURI__.core.invoke
        const tauri = (window as any).__TAURI__;
        if (tauri?.core?.invoke) {
          this.invoke = tauri.core.invoke;
        } else if (tauri?.invoke) {
          // Fallback for older API
          this.invoke = tauri.invoke;
        }
      } catch (e) {
        console.warn('Failed to initialize Tauri:', e);
      }
    }
  }

  /**
   * Check if running inside Tauri
   */
  isTauri(): boolean {
    return this.invoke !== null;
  }

  /**
   * Generic invoke wrapper with error handling
   */
  private async invokeCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    if (!this.invoke) {
      throw new Error('Not running in Tauri environment');
    }
    return this.invoke(command, args) as Promise<T>;
  }

  // ============================================
  // Health & System Commands
  // ============================================

  /**
   * Check the health of the Rust backend
   */
  async healthCheck(): Promise<HealthStatus> {
    return this.invokeCommand<HealthStatus>('health_check');
  }

  // ============================================
  // Database Commands
  // ============================================

  /**
   * Initialize the SQLite database
   * Must be called before other database operations
   */
  async initDatabase(): Promise<string> {
    return this.invokeCommand<string>('init_database');
  }

  /**
   * Get database statistics
   */
  async getDatabaseStats(): Promise<DatabaseStats> {
    return this.invokeCommand<DatabaseStats>('get_database_stats');
  }

  /**
   * Check if database is initialized
   */
  async isDatabaseInitialized(): Promise<boolean> {
    return this.invokeCommand<boolean>('is_database_initialized');
  }

  // ============================================
  // Fleet Commands
  // ============================================

  /**
   * Get all bikes in the fleet
   */
  async getFleetData(): Promise<Bike[]> {
    return this.invokeCommand<Bike[]>('get_fleet_data');
  }

  /**
   * Get a specific bike by ID
   */
  async getBikeById(bikeId: string): Promise<Bike | null> {
    return this.invokeCommand<Bike | null>('get_bike_by_id', { bikeId });
  }

  /**
   * Add a new bike to the fleet
   */
  async addBike(request: AddBikeRequest): Promise<Bike> {
    return this.invokeCommand<Bike>('add_bike', { request });
  }

  /**
   * Update the status of a bike
   */
  async updateBikeStatus(request: UpdateBikeStatusRequest): Promise<void> {
    return this.invokeCommand<void>('update_bike_status', { request });
  }

  /**
   * Get fleet statistics
   */
  async getFleetStats(): Promise<FleetStats> {
    return this.invokeCommand<FleetStats>('get_fleet_stats');
  }

  // ============================================
  // License Commands
  // ============================================

  /**
   * Activate a license key
   * Validates and stores the license if valid
   */
  async activateLicense(licenseKey: string): Promise<ActivateLicenseResponse> {
    return this.invokeCommand<ActivateLicenseResponse>('activate_license', { licenseKey });
  }

  /**
   * Get current license status
   * Returns the status of the stored license (if any)
   */
  async getLicenseStatus(): Promise<LicenseStatus> {
    return this.invokeCommand<LicenseStatus>('get_license_status');
  }

  /**
   * Deactivate (remove) the current license
   */
  async deactivateLicense(): Promise<string> {
    return this.invokeCommand<string>('deactivate_license');
  }

  /**
   * Check if a specific feature is licensed
   */
  async isFeatureLicensed(feature: string): Promise<boolean> {
    return this.invokeCommand<boolean>('is_feature_licensed', { feature });
  }

  /**
   * Validate a license key without storing it
   * Use this to preview license info before activation
   */
  async validateLicense(licenseKey: string): Promise<LicenseStatus> {
    return this.invokeCommand<LicenseStatus>('validate_license', { licenseKey });
  }
}
