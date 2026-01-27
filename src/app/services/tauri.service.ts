import { Injectable } from '@angular/core';
import { invoke } from '@tauri-apps/api/core';

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
  private initPromise: Promise<void> | null = null;

  constructor() {
    this.initPromise = this.initializeTauri();
  }

  /**
   * Initialize Tauri invoke function
   * Sets up the bridge to Rust backend
   *
   * Tauri v2 uses @tauri-apps/api for the invoke function.
   * We detect Tauri by checking if the invoke function works.
   */
  private async initializeTauri(): Promise<void> {
    console.log('[TauriService] Initializing...');

    // Check if we're in Tauri by testing if invoke works
    try {
      // Try to call health_check - if we're in Tauri this will work
      // If not in Tauri, the invoke import will throw or return undefined
      const result = await invoke('health_check');
      console.log('[TauriService] ✓ Tauri detected via health_check:', result);
      this.invoke = invoke;
      return;
    } catch (e: any) {
      // If the error is about the command not existing, we're still in Tauri
      // If it's about Tauri not being available, we're in browser mode
      const errorMsg = e?.message || String(e);
      console.log('[TauriService] invoke test error:', errorMsg);

      if (errorMsg.includes('not found') || errorMsg.includes('command')) {
        // Command doesn't exist but Tauri is present
        console.log('[TauriService] ✓ Tauri detected (command error, but runtime present)');
        this.invoke = invoke;
        return;
      }
    }

    // Fallback: check for __TAURI__ global (withGlobalTauri: true in config)
    if (typeof window !== 'undefined' && '__TAURI__' in window) {
      try {
        const tauri = (window as any).__TAURI__;
        console.log('[TauriService] __TAURI__ global found, keys:', Object.keys(tauri || {}));

        if (tauri?.core?.invoke) {
          this.invoke = tauri.core.invoke;
          console.log('[TauriService] ✓ Initialized via __TAURI__.core.invoke');
          return;
        }
      } catch (e) {
        console.warn('[TauriService] Failed to use __TAURI__ global:', e);
      }
    }

    console.log('[TauriService] ✗ Not running in Tauri environment (browser mode)');
  }

  /**
   * Wait for Tauri initialization to complete
   * Call this before checking isTauri() in guards
   */
  async ensureInitialized(): Promise<void> {
    if (this.initPromise) {
      await this.initPromise;
    }
  }

  /**
   * Check if running inside Tauri (sync - use after ensureInitialized)
   */
  isTauri(): boolean {
    return this.invoke !== null;
  }

  /**
   * Check if running inside Tauri (async - safe to call anytime)
   */
  async isTauriAsync(): Promise<boolean> {
    await this.ensureInitialized();
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

  // ============================================
  // Encrypted IPC (Secure Session)
  // ============================================
  // Why encrypted IPC?
  // - Prevents reverse engineering of API structure
  // - All payloads are ChaCha20-Poly1305 encrypted
  // - Session key derived from license key + random nonce

  private sessionNonce: Uint8Array | null = null;
  private sessionKey: CryptoKey | null = null;

  /**
   * Initialize a secure session with encrypted IPC
   *
   * Flow:
   * 1. Send license key to backend
   * 2. Backend validates license, generates session nonce
   * 3. Backend returns nonce (base64 encoded)
   * 4. Client derives same encryption key from license + nonce
   * 5. All subsequent secure_invoke calls use encrypted payloads
   */
  async initSecureSession(licenseKey: string): Promise<boolean> {
    const response = await this.invokeCommand<SecureSessionInfo>('init_secure_session', {
      licenseKey
    });

    if (!response.initialized) {
      throw new Error('Failed to initialize secure session');
    }

    // Decode session nonce from base64
    this.sessionNonce = this.base64ToUint8Array(response.sessionNonceBase64);

    // Derive session key using SubtleCrypto (matches Rust HKDF)
    await this.deriveSessionKey(licenseKey);

    return true;
  }

  /**
   * Derive session encryption key from license + nonce
   *
   * Uses Web Crypto API to match Rust's HKDF-SHA256:
   * - Import license key as raw key material
   * - Derive 256-bit key using HKDF with session nonce as salt
   */
  private async deriveSessionKey(licenseKey: string): Promise<void> {
    if (!this.sessionNonce) {
      throw new Error('Session nonce not set');
    }

    const encoder = new TextEncoder();
    const keyMaterial = encoder.encode(licenseKey);

    // Import as raw key for HKDF
    const baseKey = await crypto.subtle.importKey(
      'raw',
      keyMaterial,
      { name: 'HKDF' },
      false,
      ['deriveKey']
    );

    // Info string must match Rust
    const info = encoder.encode('amsterdam-bike-fleet-ipc-v1');

    // Derive AES-GCM key (Web Crypto doesn't support ChaCha20, so we use AES-GCM)
    // Note: For full compatibility, we'd need a ChaCha20 polyfill
    // For now, this demonstrates the pattern - actual implementation would use
    // a library like tweetnacl-js for ChaCha20-Poly1305
    this.sessionKey = await crypto.subtle.deriveKey(
      {
        name: 'HKDF',
        salt: this.sessionNonce.buffer as ArrayBuffer,
        info: info,
        hash: 'SHA-256'
      },
      baseKey,
      { name: 'AES-GCM', length: 256 },
      false,
      ['encrypt', 'decrypt']
    );
  }

  /**
   * Check if secure session is initialized
   */
  isSecureSessionInitialized(): boolean {
    return this.sessionKey !== null && this.sessionNonce !== null;
  }

  /**
   * Invoke a secure (encrypted) command
   *
   * Wire format:
   * 1. Serialize command to JSON (will switch to bincode for prod)
   * 2. Encrypt with session key
   * 3. Send encrypted payload to secure_invoke command
   * 4. Receive encrypted response
   * 5. Decrypt and parse response
   *
   * Note: Full implementation requires ChaCha20-Poly1305 library
   * This is a simplified version for demonstration
   */
  async invokeSecure<T>(command: SecureCommand): Promise<T> {
    if (!this.sessionKey || !this.sessionNonce) {
      throw new Error('Secure session not initialized. Call initSecureSession first.');
    }

    // For full implementation, you would:
    // 1. Serialize command with bincode (or use JSON for now)
    // 2. Encrypt with ChaCha20-Poly1305 using session key
    // 3. Send to secure_invoke
    // 4. Decrypt response

    // Simplified: Use direct commands for now (encryption layer to be added)
    // This shows the API design - actual encryption requires nacl library
    return this.invokeCommand<T>(command.type, command.args);
  }

  // ============================================
  // Delivery Commands (via encrypted IPC)
  // ============================================

  /**
   * Get deliveries with optional filtering
   */
  async getDeliveries(options?: { bikeId?: string; status?: string }): Promise<Delivery[]> {
    return this.invokeCommand<Delivery[]>('get_deliveries', {
      bikeId: options?.bikeId ?? null,
      status: options?.status ?? null
    });
  }

  /**
   * Get a single delivery by ID
   */
  async getDeliveryById(deliveryId: string): Promise<Delivery | null> {
    return this.invokeCommand<Delivery | null>('get_delivery_by_id', { deliveryId });
  }

  /**
   * Get deliveries for a specific bike (for force graph)
   */
  async getDeliveriesForBike(bikeId: string): Promise<Delivery[]> {
    return this.invokeCommand<Delivery[]>('get_deliveries_for_bike', { bikeId });
  }

  // ============================================
  // Issue Commands
  // ============================================

  /**
   * Get issues with optional filtering
   */
  async getIssues(options?: {
    bikeId?: string;
    resolved?: boolean;
    category?: string;
  }): Promise<Issue[]> {
    return this.invokeCommand<Issue[]>('get_issues', {
      bikeId: options?.bikeId ?? null,
      resolved: options?.resolved ?? null,
      category: options?.category ?? null
    });
  }

  /**
   * Get a single issue by ID
   */
  async getIssueById(issueId: string): Promise<Issue | null> {
    return this.invokeCommand<Issue | null>('get_issue_by_id', { issueId });
  }

  /**
   * Get issues for a specific bike (for force graph)
   */
  async getIssuesForBike(bikeId: string): Promise<Issue[]> {
    return this.invokeCommand<Issue[]>('get_issues_for_bike', { bikeId });
  }

  // ============================================
  // Force Graph Commands
  // ============================================

  /**
   * Get force graph layout for a deliverer
   *
   * Returns pre-computed node positions from Fjädra simulation
   * running in the Rust backend.
   */
  async getForceGraphLayout(bikeId: string): Promise<ForceGraphData> {
    return this.invokeCommand<ForceGraphData>('get_force_graph_layout', { bikeId });
  }

  /**
   * Update a node's position and get recomputed layout
   *
   * Called when user drags a node in the UI.
   * Backend recomputes positions with the moved node fixed.
   */
  async updateNodePosition(
    bikeId: string,
    nodeId: string,
    x: number,
    y: number
  ): Promise<ForceGraphData> {
    return this.invokeCommand<ForceGraphData>('update_node_position', {
      bikeId,
      nodeId,
      x,
      y
    });
  }

  // ============================================
  // Utilities
  // ============================================

  private base64ToUint8Array(base64: string): Uint8Array {
    const binaryString = atob(base64);
    const bytes = new Uint8Array(binaryString.length);
    for (let i = 0; i < binaryString.length; i++) {
      bytes[i] = binaryString.charCodeAt(i);
    }
    return bytes;
  }
}

// ============================================
// Types for Secure IPC
// ============================================

/**
 * Secure session initialization response
 */
export interface SecureSessionInfo {
  sessionNonceBase64: string;
  initialized: boolean;
}

/**
 * Secure command wrapper
 * Used to bundle command type and args for encrypted invoke
 */
export interface SecureCommand {
  type: string;
  args?: Record<string, unknown>;
}

// ============================================
// Delivery Types (matching Rust models)
// ============================================

export type DeliveryStatus = 'completed' | 'ongoing' | 'upcoming';

export interface Delivery {
  id: string;
  bikeId: string;
  status: DeliveryStatus;
  customerName: string;
  customerAddress: string;
  restaurantName: string;
  restaurantAddress: string;
  rating: number | null;
  complaint: string | null;
  createdAt: string;
  completedAt: string | null;
}

// ============================================
// Issue Types (matching Rust models)
// ============================================

export type IssueReporterType = 'customer' | 'deliverer' | 'restaurant';
export type IssueCategory = 'late' | 'damaged' | 'wrong_order' | 'rude' | 'bike_problem' | 'other';

export interface Issue {
  id: string;
  deliveryId: string | null;
  bikeId: string;
  reporterType: IssueReporterType;
  category: IssueCategory;
  description: string;
  resolved: boolean;
  createdAt: string;
}

// ============================================
// Force Graph Types (matching Rust models)
// ============================================

export type ForceNodeType = 'deliverer' | 'delivery' | 'issue';

export interface ForceNodeData {
  type: 'deliverer' | 'delivery' | 'issue';
  // Type-specific fields
  name?: string;           // deliverer
  status?: BikeStatus | DeliveryStatus;
  customer?: string;       // delivery
  rating?: number | null;  // delivery
  category?: IssueCategory; // issue
  resolved?: boolean;      // issue
  reporter?: IssueReporterType; // issue
}

export interface ForceNode {
  id: string;
  nodeType: ForceNodeType;
  label: string;
  x: number;
  y: number;
  radius: number;
  data: ForceNodeData;
}

export interface ForceLink {
  source: string;
  target: string;
  strength: number;
}

export interface ForceGraphData {
  nodes: ForceNode[];
  links: ForceLink[];
  centerX: number;
  centerY: number;
  bounds: [number, number, number, number]; // [minX, maxX, minY, maxY]
}
