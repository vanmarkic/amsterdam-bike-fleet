import { Injectable } from '@angular/core';
import { BikePosition } from '../models/fleet.models';

// ============================================================================
// TypeScript Types for WASM Functions
// ============================================================================

/**
 * Fleet statistics calculated by WASM module
 */
export interface FleetStatistics {
  totalBikes: number;
  deliveringCount: number;
  idleCount: number;
  returningCount: number;
  averageSpeed: number;
  maxSpeed: number;
  minSpeed: number;
  activePercentage: number;
  fleetCenterLongitude: number;
  fleetCenterLatitude: number;
}

/**
 * Validation result for bike data
 */
export interface ValidationResult {
  isValid: boolean;
  errors: string[];
  warnings: string[];
  sanitizedData: BikePosition | null;
}

/**
 * Distance calculation result
 */
export interface DistanceResult {
  distanceKm: number;
  distanceMiles: number;
  bearingDegrees: number;
}

/**
 * Result of bike movement simulation
 */
export interface SimulationResult {
  bikes: BikePosition[];
  movementsApplied: number;
  boundCorrections: number;
}

/**
 * Status transition result
 */
export interface StatusTransitionResult {
  newStatus: 'delivering' | 'returning' | 'idle';
  transitionOccurred: boolean;
  probabilityUsed: number;
}

/**
 * Speed calculation result
 */
export interface SpeedResult {
  speed: number;
  baseSpeed: number;
  trafficPenalty: number;
  statusFactor: string;
}

/**
 * Complete simulation tick result - combines movement, status, speed, and stats
 */
export interface SimulationTickResult {
  bikes: BikePosition[];
  statistics: FleetStatistics;
  positionHash: number;
  stateHash: number;
  statusTransitions: number;
  boundsCorrections: number;
}

/**
 * Geographic coordinate
 */
export interface Coordinate {
  longitude: number;
  latitude: number;
}

// ============================================================================
// WASM Module Interface
// ============================================================================

/**
 * Interface for the WASM module exports
 */
interface WasmModule {
  // Fleet statistics
  calculateFleetStatistics(bikes: BikePosition[]): FleetStatistics;

  // Validation
  validateBikeData(bike: BikePosition): ValidationResult;
  validateBikeDataBatch(bikes: BikePosition[]): ValidationResult[];

  // Geographic calculations
  calculateDistance(from: Coordinate, to: Coordinate): DistanceResult;
  calculateBikeDistance(bike: BikePosition, target: Coordinate): DistanceResult;
  findNearestBike(bikes: BikePosition[], target: Coordinate): BikePosition;
  findBikesInRadius(bikes: BikePosition[], center: Coordinate, radiusKm: number): BikePosition[];

  // Simulation functions (NEW)
  simulateBikeMovement(bikes: BikePosition[], seed: number): SimulationResult;
  transitionBikeStatus(currentStatus: string, randomValue: number): StatusTransitionResult;
  transitionBikeStatusBatch(statuses: string[], randomValues: number[]): StatusTransitionResult[];
  calculateBikeSpeed(status: string, isInTraffic: boolean, randomFactor: number): SpeedResult;
  calculateBikeSpeedBatch(statuses: string[], inTraffic: boolean[], randomFactors: number[]): number[];
  hashBikePositions(bikes: BikePosition[]): number;
  hashBikeState(bikes: BikePosition[]): number;
  simulationTick(bikes: BikePosition[], timestamp: number, transitionProbability: number): SimulationTickResult;
}

// ============================================================================
// WASM Service
// ============================================================================

/**
 * Service for loading and interacting with the WASM module.
 *
 * This service provides access to protected client-side algorithms
 * compiled to WebAssembly for performance and intellectual property protection.
 *
 * @example
 * ```typescript
 * // In a component
 * constructor(private wasmService: WasmService) {}
 *
 * async ngOnInit() {
 *   await this.wasmService.initialize();
 *   const stats = this.wasmService.calculateFleetStatistics(bikes);
 * }
 * ```
 */
@Injectable({
  providedIn: 'root'
})
export class WasmService {
  private wasmModule: WasmModule | null = null;
  private initPromise: Promise<void> | null = null;
  private _isInitialized = false;

  /**
   * Whether the WASM module has been initialized
   */
  get isInitialized(): boolean {
    return this._isInitialized;
  }

  /**
   * Initialize the WASM module.
   * This should be called before using any WASM functions.
   * Safe to call multiple times - subsequent calls return the same promise.
   */
  async initialize(): Promise<void> {
    // Return existing promise if already initializing
    if (this.initPromise) {
      return this.initPromise;
    }

    this.initPromise = this.loadWasmModule();
    return this.initPromise;
  }

  /**
   * Internal method to load the WASM module
   */
  private async loadWasmModule(): Promise<void> {
    try {
      // Dynamic import of the WASM package
      // Uses path alias @wasm/* configured in tsconfig.json
      // eslint-disable-next-line @typescript-eslint/ban-ts-comment
      // @ts-ignore - Path resolved at build time, stub exists for dev
      const wasm = await import('@wasm/amsterdam_bike_fleet_wasm');

      // Initialize the WASM module (calls the init function in lib.rs)
      if (typeof wasm.default === 'function') {
        await wasm.default();
      }

      this.wasmModule = wasm as unknown as WasmModule;
      this._isInitialized = true;

      console.log('[WasmService] WASM module initialized successfully');
    } catch (error) {
      console.error('[WasmService] Failed to initialize WASM module:', error);
      throw new Error(
        'Failed to load WASM module. Make sure to run "npm run wasm:build" first. ' +
        `Error: ${error instanceof Error ? error.message : String(error)}`
      );
    }
  }

  /**
   * Ensure the WASM module is initialized before calling a function
   */
  private ensureInitialized(): void {
    if (!this._isInitialized || !this.wasmModule) {
      throw new Error(
        'WASM module not initialized. Call initialize() first and await its completion.'
      );
    }
  }

  // ==========================================================================
  // Fleet Statistics
  // ==========================================================================

  /**
   * Calculate comprehensive fleet statistics from bike position data.
   *
   * @param bikes - Array of bike positions
   * @returns Fleet statistics including counts, speeds, and geographic center
   * @throws Error if WASM not initialized or calculation fails
   */
  calculateFleetStatistics(bikes: BikePosition[]): FleetStatistics {
    this.ensureInitialized();
    try {
      return this.wasmModule!.calculateFleetStatistics(bikes);
    } catch (error) {
      throw this.wrapWasmError('calculateFleetStatistics', error);
    }
  }

  // ==========================================================================
  // Data Validation
  // ==========================================================================

  /**
   * Validate and sanitize a single bike position.
   *
   * Checks that coordinates are within Amsterdam bounds, speed is reasonable,
   * and all required fields are present.
   *
   * @param bike - Bike position to validate
   * @returns Validation result with errors, warnings, and sanitized data
   * @throws Error if WASM not initialized
   */
  validateBikeData(bike: BikePosition): ValidationResult {
    this.ensureInitialized();
    try {
      return this.wasmModule!.validateBikeData(bike);
    } catch (error) {
      throw this.wrapWasmError('validateBikeData', error);
    }
  }

  /**
   * Validate multiple bike positions in batch.
   *
   * @param bikes - Array of bike positions to validate
   * @returns Array of validation results
   * @throws Error if WASM not initialized
   */
  validateBikeDataBatch(bikes: BikePosition[]): ValidationResult[] {
    this.ensureInitialized();
    try {
      return this.wasmModule!.validateBikeDataBatch(bikes);
    } catch (error) {
      throw this.wrapWasmError('validateBikeDataBatch', error);
    }
  }

  // ==========================================================================
  // Geographic Calculations
  // ==========================================================================

  /**
   * Calculate the distance between two geographic coordinates.
   *
   * Uses the Haversine formula for accurate great-circle distance.
   *
   * @param from - Starting coordinate
   * @param to - Ending coordinate
   * @returns Distance in km, miles, and bearing in degrees
   * @throws Error if WASM not initialized
   */
  calculateDistance(from: Coordinate, to: Coordinate): DistanceResult {
    this.ensureInitialized();
    try {
      return this.wasmModule!.calculateDistance(from, to);
    } catch (error) {
      throw this.wrapWasmError('calculateDistance', error);
    }
  }

  /**
   * Calculate the distance from a bike to a target coordinate.
   *
   * @param bike - Bike position
   * @param target - Target coordinate
   * @returns Distance result
   * @throws Error if WASM not initialized
   */
  calculateBikeDistance(bike: BikePosition, target: Coordinate): DistanceResult {
    this.ensureInitialized();
    try {
      return this.wasmModule!.calculateBikeDistance(bike, target);
    } catch (error) {
      throw this.wrapWasmError('calculateBikeDistance', error);
    }
  }

  /**
   * Find the nearest bike to a given coordinate.
   *
   * @param bikes - Array of bike positions to search
   * @param target - Target coordinate
   * @returns The nearest bike
   * @throws Error if WASM not initialized or no bikes provided
   */
  findNearestBike(bikes: BikePosition[], target: Coordinate): BikePosition {
    this.ensureInitialized();
    try {
      return this.wasmModule!.findNearestBike(bikes, target);
    } catch (error) {
      throw this.wrapWasmError('findNearestBike', error);
    }
  }

  /**
   * Find all bikes within a given radius of a coordinate.
   *
   * @param bikes - Array of bike positions to search
   * @param center - Center coordinate
   * @param radiusKm - Radius in kilometers
   * @returns Array of bikes within the radius
   * @throws Error if WASM not initialized
   */
  findBikesInRadius(bikes: BikePosition[], center: Coordinate, radiusKm: number): BikePosition[] {
    this.ensureInitialized();
    try {
      return this.wasmModule!.findBikesInRadius(bikes, center, radiusKm);
    } catch (error) {
      throw this.wrapWasmError('findBikesInRadius', error);
    }
  }

  // ==========================================================================
  // Simulation Functions (NEW - migrated from TypeScript)
  // ==========================================================================

  /**
   * Simulate bike movement for one tick.
   *
   * Applies realistic movement physics:
   * - Idle bikes drift slightly (GPS jitter)
   * - Active bikes move purposefully
   * - Positions clamped to Amsterdam bounds
   *
   * @param bikes - Array of current bike positions
   * @param seed - Random seed for deterministic movement (use Date.now())
   * @returns SimulationResult with updated positions
   */
  simulateBikeMovement(bikes: BikePosition[], seed: number): SimulationResult {
    this.ensureInitialized();
    try {
      return this.wasmModule!.simulateBikeMovement(bikes, seed);
    } catch (error) {
      throw this.wrapWasmError('simulateBikeMovement', error);
    }
  }

  /**
   * Determine next status using Markov chain transition probabilities.
   *
   * Transition probabilities:
   * - Delivering: 70% stay, 15% returning, 15% idle
   * - Returning: 10% delivering, 65% stay, 25% idle
   * - Idle: 30% delivering, 10% returning, 60% stay
   *
   * @param currentStatus - Current bike status
   * @param randomValue - Random value 0.0-1.0 (use Math.random())
   * @returns StatusTransitionResult with new status
   */
  transitionBikeStatus(currentStatus: string, randomValue: number): StatusTransitionResult {
    this.ensureInitialized();
    try {
      return this.wasmModule!.transitionBikeStatus(currentStatus, randomValue);
    } catch (error) {
      throw this.wrapWasmError('transitionBikeStatus', error);
    }
  }

  /**
   * Batch transition statuses for multiple bikes.
   *
   * @param statuses - Array of current status strings
   * @param randomValues - Array of random values (same length)
   * @returns Array of StatusTransitionResult
   */
  transitionBikeStatusBatch(statuses: string[], randomValues: number[]): StatusTransitionResult[] {
    this.ensureInitialized();
    try {
      return this.wasmModule!.transitionBikeStatusBatch(statuses, randomValues);
    } catch (error) {
      throw this.wrapWasmError('transitionBikeStatusBatch', error);
    }
  }

  /**
   * Calculate bike speed based on status and traffic conditions.
   *
   * Speed ranges:
   * - Delivering: 15-35 km/h
   * - Returning: 10-25 km/h
   * - Idle: 0 km/h
   * - Traffic penalty: 40% reduction
   *
   * @param status - Current bike status
   * @param isInTraffic - Whether bike is in traffic zone
   * @param randomFactor - Random 0.0-1.0 for speed variation
   * @returns SpeedResult with calculated speed
   */
  calculateBikeSpeed(status: string, isInTraffic: boolean, randomFactor: number): SpeedResult {
    this.ensureInitialized();
    try {
      return this.wasmModule!.calculateBikeSpeed(status, isInTraffic, randomFactor);
    } catch (error) {
      throw this.wrapWasmError('calculateBikeSpeed', error);
    }
  }

  /**
   * Calculate speeds for multiple bikes at once.
   *
   * @param statuses - Array of status strings
   * @param inTraffic - Array of traffic booleans
   * @param randomFactors - Array of random factors
   * @returns Array of speeds (numbers)
   */
  calculateBikeSpeedBatch(statuses: string[], inTraffic: boolean[], randomFactors: number[]): number[] {
    this.ensureInitialized();
    try {
      return this.wasmModule!.calculateBikeSpeedBatch(statuses, inTraffic, randomFactors);
    } catch (error) {
      throw this.wrapWasmError('calculateBikeSpeedBatch', error);
    }
  }

  /**
   * Fast hash of bike positions for change detection.
   *
   * Uses FNV-1a algorithm for deck.gl updateTriggers.
   *
   * @param bikes - Array of bike positions
   * @returns 32-bit hash value
   */
  hashBikePositions(bikes: BikePosition[]): number {
    this.ensureInitialized();
    try {
      return this.wasmModule!.hashBikePositions(bikes);
    } catch (error) {
      throw this.wrapWasmError('hashBikePositions', error);
    }
  }

  /**
   * Hash bike state including status and speed.
   *
   * More comprehensive than hashBikePositions.
   *
   * @param bikes - Array of bike positions
   * @returns 32-bit hash value
   */
  hashBikeState(bikes: BikePosition[]): number {
    this.ensureInitialized();
    try {
      return this.wasmModule!.hashBikeState(bikes);
    } catch (error) {
      throw this.wrapWasmError('hashBikeState', error);
    }
  }

  /**
   * Perform a complete simulation tick.
   *
   * This is the main entry point that combines:
   * 1. Position movement
   * 2. Status transitions
   * 3. Speed calculation
   * 4. Statistics calculation
   * 5. Hash computation
   *
   * @param bikes - Current bike positions
   * @param timestamp - Current timestamp (seed for determinism)
   * @param transitionProbability - Probability of status change (0.0-1.0)
   * @returns SimulationTickResult with all updated data
   */
  simulationTick(bikes: BikePosition[], timestamp: number, transitionProbability: number): SimulationTickResult {
    this.ensureInitialized();
    try {
      return this.wasmModule!.simulationTick(bikes, timestamp, transitionProbability);
    } catch (error) {
      throw this.wrapWasmError('simulationTick', error);
    }
  }

  // ==========================================================================
  // Utility Methods
  // ==========================================================================

  /**
   * Wrap WASM errors with more context
   */
  private wrapWasmError(functionName: string, error: unknown): Error {
    const message = error instanceof Error ? error.message : String(error);
    return new Error(`WASM ${functionName} failed: ${message}`);
  }

  /**
   * Check if WASM is supported in the current environment
   */
  static isWasmSupported(): boolean {
    try {
      if (typeof WebAssembly === 'object' &&
          typeof WebAssembly.instantiate === 'function') {
        const module = new WebAssembly.Module(
          Uint8Array.of(0x0, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00)
        );
        if (module instanceof WebAssembly.Module) {
          return new WebAssembly.Instance(module) instanceof WebAssembly.Instance;
        }
      }
    } catch (e) {
      // WASM not supported
    }
    return false;
  }
}
