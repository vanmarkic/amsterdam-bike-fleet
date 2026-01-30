//! PostgreSQL Fleet Commands for Tauri
//!
//! Async versions of fleet commands for PostgreSQL backend.

use crate::models::{AddBikeRequest, Bike, BikeStatus, FleetStats, UpdateBikeStatusRequest};
use crate::AppState;
use tauri::State;

/// Get all fleet data including bikes and statistics
#[tauri::command]
pub async fn get_fleet_data(state: State<'_, AppState>) -> Result<Vec<Bike>, String> {
    let db_guard = state.db.lock().map_err(|e| e.to_string())?;

    match db_guard.as_ref() {
        Some(db) => db.get_all_bikes().await.map_err(|e| e.to_string()),
        None => {
            // Return mock data if database is not initialized
            Ok(generate_mock_fleet())
        }
    }
}

/// Get a specific bike by ID
#[tauri::command]
pub async fn get_bike_by_id(
    bike_id: String,
    state: State<'_, AppState>,
) -> Result<Option<Bike>, String> {
    let db_guard = state.db.lock().map_err(|e| e.to_string())?;

    match db_guard.as_ref() {
        Some(db) => db.get_bike_by_id(&bike_id).await.map_err(|e| e.to_string()),
        None => {
            // Search in mock data
            let mock_fleet = generate_mock_fleet();
            Ok(mock_fleet.into_iter().find(|b| b.id == bike_id))
        }
    }
}

/// Add a new bike to the fleet
#[tauri::command]
pub async fn add_bike(
    request: AddBikeRequest,
    state: State<'_, AppState>,
) -> Result<Bike, String> {
    let db_guard = state.db.lock().map_err(|e| e.to_string())?;

    match db_guard.as_ref() {
        Some(db) => db
            .add_bike(
                &request.name,
                request.latitude,
                request.longitude,
                request.battery_level,
            )
            .await
            .map_err(|e| e.to_string()),
        None => Err("Database not initialized. Call init_database first.".to_string()),
    }
}

/// Update bike status
#[tauri::command]
pub async fn update_bike_status(
    request: UpdateBikeStatusRequest,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db_guard = state.db.lock().map_err(|e| e.to_string())?;

    match db_guard.as_ref() {
        Some(db) => db
            .update_bike_status(
                &request.bike_id,
                &request.status,
                request.latitude,
                request.longitude,
                request.battery_level,
            )
            .await
            .map_err(|e| e.to_string()),
        None => Err("Database not initialized. Call init_database first.".to_string()),
    }
}

/// Generate mock fleet data for when database is not available
fn generate_mock_fleet() -> Vec<Bike> {
    use chrono::Utc;

    let amsterdam_locations = vec![
        ("BIKE-0001", "Central Station Bike", 52.3791, 4.9003, BikeStatus::Available, 85),
        ("BIKE-0002", "Dam Square Bike", 52.3731, 4.8932, BikeStatus::InUse, 62),
        ("BIKE-0003", "Vondelpark Bike", 52.3579, 4.8686, BikeStatus::Available, 91),
        ("BIKE-0004", "Rijksmuseum Bike", 52.3600, 4.8852, BikeStatus::Charging, 34),
        ("BIKE-0005", "Anne Frank House Bike", 52.3752, 4.8840, BikeStatus::Available, 78),
        ("BIKE-0006", "Jordaan Bike", 52.3747, 4.8797, BikeStatus::InUse, 55),
        ("BIKE-0007", "De Pijp Bike", 52.3533, 4.8936, BikeStatus::Maintenance, 42),
        ("BIKE-0008", "Oost Bike", 52.3614, 4.9366, BikeStatus::Available, 89),
        ("BIKE-0009", "Noord Bike", 52.3907, 4.9228, BikeStatus::Available, 67),
        ("BIKE-0010", "Amstel Bike", 52.3632, 4.9039, BikeStatus::Charging, 23),
    ];

    let now = Utc::now();

    amsterdam_locations
        .into_iter()
        .enumerate()
        .map(|(i, (id, name, lat, lon, status, battery))| Bike {
            id: id.to_string(),
            name: name.to_string(),
            status,
            latitude: lat,
            longitude: lon,
            battery_level: Some(battery),
            last_maintenance: None,
            total_trips: (i as u32 * 17) % 200,
            total_distance_km: (i as f64 * 12.5) % 500.0,
            created_at: now,
            updated_at: now,
        })
        .collect()
}

/// Get fleet statistics
#[tauri::command]
pub async fn get_fleet_stats(state: State<'_, AppState>) -> Result<FleetStats, String> {
    let bikes = get_fleet_data(state).await?;

    let total = bikes.len() as u32;
    let available = bikes.iter().filter(|b| b.status == BikeStatus::Available).count() as u32;
    let in_use = bikes.iter().filter(|b| b.status == BikeStatus::InUse).count() as u32;
    let maintenance = bikes.iter().filter(|b| b.status == BikeStatus::Maintenance).count() as u32;
    let charging = bikes.iter().filter(|b| b.status == BikeStatus::Charging).count() as u32;
    let offline = bikes.iter().filter(|b| b.status == BikeStatus::Offline).count() as u32;

    let avg_battery: f64 = bikes
        .iter()
        .filter_map(|b| b.battery_level)
        .map(|b| b as f64)
        .sum::<f64>()
        / bikes.iter().filter(|b| b.battery_level.is_some()).count().max(1) as f64;

    Ok(FleetStats {
        total_bikes: total,
        available_bikes: available,
        bikes_in_use: in_use,
        bikes_in_maintenance: maintenance,
        bikes_charging: charging,
        bikes_offline: offline,
        average_battery: avg_battery,
        total_trips_today: 42, // Mock value
    })
}
