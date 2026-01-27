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

// ============================================================================
// Delivery Models
// ============================================================================

/// Delivery status matching TypeScript DeliveryStatus
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DeliveryStatus {
    Completed,
    Ongoing,
    Upcoming,
}

impl DeliveryStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            DeliveryStatus::Completed => "completed",
            DeliveryStatus::Ongoing => "ongoing",
            DeliveryStatus::Upcoming => "upcoming",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "completed" => Some(DeliveryStatus::Completed),
            "ongoing" => Some(DeliveryStatus::Ongoing),
            "upcoming" => Some(DeliveryStatus::Upcoming),
            _ => None,
        }
    }
}

/// Represents a delivery in the fleet system
///
/// # Why this structure?
/// - `bike_id` links to the courier (deliverer) for the force graph center node
/// - `rating` and `complaint` only populated for completed deliveries
/// - Timestamps enable time-based filtering and analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Delivery {
    pub id: String,
    pub bike_id: String,
    pub status: DeliveryStatus,
    pub customer_name: String,
    pub customer_address: String,
    pub restaurant_name: String,
    pub restaurant_address: String,
    pub rating: Option<u8>,           // 1-5, only for completed
    pub complaint: Option<String>,    // Customer complaint text
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

// ============================================================================
// Issue Models
// ============================================================================

/// Who reported the issue
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum IssueReporterType {
    Customer,
    Deliverer,
    Restaurant,
}

impl IssueReporterType {
    pub fn as_str(&self) -> &'static str {
        match self {
            IssueReporterType::Customer => "customer",
            IssueReporterType::Deliverer => "deliverer",
            IssueReporterType::Restaurant => "restaurant",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "customer" => Some(IssueReporterType::Customer),
            "deliverer" => Some(IssueReporterType::Deliverer),
            "restaurant" => Some(IssueReporterType::Restaurant),
            _ => None,
        }
    }
}

/// Issue category for classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IssueCategory {
    Late,
    Damaged,
    WrongOrder,
    Rude,
    BikeProblem,
    Other,
}

impl IssueCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            IssueCategory::Late => "late",
            IssueCategory::Damaged => "damaged",
            IssueCategory::WrongOrder => "wrong_order",
            IssueCategory::Rude => "rude",
            IssueCategory::BikeProblem => "bike_problem",
            IssueCategory::Other => "other",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "late" => Some(IssueCategory::Late),
            "damaged" => Some(IssueCategory::Damaged),
            "wrong_order" => Some(IssueCategory::WrongOrder),
            "rude" => Some(IssueCategory::Rude),
            "bike_problem" => Some(IssueCategory::BikeProblem),
            "other" => Some(IssueCategory::Other),
            _ => None,
        }
    }
}

/// Represents an issue/problem report
///
/// # Why this structure?
/// - `delivery_id` is optional: issues can be standalone (e.g., bike problems)
/// - `bike_id` always present: every issue is associated with a deliverer
/// - This dual-linking enables the force graph to show:
///   - Issues connected to specific deliveries
///   - Standalone issues connected directly to the deliverer
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub id: String,
    pub delivery_id: Option<String>,  // null = standalone issue
    pub bike_id: String,
    pub reporter_type: IssueReporterType,
    pub category: IssueCategory,
    pub description: String,
    pub resolved: bool,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Force Graph Models
// ============================================================================

/// Node types in the force-directed graph
///
/// # Why three types?
/// - Deliverer: Center of the graph (the courier/bike)
/// - Delivery: Primary connections to deliverer
/// - Issue: Secondary connections (to delivery or directly to deliverer)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ForceNodeType {
    Deliverer,
    Delivery,
    Issue,
}

/// Type-specific data payload for force graph nodes
///
/// # Why an enum?
/// - Each node type carries different data
/// - Rust enum with variants provides type safety
/// - Serializes to discriminated union in TypeScript
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ForceNodeData {
    Deliverer {
        name: String,
        status: BikeStatus,
    },
    Delivery {
        status: DeliveryStatus,
        customer: String,
        rating: Option<u8>,
    },
    Issue {
        category: IssueCategory,
        resolved: bool,
        reporter: IssueReporterType,
    },
}

/// A node in the force-directed graph
///
/// # Why Fjädra computes positions server-side?
/// - Maximum reverse-engineering protection: algorithms not in browser
/// - Positions (x, y) are the only layout data sent to client
/// - Client just renders what it receives
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForceNode {
    pub id: String,
    pub node_type: ForceNodeType,
    pub label: String,
    pub x: f64,              // Computed by Fjädra
    pub y: f64,              // Computed by Fjädra
    pub radius: f64,         // For collision detection and rendering
    pub data: ForceNodeData, // Type-specific payload
}

/// A link/edge in the force-directed graph
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForceLink {
    pub source: String,   // Node ID
    pub target: String,   // Node ID
    pub strength: f64,    // Link strength (0.0 - 1.0)
}

/// Complete force graph data returned to the client
///
/// # Why include bounds?
/// - Client can compute proper SVG viewBox
/// - No need for client to iterate all nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForceGraphData {
    pub nodes: Vec<ForceNode>,
    pub links: Vec<ForceLink>,
    pub center_x: f64,
    pub center_y: f64,
    pub bounds: (f64, f64, f64, f64), // (min_x, max_x, min_y, max_y)
}
