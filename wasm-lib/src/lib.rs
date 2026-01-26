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
}
