// SQLite commands (default)
#[cfg(feature = "sqlite")]
pub mod database;
#[cfg(feature = "sqlite")]
pub mod deliveries;
#[cfg(feature = "sqlite")]
pub mod fleet;
#[cfg(feature = "sqlite")]
pub mod force_graph;
#[cfg(feature = "sqlite")]
pub mod issues;

// PostgreSQL commands (for HA deployments)
#[cfg(feature = "postgres")]
pub mod database_pg;
#[cfg(feature = "postgres")]
pub mod deliveries_pg;
#[cfg(feature = "postgres")]
pub mod fleet_pg;
#[cfg(feature = "postgres")]
pub mod force_graph_pg;
#[cfg(feature = "postgres")]
pub mod issues_pg;

// Shared modules (both backends)
pub mod health;
pub mod license;
pub mod secure;
