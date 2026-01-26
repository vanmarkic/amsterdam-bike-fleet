use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a bike in the Amsterdam fleet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bike {
    pub id: String,
    pub name: String,
    pub status: BikeStatus,
    pub latitude: f64,
    pub longitude: f64,
    pub battery_level: Option<u8>,
    pub last_maintenance: Option<DateTime<Utc>>,
    pub total_trips: u32,
    pub total_distance_km: f64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Bike availability status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BikeStatus {
    Available,
    InUse,
    Maintenance,
    Charging,
    Offline,
}

impl BikeStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            BikeStatus::Available => "available",
            BikeStatus::InUse => "in_use",
            BikeStatus::Maintenance => "maintenance",
            BikeStatus::Charging => "charging",
            BikeStatus::Offline => "offline",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "available" => Some(BikeStatus::Available),
            "in_use" => Some(BikeStatus::InUse),
            "maintenance" => Some(BikeStatus::Maintenance),
            "charging" => Some(BikeStatus::Charging),
            "offline" => Some(BikeStatus::Offline),
            _ => None,
        }
    }
}

/// Fleet statistics summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetStats {
    pub total_bikes: u32,
    pub available_bikes: u32,
    pub bikes_in_use: u32,
    pub bikes_in_maintenance: u32,
    pub bikes_charging: u32,
    pub bikes_offline: u32,
    pub average_battery: f64,
    pub total_trips_today: u32,
}

/// Database statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub total_bikes: u32,
    pub total_trips: u32,
    pub database_size_bytes: u64,
    pub last_sync: Option<DateTime<Utc>>,
}

/// Request to add a new bike
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddBikeRequest {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub battery_level: Option<u8>,
}

/// Request to update bike status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateBikeStatusRequest {
    pub bike_id: String,
    pub status: BikeStatus,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub battery_level: Option<u8>,
}
