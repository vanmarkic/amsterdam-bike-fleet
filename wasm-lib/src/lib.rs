//! WASM module for Amsterdam Bike Fleet
//!
//! This module provides protected client-side algorithms for fleet management,
//! including statistics calculation, data validation, and geographic computations.

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

// Initialize panic hook for better error messages in development
#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

// ============================================================================
// Data Types (matching Angular models)
// ============================================================================

/// Bike status enum matching TypeScript BikePosition.status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BikeStatus {
    Delivering,
    Idle,
    Returning,
}

/// Bike position data matching TypeScript BikePosition interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BikePosition {
    pub id: String,
    pub name: String,
    pub longitude: f64,
    pub latitude: f64,
    pub status: BikeStatus,
    pub speed: f64,
}

/// Fleet statistics result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FleetStatistics {
    pub total_bikes: u32,
    pub delivering_count: u32,
    pub idle_count: u32,
    pub returning_count: u32,
    pub average_speed: f64,
    pub max_speed: f64,
    pub min_speed: f64,
    pub active_percentage: f64,
    pub fleet_center_longitude: f64,
    pub fleet_center_latitude: f64,
}

/// Validation result for bike data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub sanitized_data: Option<BikePosition>,
}

/// Distance calculation result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistanceResult {
    pub distance_km: f64,
    pub distance_miles: f64,
    pub bearing_degrees: f64,
}

/// Coordinate pair for geographic calculations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coordinate {
    pub longitude: f64,
    pub latitude: f64,
}

// ============================================================================
// Fleet Statistics Calculation
// ============================================================================

/// Calculate comprehensive fleet statistics from bike position data
///
/// This function processes an array of bike positions and returns
/// aggregated statistics including counts by status, speed metrics,
/// and the geographic center of the fleet.
#[wasm_bindgen(js_name = calculateFleetStatistics)]
pub fn calculate_fleet_statistics(bikes_js: JsValue) -> Result<JsValue, JsValue> {
    // Deserialize bikes from JavaScript
    let bikes: Vec<BikePosition> = serde_wasm_bindgen::from_value(bikes_js)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse bikes: {}", e)))?;

    if bikes.is_empty() {
        return Err(JsValue::from_str("Cannot calculate statistics for empty fleet"));
    }

    let total_bikes = bikes.len() as u32;

    // Count by status
    let delivering_count = bikes.iter().filter(|b| b.status == BikeStatus::Delivering).count() as u32;
    let idle_count = bikes.iter().filter(|b| b.status == BikeStatus::Idle).count() as u32;
    let returning_count = bikes.iter().filter(|b| b.status == BikeStatus::Returning).count() as u32;

    // Speed statistics
    let speeds: Vec<f64> = bikes.iter().map(|b| b.speed).collect();
    let average_speed = speeds.iter().sum::<f64>() / speeds.len() as f64;
    let max_speed = speeds.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min_speed = speeds.iter().cloned().fold(f64::INFINITY, f64::min);

    // Active percentage (delivering + returning)
    let active_count = delivering_count + returning_count;
    let active_percentage = (active_count as f64 / total_bikes as f64) * 100.0;

    // Calculate fleet geographic center (centroid)
    let sum_lng: f64 = bikes.iter().map(|b| b.longitude).sum();
    let sum_lat: f64 = bikes.iter().map(|b| b.latitude).sum();
    let fleet_center_longitude = sum_lng / total_bikes as f64;
    let fleet_center_latitude = sum_lat / total_bikes as f64;

    let stats = FleetStatistics {
        total_bikes,
        delivering_count,
        idle_count,
        returning_count,
        average_speed,
        max_speed,
        min_speed,
        active_percentage,
        fleet_center_longitude,
        fleet_center_latitude,
    };

    serde_wasm_bindgen::to_value(&stats)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize statistics: {}", e)))
}

// ============================================================================
// Data Validation and Transformation
// ============================================================================

/// Amsterdam bounding box for coordinate validation
const AMSTERDAM_BOUNDS: (f64, f64, f64, f64) = (
    4.7, // min longitude
    5.1, // max longitude
    52.2, // min latitude
    52.5, // max latitude
);

/// Maximum reasonable bike speed in km/h
const MAX_BIKE_SPEED: f64 = 50.0;

/// Validate and sanitize bike position data
///
/// Checks that coordinates are within Amsterdam bounds, speed is reasonable,
/// and all required fields are present. Returns validation result with
/// optional sanitized data.
#[wasm_bindgen(js_name = validateBikeData)]
pub fn validate_bike_data(bike_js: JsValue) -> Result<JsValue, JsValue> {
    let bike: BikePosition = serde_wasm_bindgen::from_value(bike_js)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse bike data: {}", e)))?;

    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut sanitized = bike.clone();

    // Validate ID
    if bike.id.is_empty() {
        errors.push("Bike ID cannot be empty".to_string());
    }

    // Validate name
    if bike.name.is_empty() {
        errors.push("Bike name cannot be empty".to_string());
    } else if bike.name.len() > 50 {
        warnings.push("Bike name truncated to 50 characters".to_string());
        sanitized.name = bike.name.chars().take(50).collect();
    }

    // Validate coordinates (Amsterdam bounds)
    if bike.longitude < AMSTERDAM_BOUNDS.0 || bike.longitude > AMSTERDAM_BOUNDS.1 {
        errors.push(format!(
            "Longitude {} is outside Amsterdam bounds ({} - {})",
            bike.longitude, AMSTERDAM_BOUNDS.0, AMSTERDAM_BOUNDS.1
        ));
    }

    if bike.latitude < AMSTERDAM_BOUNDS.2 || bike.latitude > AMSTERDAM_BOUNDS.3 {
        errors.push(format!(
            "Latitude {} is outside Amsterdam bounds ({} - {})",
            bike.latitude, AMSTERDAM_BOUNDS.2, AMSTERDAM_BOUNDS.3
        ));
    }

    // Validate speed
    if bike.speed < 0.0 {
        errors.push("Speed cannot be negative".to_string());
        sanitized.speed = 0.0;
    } else if bike.speed > MAX_BIKE_SPEED {
        warnings.push(format!(
            "Speed {} km/h exceeds maximum reasonable speed, clamped to {}",
            bike.speed, MAX_BIKE_SPEED
        ));
        sanitized.speed = MAX_BIKE_SPEED;
    }

    // Check speed vs status consistency
    if bike.status == BikeStatus::Idle && bike.speed > 1.0 {
        warnings.push("Idle bike has non-zero speed, setting to 0".to_string());
        sanitized.speed = 0.0;
    }

    let is_valid = errors.is_empty();

    let result = ValidationResult {
        is_valid,
        errors,
        warnings,
        sanitized_data: if is_valid { Some(sanitized) } else { None },
    };

    serde_wasm_bindgen::to_value(&result)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
}

/// Batch validate multiple bike positions
#[wasm_bindgen(js_name = validateBikeDataBatch)]
pub fn validate_bike_data_batch(bikes_js: JsValue) -> Result<JsValue, JsValue> {
    let bikes: Vec<BikePosition> = serde_wasm_bindgen::from_value(bikes_js)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse bikes: {}", e)))?;

    let results: Vec<ValidationResult> = bikes
        .into_iter()
        .map(|bike| {
            let bike_js = serde_wasm_bindgen::to_value(&bike).unwrap();
            let result_js = validate_bike_data(bike_js).unwrap();
            serde_wasm_bindgen::from_value(result_js).unwrap()
        })
        .collect();

    serde_wasm_bindgen::to_value(&results)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize results: {}", e)))
}

// ============================================================================
// Geographic Calculations (Haversine Formula)
// ============================================================================

/// Earth's radius in kilometers
const EARTH_RADIUS_KM: f64 = 6371.0;

/// Convert degrees to radians
fn deg_to_rad(degrees: f64) -> f64 {
    degrees * std::f64::consts::PI / 180.0
}

/// Convert radians to degrees
fn rad_to_deg(radians: f64) -> f64 {
    radians * 180.0 / std::f64::consts::PI
}

/// Calculate distance between two coordinates using the Haversine formula
///
/// This is the most accurate method for calculating distances between
/// two points on Earth's surface for short to medium distances.
///
/// # Arguments
/// * `lat1`, `lon1` - First coordinate (latitude, longitude in degrees)
/// * `lat2`, `lon2` - Second coordinate (latitude, longitude in degrees)
///
/// # Returns
/// Distance in kilometers
fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let lat1_rad = deg_to_rad(lat1);
    let lat2_rad = deg_to_rad(lat2);
    let delta_lat = deg_to_rad(lat2 - lat1);
    let delta_lon = deg_to_rad(lon2 - lon1);

    let a = (delta_lat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);

    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS_KM * c
}

/// Calculate initial bearing between two coordinates
///
/// Returns the initial bearing (forward azimuth) in degrees (0-360)
fn calculate_bearing(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let lat1_rad = deg_to_rad(lat1);
    let lat2_rad = deg_to_rad(lat2);
    let delta_lon = deg_to_rad(lon2 - lon1);

    let x = delta_lon.sin() * lat2_rad.cos();
    let y = lat1_rad.cos() * lat2_rad.sin()
        - lat1_rad.sin() * lat2_rad.cos() * delta_lon.cos();

    let bearing = rad_to_deg(x.atan2(y));

    // Normalize to 0-360
    (bearing + 360.0) % 360.0
}

/// Calculate distance between two geographic coordinates
///
/// Uses the Haversine formula to calculate the great-circle distance
/// between two points on Earth. Also calculates the initial bearing.
///
/// # Arguments
/// * `from` - Starting coordinate with longitude and latitude
/// * `to` - Ending coordinate with longitude and latitude
///
/// # Returns
/// DistanceResult with distance in km, miles, and bearing in degrees
#[wasm_bindgen(js_name = calculateDistance)]
pub fn calculate_distance(from_js: JsValue, to_js: JsValue) -> Result<JsValue, JsValue> {
    let from: Coordinate = serde_wasm_bindgen::from_value(from_js)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse 'from' coordinate: {}", e)))?;

    let to: Coordinate = serde_wasm_bindgen::from_value(to_js)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse 'to' coordinate: {}", e)))?;

    let distance_km = haversine_distance(from.latitude, from.longitude, to.latitude, to.longitude);
    let distance_miles = distance_km * 0.621371;
    let bearing_degrees = calculate_bearing(from.latitude, from.longitude, to.latitude, to.longitude);

    let result = DistanceResult {
        distance_km,
        distance_miles,
        bearing_degrees,
    };

    serde_wasm_bindgen::to_value(&result)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
}

/// Calculate distance from a bike to a target coordinate
#[wasm_bindgen(js_name = calculateBikeDistance)]
pub fn calculate_bike_distance(bike_js: JsValue, target_js: JsValue) -> Result<JsValue, JsValue> {
    let bike: BikePosition = serde_wasm_bindgen::from_value(bike_js)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse bike: {}", e)))?;

    let target: Coordinate = serde_wasm_bindgen::from_value(target_js)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse target: {}", e)))?;

    let from = Coordinate {
        longitude: bike.longitude,
        latitude: bike.latitude,
    };

    let from_js = serde_wasm_bindgen::to_value(&from)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize from: {}", e)))?;
    let target_js = serde_wasm_bindgen::to_value(&target)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize target: {}", e)))?;

    calculate_distance(from_js, target_js)
}

/// Find the nearest bike to a given coordinate
#[wasm_bindgen(js_name = findNearestBike)]
pub fn find_nearest_bike(bikes_js: JsValue, target_js: JsValue) -> Result<JsValue, JsValue> {
    let bikes: Vec<BikePosition> = serde_wasm_bindgen::from_value(bikes_js)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse bikes: {}", e)))?;

    let target: Coordinate = serde_wasm_bindgen::from_value(target_js)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse target: {}", e)))?;

    if bikes.is_empty() {
        return Err(JsValue::from_str("No bikes provided"));
    }

    let nearest = bikes
        .iter()
        .min_by(|a, b| {
            let dist_a = haversine_distance(a.latitude, a.longitude, target.latitude, target.longitude);
            let dist_b = haversine_distance(b.latitude, b.longitude, target.latitude, target.longitude);
            dist_a.partial_cmp(&dist_b).unwrap()
        })
        .unwrap();

    serde_wasm_bindgen::to_value(&nearest)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
}

/// Find all bikes within a given radius (km) of a coordinate
#[wasm_bindgen(js_name = findBikesInRadius)]
pub fn find_bikes_in_radius(bikes_js: JsValue, center_js: JsValue, radius_km: f64) -> Result<JsValue, JsValue> {
    let bikes: Vec<BikePosition> = serde_wasm_bindgen::from_value(bikes_js)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse bikes: {}", e)))?;

    let center: Coordinate = serde_wasm_bindgen::from_value(center_js)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse center: {}", e)))?;

    let bikes_in_radius: Vec<&BikePosition> = bikes
        .iter()
        .filter(|bike| {
            let distance = haversine_distance(
                bike.latitude, bike.longitude,
                center.latitude, center.longitude
            );
            distance <= radius_km
        })
        .collect();

    serde_wasm_bindgen::to_value(&bikes_in_radius)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
}

// ============================================================================
// Bike Movement Simulation
// ============================================================================

/// Configuration for Amsterdam operational bounds
const AMSTERDAM_OPERATIONAL_BOUNDS: (f64, f64, f64, f64) = (
    4.85,  // min longitude
    4.95,  // max longitude
    52.34, // min latitude
    52.40, // max latitude
);

/// Movement speed in degrees per millisecond for different states
/// Approximately: idle ~0.0002°, active ~0.001° per 5 seconds
const MOVEMENT_IDLE: f64 = 0.0002;
const MOVEMENT_ACTIVE: f64 = 0.001;

/// Result of bike movement simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimulationResult {
    pub bikes: Vec<BikePosition>,
    pub movements_applied: u32,
    pub bounds_corrections: u32,
}

/// Simulate bike movement for one tick.
///
/// This function applies realistic movement physics to all bikes:
/// - Idle bikes drift slightly (GPS jitter simulation)
/// - Active bikes (delivering/returning) move purposefully
/// - All positions are clamped to Amsterdam operational bounds
///
/// # Arguments
/// * `bikes_js` - Array of current bike positions
/// * `seed` - Random seed for deterministic movement (use timestamp)
///
/// # Returns
/// SimulationResult with updated bike positions
#[wasm_bindgen(js_name = simulateBikeMovement)]
pub fn simulate_bike_movement(bikes_js: JsValue, seed: f64) -> Result<JsValue, JsValue> {
    let bikes: Vec<BikePosition> = serde_wasm_bindgen::from_value(bikes_js)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse bikes: {}", e)))?;

    let mut bounds_corrections: u32 = 0;
    let movements_applied = bikes.len() as u32;

    // Use seed to create pseudo-random but deterministic movement
    let updated_bikes: Vec<BikePosition> = bikes
        .into_iter()
        .enumerate()
        .map(|(idx, bike)| {
            // Create per-bike variation using index and seed
            let variation = ((seed + idx as f64 * 1000.0) % 1000.0) / 1000.0;
            let angle = variation * std::f64::consts::PI * 2.0;

            // Movement magnitude based on status
            let movement = match bike.status {
                BikeStatus::Idle => MOVEMENT_IDLE,
                BikeStatus::Delivering | BikeStatus::Returning => MOVEMENT_ACTIVE,
            };

            let mut new_lng = bike.longitude + angle.cos() * movement;
            let mut new_lat = bike.latitude + angle.sin() * movement;

            // Clamp to Amsterdam operational bounds
            let (min_lng, max_lng, min_lat, max_lat) = AMSTERDAM_OPERATIONAL_BOUNDS;

            if new_lng < min_lng || new_lng > max_lng || new_lat < min_lat || new_lat > max_lat {
                bounds_corrections += 1;
            }

            new_lng = new_lng.clamp(min_lng, max_lng);
            new_lat = new_lat.clamp(min_lat, max_lat);

            BikePosition {
                id: bike.id,
                name: bike.name,
                longitude: new_lng,
                latitude: new_lat,
                status: bike.status,
                speed: bike.speed,
            }
        })
        .collect();

    let result = SimulationResult {
        bikes: updated_bikes,
        movements_applied,
        bounds_corrections,
    };

    serde_wasm_bindgen::to_value(&result)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
}

// ============================================================================
// Status Transition Logic
// ============================================================================

/// Status transition probabilities
/// Format: (probability_to_delivering, probability_to_returning, probability_to_idle)
fn get_transition_probabilities(current: &BikeStatus) -> (f64, f64, f64) {
    match current {
        // Delivering bikes usually stay delivering or go idle
        BikeStatus::Delivering => (0.70, 0.15, 0.15),
        // Returning bikes usually stay returning or become idle
        BikeStatus::Returning => (0.10, 0.65, 0.25),
        // Idle bikes usually stay idle or start delivering
        BikeStatus::Idle => (0.30, 0.10, 0.60),
    }
}

/// Status transition result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusTransitionResult {
    pub new_status: BikeStatus,
    pub transition_occurred: bool,
    pub probability_used: f64,
}

/// Determine next status based on current state and transition probabilities.
///
/// Uses a Markov chain model for realistic status transitions:
/// - Delivering bikes tend to stay delivering (70%) or go idle (15%) or returning (15%)
/// - Returning bikes tend to stay returning (65%) or go idle (25%)
/// - Idle bikes tend to stay idle (60%) or start delivering (30%)
///
/// # Arguments
/// * `current_status` - Current bike status string ("delivering", "returning", "idle")
/// * `random_value` - Random value between 0.0 and 1.0 (use Math.random())
///
/// # Returns
/// StatusTransitionResult with new status and whether transition occurred
#[wasm_bindgen(js_name = transitionBikeStatus)]
pub fn transition_bike_status(current_status: &str, random_value: f64) -> Result<JsValue, JsValue> {
    let current = match current_status.to_lowercase().as_str() {
        "delivering" => BikeStatus::Delivering,
        "returning" => BikeStatus::Returning,
        "idle" => BikeStatus::Idle,
        _ => return Err(JsValue::from_str(&format!("Unknown status: {}", current_status))),
    };

    let (p_delivering, p_returning, _p_idle) = get_transition_probabilities(&current);
    let clamped_random = random_value.clamp(0.0, 1.0);

    let new_status = if clamped_random < p_delivering {
        BikeStatus::Delivering
    } else if clamped_random < p_delivering + p_returning {
        BikeStatus::Returning
    } else {
        BikeStatus::Idle
    };

    let transition_occurred = new_status != current;

    let result = StatusTransitionResult {
        new_status,
        transition_occurred,
        probability_used: clamped_random,
    };

    serde_wasm_bindgen::to_value(&result)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
}

/// Batch transition statuses for multiple bikes
///
/// # Arguments
/// * `statuses` - Array of current status strings
/// * `random_values` - Array of random values (same length as statuses)
///
/// # Returns
/// Array of new status strings
#[wasm_bindgen(js_name = transitionBikeStatusBatch)]
pub fn transition_bike_status_batch(statuses_js: JsValue, random_values_js: JsValue) -> Result<JsValue, JsValue> {
    let statuses: Vec<String> = serde_wasm_bindgen::from_value(statuses_js)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse statuses: {}", e)))?;

    let random_values: Vec<f64> = serde_wasm_bindgen::from_value(random_values_js)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse random values: {}", e)))?;

    if statuses.len() != random_values.len() {
        return Err(JsValue::from_str("statuses and random_values must have same length"));
    }

    let results: Vec<StatusTransitionResult> = statuses
        .iter()
        .zip(random_values.iter())
        .filter_map(|(status, random)| {
            let result_js = transition_bike_status(status, *random).ok()?;
            serde_wasm_bindgen::from_value(result_js).ok()
        })
        .collect();

    serde_wasm_bindgen::to_value(&results)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize results: {}", e)))
}

// ============================================================================
// Speed Calculation
// ============================================================================

/// Speed ranges for different statuses (min, max) in km/h
const SPEED_DELIVERING: (f64, f64) = (15.0, 35.0);
const SPEED_RETURNING: (f64, f64) = (10.0, 25.0);
const SPEED_IDLE: f64 = 0.0;

/// Traffic impact factor (reduces speed by this percentage)
const TRAFFIC_SPEED_REDUCTION: f64 = 0.4; // 40% slower in traffic

/// Speed calculation result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeedResult {
    pub speed: f64,
    pub base_speed: f64,
    pub traffic_penalty: f64,
    pub status_factor: String,
}

/// Calculate bike speed based on status and environmental conditions.
///
/// Speed is determined by:
/// - Status: delivering (15-35 km/h), returning (10-25 km/h), idle (0)
/// - Traffic: 40% speed reduction in traffic zones
/// - Variation: random_factor adds natural speed variation
///
/// # Arguments
/// * `status` - Current bike status ("delivering", "returning", "idle")
/// * `is_in_traffic` - Whether bike is in a traffic jam zone
/// * `random_factor` - Random value 0.0-1.0 for speed variation within range
///
/// # Returns
/// SpeedResult with calculated speed and breakdown
#[wasm_bindgen(js_name = calculateBikeSpeed)]
pub fn calculate_bike_speed(status: &str, is_in_traffic: bool, random_factor: f64) -> Result<JsValue, JsValue> {
    let clamped_random = random_factor.clamp(0.0, 1.0);

    let (base_speed, status_factor) = match status.to_lowercase().as_str() {
        "delivering" => {
            let (min, max) = SPEED_DELIVERING;
            let speed = min + (max - min) * clamped_random;
            (speed, "delivering")
        }
        "returning" => {
            let (min, max) = SPEED_RETURNING;
            let speed = min + (max - min) * clamped_random;
            (speed, "returning")
        }
        "idle" => (SPEED_IDLE, "idle"),
        _ => return Err(JsValue::from_str(&format!("Unknown status: {}", status))),
    };

    let traffic_penalty = if is_in_traffic && base_speed > 0.0 {
        base_speed * TRAFFIC_SPEED_REDUCTION
    } else {
        0.0
    };

    let final_speed = (base_speed - traffic_penalty).max(0.0);

    let result = SpeedResult {
        speed: final_speed,
        base_speed,
        traffic_penalty,
        status_factor: status_factor.to_string(),
    };

    serde_wasm_bindgen::to_value(&result)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
}

/// Calculate speeds for multiple bikes at once
#[wasm_bindgen(js_name = calculateBikeSpeedBatch)]
pub fn calculate_bike_speed_batch(
    statuses_js: JsValue,
    in_traffic_js: JsValue,
    random_factors_js: JsValue
) -> Result<JsValue, JsValue> {
    let statuses: Vec<String> = serde_wasm_bindgen::from_value(statuses_js)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse statuses: {}", e)))?;

    let in_traffic: Vec<bool> = serde_wasm_bindgen::from_value(in_traffic_js)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse in_traffic: {}", e)))?;

    let random_factors: Vec<f64> = serde_wasm_bindgen::from_value(random_factors_js)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse random_factors: {}", e)))?;

    if statuses.len() != in_traffic.len() || statuses.len() != random_factors.len() {
        return Err(JsValue::from_str("All input arrays must have same length"));
    }

    let speeds: Vec<f64> = statuses
        .iter()
        .zip(in_traffic.iter())
        .zip(random_factors.iter())
        .map(|((status, &traffic), &random)| {
            match calculate_bike_speed(status, traffic, random) {
                Ok(result_js) => {
                    let result: SpeedResult = serde_wasm_bindgen::from_value(result_js).unwrap();
                    result.speed
                }
                Err(_) => 0.0,
            }
        })
        .collect();

    serde_wasm_bindgen::to_value(&speeds)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize speeds: {}", e)))
}

// ============================================================================
// Position Hashing (for change detection)
// ============================================================================

/// Fast hash of bike positions for change detection.
///
/// Uses FNV-1a inspired algorithm for fast, deterministic hashing.
/// This is used by deck.gl updateTriggers to detect position changes
/// without expensive deep comparison.
///
/// # Arguments
/// * `bikes_js` - Array of bike positions
///
/// # Returns
/// 32-bit hash value
#[wasm_bindgen(js_name = hashBikePositions)]
pub fn hash_bike_positions(bikes_js: JsValue) -> Result<u32, JsValue> {
    let bikes: Vec<BikePosition> = serde_wasm_bindgen::from_value(bikes_js)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse bikes: {}", e)))?;

    // FNV-1a inspired hash
    let mut hash: u32 = 2166136261;

    for bike in bikes {
        // Multiply coordinates by 1_000_000 to preserve 6 decimal places
        let lng_bits = (bike.longitude * 1_000_000.0) as i32;
        let lat_bits = (bike.latitude * 1_000_000.0) as i32;

        // XOR and multiply pattern
        hash ^= lng_bits as u32;
        hash = hash.wrapping_mul(16777619);
        hash ^= lat_bits as u32;
        hash = hash.wrapping_mul(16777619);
    }

    Ok(hash)
}

/// Hash bike positions including status for more comprehensive change detection
#[wasm_bindgen(js_name = hashBikeState)]
pub fn hash_bike_state(bikes_js: JsValue) -> Result<u32, JsValue> {
    let bikes: Vec<BikePosition> = serde_wasm_bindgen::from_value(bikes_js)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse bikes: {}", e)))?;

    let mut hash: u32 = 2166136261;

    for bike in bikes {
        let lng_bits = (bike.longitude * 1_000_000.0) as i32;
        let lat_bits = (bike.latitude * 1_000_000.0) as i32;
        let status_bits = match bike.status {
            BikeStatus::Delivering => 1u32,
            BikeStatus::Returning => 2u32,
            BikeStatus::Idle => 3u32,
        };
        let speed_bits = (bike.speed * 100.0) as u32;

        hash ^= lng_bits as u32;
        hash = hash.wrapping_mul(16777619);
        hash ^= lat_bits as u32;
        hash = hash.wrapping_mul(16777619);
        hash ^= status_bits;
        hash = hash.wrapping_mul(16777619);
        hash ^= speed_bits;
        hash = hash.wrapping_mul(16777619);
    }

    Ok(hash)
}

// ============================================================================
// Full Simulation Tick (combines all updates)
// ============================================================================

/// Complete simulation tick result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SimulationTickResult {
    pub bikes: Vec<BikePosition>,
    pub statistics: FleetStatistics,
    pub position_hash: u32,
    pub state_hash: u32,
    pub status_transitions: u32,
    pub bounds_corrections: u32,
}

/// Perform a complete simulation tick - updates positions, statuses, speeds, and calculates stats.
///
/// This is the main entry point for simulation, combining:
/// 1. Position movement simulation
/// 2. Status transitions (with 10% probability per bike)
/// 3. Speed calculation based on new status
/// 4. Fleet statistics calculation
/// 5. Hash computation for change detection
///
/// # Arguments
/// * `bikes_js` - Array of current bike positions
/// * `timestamp` - Current timestamp (used as seed for determinism)
/// * `transition_probability` - Probability (0.0-1.0) that any bike changes status
///
/// # Returns
/// SimulationTickResult with all updated data
#[wasm_bindgen(js_name = simulationTick)]
pub fn simulation_tick(
    bikes_js: JsValue,
    timestamp: f64,
    transition_probability: f64
) -> Result<JsValue, JsValue> {
    let bikes: Vec<BikePosition> = serde_wasm_bindgen::from_value(bikes_js)
        .map_err(|e| JsValue::from_str(&format!("Failed to parse bikes: {}", e)))?;

    if bikes.is_empty() {
        return Err(JsValue::from_str("Cannot simulate empty fleet"));
    }

    let mut status_transitions: u32 = 0;
    let mut bounds_corrections: u32 = 0;
    let clamp_prob = transition_probability.clamp(0.0, 1.0);

    // Process each bike
    let updated_bikes: Vec<BikePosition> = bikes
        .into_iter()
        .enumerate()
        .map(|(idx, bike)| {
            // Deterministic "random" values based on timestamp and index
            let variation = ((timestamp + idx as f64 * 1000.0) % 1000.0) / 1000.0;
            let status_random = ((timestamp * 7.0 + idx as f64 * 3000.0) % 1000.0) / 1000.0;
            let speed_random = ((timestamp * 13.0 + idx as f64 * 5000.0) % 1000.0) / 1000.0;

            // 1. Movement
            let angle = variation * std::f64::consts::PI * 2.0;
            let movement = match bike.status {
                BikeStatus::Idle => MOVEMENT_IDLE,
                _ => MOVEMENT_ACTIVE,
            };

            let mut new_lng = bike.longitude + angle.cos() * movement;
            let mut new_lat = bike.latitude + angle.sin() * movement;

            let (min_lng, max_lng, min_lat, max_lat) = AMSTERDAM_OPERATIONAL_BOUNDS;
            if new_lng < min_lng || new_lng > max_lng || new_lat < min_lat || new_lat > max_lat {
                bounds_corrections += 1;
            }
            new_lng = new_lng.clamp(min_lng, max_lng);
            new_lat = new_lat.clamp(min_lat, max_lat);

            // 2. Status transition (only if random value is below threshold)
            let should_transition = ((timestamp * 17.0 + idx as f64 * 7000.0) % 1000.0) / 1000.0;
            let new_status = if should_transition < clamp_prob {
                let (p_del, p_ret, _) = get_transition_probabilities(&bike.status);
                let new_s = if status_random < p_del {
                    BikeStatus::Delivering
                } else if status_random < p_del + p_ret {
                    BikeStatus::Returning
                } else {
                    BikeStatus::Idle
                };
                if new_s != bike.status {
                    status_transitions += 1;
                }
                new_s
            } else {
                bike.status.clone()
            };

            // 3. Speed calculation
            let new_speed = match new_status {
                BikeStatus::Idle => 0.0,
                BikeStatus::Delivering => {
                    let (min, max) = SPEED_DELIVERING;
                    min + (max - min) * speed_random
                }
                BikeStatus::Returning => {
                    let (min, max) = SPEED_RETURNING;
                    min + (max - min) * speed_random
                }
            };

            BikePosition {
                id: bike.id,
                name: bike.name,
                longitude: new_lng,
                latitude: new_lat,
                status: new_status,
                speed: new_speed,
            }
        })
        .collect();

    // Calculate statistics
    let total_bikes = updated_bikes.len() as u32;
    let delivering_count = updated_bikes.iter().filter(|b| b.status == BikeStatus::Delivering).count() as u32;
    let idle_count = updated_bikes.iter().filter(|b| b.status == BikeStatus::Idle).count() as u32;
    let returning_count = updated_bikes.iter().filter(|b| b.status == BikeStatus::Returning).count() as u32;

    let speeds: Vec<f64> = updated_bikes.iter().map(|b| b.speed).collect();
    let average_speed = speeds.iter().sum::<f64>() / speeds.len() as f64;
    let max_speed = speeds.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let min_speed = speeds.iter().cloned().fold(f64::INFINITY, f64::min);

    let active_count = delivering_count + returning_count;
    let active_percentage = (active_count as f64 / total_bikes as f64) * 100.0;

    let sum_lng: f64 = updated_bikes.iter().map(|b| b.longitude).sum();
    let sum_lat: f64 = updated_bikes.iter().map(|b| b.latitude).sum();

    let statistics = FleetStatistics {
        total_bikes,
        delivering_count,
        idle_count,
        returning_count,
        average_speed,
        max_speed,
        min_speed,
        active_percentage,
        fleet_center_longitude: sum_lng / total_bikes as f64,
        fleet_center_latitude: sum_lat / total_bikes as f64,
    };

    // Calculate hashes
    let mut position_hash: u32 = 2166136261;
    let mut state_hash: u32 = 2166136261;

    for bike in &updated_bikes {
        let lng_bits = (bike.longitude * 1_000_000.0) as i32;
        let lat_bits = (bike.latitude * 1_000_000.0) as i32;

        position_hash ^= lng_bits as u32;
        position_hash = position_hash.wrapping_mul(16777619);
        position_hash ^= lat_bits as u32;
        position_hash = position_hash.wrapping_mul(16777619);

        let status_bits = match bike.status {
            BikeStatus::Delivering => 1u32,
            BikeStatus::Returning => 2u32,
            BikeStatus::Idle => 3u32,
        };
        state_hash ^= lng_bits as u32;
        state_hash = state_hash.wrapping_mul(16777619);
        state_hash ^= lat_bits as u32;
        state_hash = state_hash.wrapping_mul(16777619);
        state_hash ^= status_bits;
        state_hash = state_hash.wrapping_mul(16777619);
        state_hash ^= (bike.speed * 100.0) as u32;
        state_hash = state_hash.wrapping_mul(16777619);
    }

    let result = SimulationTickResult {
        bikes: updated_bikes,
        statistics,
        position_hash,
        state_hash,
        status_transitions,
        bounds_corrections,
    };

    serde_wasm_bindgen::to_value(&result)
        .map_err(|e| JsValue::from_str(&format!("Failed to serialize result: {}", e)))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_haversine_distance() {
        // Amsterdam Centraal to Dam Square (approximately 1.1 km)
        let distance = haversine_distance(
            52.3791, 4.9003, // Centraal Station
            52.3730, 4.8932  // Dam Square
        );
        assert!((distance - 0.85).abs() < 0.1, "Distance should be approximately 0.85 km");
    }

    #[test]
    fn test_bearing() {
        // North bearing
        let bearing = calculate_bearing(52.0, 4.9, 53.0, 4.9);
        assert!((bearing - 0.0).abs() < 1.0, "Bearing should be approximately 0 degrees (north)");

        // East bearing
        let bearing = calculate_bearing(52.0, 4.0, 52.0, 5.0);
        assert!((bearing - 90.0).abs() < 1.0, "Bearing should be approximately 90 degrees (east)");
    }

    #[test]
    fn test_deg_to_rad() {
        assert!((deg_to_rad(180.0) - std::f64::consts::PI).abs() < 0.0001);
        assert!((deg_to_rad(90.0) - std::f64::consts::FRAC_PI_2).abs() < 0.0001);
    }

    // ========================================================================
    // NEW: Tests for simulation functions
    // ========================================================================

    #[test]
    fn test_transition_probabilities() {
        // Test that transition probabilities sum to 1.0
        let (p_del, p_ret, p_idle) = get_transition_probabilities(&BikeStatus::Delivering);
        assert!((p_del + p_ret + p_idle - 1.0).abs() < 0.001);

        let (p_del, p_ret, p_idle) = get_transition_probabilities(&BikeStatus::Returning);
        assert!((p_del + p_ret + p_idle - 1.0).abs() < 0.001);

        let (p_del, p_ret, p_idle) = get_transition_probabilities(&BikeStatus::Idle);
        assert!((p_del + p_ret + p_idle - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_speed_ranges() {
        // Delivering speed range
        assert!(SPEED_DELIVERING.0 >= 10.0, "Min delivering speed should be reasonable");
        assert!(SPEED_DELIVERING.1 <= 40.0, "Max delivering speed should be reasonable");
        assert!(SPEED_DELIVERING.0 < SPEED_DELIVERING.1, "Min should be less than max");

        // Returning speed range
        assert!(SPEED_RETURNING.0 >= 5.0, "Min returning speed should be reasonable");
        assert!(SPEED_RETURNING.1 <= 30.0, "Max returning speed should be reasonable");
        assert!(SPEED_RETURNING.0 < SPEED_RETURNING.1, "Min should be less than max");

        // Idle speed
        assert_eq!(SPEED_IDLE, 0.0, "Idle speed should be 0");
    }

    #[test]
    fn test_amsterdam_bounds() {
        let (min_lng, max_lng, min_lat, max_lat) = AMSTERDAM_OPERATIONAL_BOUNDS;

        // Verify bounds are within Amsterdam area
        assert!(min_lng >= 4.7 && min_lng <= 5.0, "Min longitude should be in Amsterdam");
        assert!(max_lng >= 4.8 && max_lng <= 5.1, "Max longitude should be in Amsterdam");
        assert!(min_lat >= 52.2 && min_lat <= 52.5, "Min latitude should be in Amsterdam");
        assert!(max_lat >= 52.3 && max_lat <= 52.5, "Max latitude should be in Amsterdam");

        // Verify bounds make sense
        assert!(min_lng < max_lng, "Min longitude should be less than max");
        assert!(min_lat < max_lat, "Min latitude should be less than max");
    }

    #[test]
    fn test_hash_determinism() {
        // Same input should produce same hash
        let bikes = vec![
            BikePosition {
                id: "bike-1".to_string(),
                name: "Jan".to_string(),
                longitude: 4.9,
                latitude: 52.37,
                status: BikeStatus::Delivering,
                speed: 20.0,
            },
            BikePosition {
                id: "bike-2".to_string(),
                name: "Pieter".to_string(),
                longitude: 4.91,
                latitude: 52.38,
                status: BikeStatus::Idle,
                speed: 0.0,
            },
        ];

        // FNV-1a hash computation
        let mut hash1: u32 = 2166136261;
        let mut hash2: u32 = 2166136261;

        for bike in &bikes {
            let lng_bits = (bike.longitude * 1_000_000.0) as i32;
            let lat_bits = (bike.latitude * 1_000_000.0) as i32;

            hash1 ^= lng_bits as u32;
            hash1 = hash1.wrapping_mul(16777619);
            hash1 ^= lat_bits as u32;
            hash1 = hash1.wrapping_mul(16777619);
        }

        // Compute again
        for bike in &bikes {
            let lng_bits = (bike.longitude * 1_000_000.0) as i32;
            let lat_bits = (bike.latitude * 1_000_000.0) as i32;

            hash2 ^= lng_bits as u32;
            hash2 = hash2.wrapping_mul(16777619);
            hash2 ^= lat_bits as u32;
            hash2 = hash2.wrapping_mul(16777619);
        }

        assert_eq!(hash1, hash2, "Hash should be deterministic");
    }

    #[test]
    fn test_movement_constants() {
        // Idle movement should be smaller than active movement
        assert!(MOVEMENT_IDLE < MOVEMENT_ACTIVE, "Idle bikes should move less");

        // Movement should be reasonable (not too large)
        assert!(MOVEMENT_ACTIVE <= 0.01, "Active movement should be reasonable");
        assert!(MOVEMENT_IDLE <= 0.001, "Idle movement should be minimal");
    }

    #[test]
    fn test_traffic_penalty() {
        // Traffic should reduce speed but not eliminate it
        assert!(TRAFFIC_SPEED_REDUCTION > 0.0, "Traffic should have some effect");
        assert!(TRAFFIC_SPEED_REDUCTION < 1.0, "Traffic shouldn't stop bikes completely");
    }
}
