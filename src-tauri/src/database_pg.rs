// ============================================================================
// PostgreSQL Database Module for High Availability
// ============================================================================
//
// This module replaces SQLite with PostgreSQL for on-premise HA deployments.
//
// Key differences from SQLite version:
// - Async operations with tokio-postgres
// - Connection pooling with deadpool-postgres
// - Automatic reconnection on failure
// - Prepared statements for performance
// - Compatible with Patroni/HAProxy failover
//
// Connection string format:
// "host=10.0.0.100 port=5432 user=fleet_app password=*** dbname=bike_fleet"
//
// The host should point to HAProxy VIP for automatic failover.

use crate::models::{
    Bike, BikeStatus, DatabaseStats, Delivery, DeliveryStatus, Issue, IssueCategory,
    IssueReporterType,
};
use chrono::{DateTime, Utc};
use deadpool_postgres::{Config, ManagerConfig, Pool, RecyclingMethod, Runtime};
use std::sync::Arc;
use thiserror::Error;
use tokio_postgres::types::ToSql;
use tokio_postgres::NoTls;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("PostgreSQL error: {0}")]
    Postgres(#[from] tokio_postgres::Error),

    #[error("Pool error: {0}")]
    Pool(#[from] deadpool_postgres::PoolError),

    #[error("Database not initialized")]
    NotInitialized,

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

impl serde::Serialize for DatabaseError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Database configuration for PostgreSQL
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub dbname: String,
    pub pool_size: usize,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 5432,
            user: "fleet_app".to_string(),
            password: String::new(),
            dbname: "bike_fleet".to_string(),
            pool_size: 16,
        }
    }
}

impl DatabaseConfig {
    /// Create config from environment variables
    ///
    /// Expected env vars:
    /// - PG_HOST (default: localhost)
    /// - PG_PORT (default: 5432)
    /// - PG_USER (default: fleet_app)
    /// - PG_PASSWORD (required)
    /// - PG_DATABASE (default: bike_fleet)
    /// - PG_POOL_SIZE (default: 16)
    pub fn from_env() -> Result<Self, DatabaseError> {
        Ok(Self {
            host: std::env::var("PG_HOST").unwrap_or_else(|_| "localhost".to_string()),
            port: std::env::var("PG_PORT")
                .unwrap_or_else(|_| "5432".to_string())
                .parse()
                .unwrap_or(5432),
            user: std::env::var("PG_USER").unwrap_or_else(|_| "fleet_app".to_string()),
            password: std::env::var("PG_PASSWORD")
                .map_err(|_| DatabaseError::Config("PG_PASSWORD environment variable required".to_string()))?,
            dbname: std::env::var("PG_DATABASE").unwrap_or_else(|_| "bike_fleet".to_string()),
            pool_size: std::env::var("PG_POOL_SIZE")
                .unwrap_or_else(|_| "16".to_string())
                .parse()
                .unwrap_or(16),
        })
    }
}

/// PostgreSQL database wrapper with connection pooling
///
/// # Why connection pooling?
/// - Reuses connections (expensive to create)
/// - Handles connection failures gracefully
/// - Limits max connections to prevent overload
/// - Works transparently with HAProxy failover
pub struct Database {
    pool: Pool,
}

impl Database {
    /// Create a new database connection pool
    ///
    /// # Arguments
    /// * `config` - Database connection configuration
    ///
    /// # Returns
    /// A new Database instance with an active connection pool
    pub async fn new(config: DatabaseConfig) -> Result<Self, DatabaseError> {
        let mut cfg = Config::new();
        cfg.host = Some(config.host);
        cfg.port = Some(config.port);
        cfg.user = Some(config.user);
        cfg.password = Some(config.password);
        cfg.dbname = Some(config.dbname);
        cfg.manager = Some(ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        });

        let pool = cfg
            .builder(NoTls)?
            .max_size(config.pool_size)
            .runtime(Runtime::Tokio1)
            .build()
            .map_err(|e| DatabaseError::Config(e.to_string()))?;

        let db = Database { pool };

        // Initialize schema
        db.initialize_schema().await?;

        Ok(db)
    }

    /// Initialize the database schema
    ///
    /// # Why idempotent schema creation?
    /// - Safe to run on every startup
    /// - Uses IF NOT EXISTS for all objects
    /// - Allows rolling deployments without manual migrations
    async fn initialize_schema(&self) -> Result<(), DatabaseError> {
        let client = self.pool.get().await?;

        client
            .batch_execute(
                r#"
            -- Enable UUID extension for better primary keys
            CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

            -- Bikes table
            CREATE TABLE IF NOT EXISTS bikes (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'available',
                latitude DOUBLE PRECISION NOT NULL,
                longitude DOUBLE PRECISION NOT NULL,
                battery_level INTEGER,
                last_maintenance TIMESTAMPTZ,
                total_trips INTEGER NOT NULL DEFAULT 0,
                total_distance_km DOUBLE PRECISION NOT NULL DEFAULT 0.0,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );

            -- Trips table
            CREATE TABLE IF NOT EXISTS trips (
                id TEXT PRIMARY KEY,
                bike_id TEXT NOT NULL REFERENCES bikes(id),
                start_time TIMESTAMPTZ NOT NULL,
                end_time TIMESTAMPTZ,
                start_latitude DOUBLE PRECISION NOT NULL,
                start_longitude DOUBLE PRECISION NOT NULL,
                end_latitude DOUBLE PRECISION,
                end_longitude DOUBLE PRECISION,
                distance_km DOUBLE PRECISION
            );

            -- Deliveries table
            CREATE TABLE IF NOT EXISTS deliveries (
                id TEXT PRIMARY KEY,
                bike_id TEXT NOT NULL REFERENCES bikes(id),
                status TEXT NOT NULL DEFAULT 'upcoming',
                customer_name TEXT NOT NULL,
                customer_address TEXT NOT NULL,
                restaurant_name TEXT NOT NULL,
                restaurant_address TEXT NOT NULL,
                rating INTEGER CHECK (rating >= 1 AND rating <= 5),
                complaint TEXT,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                completed_at TIMESTAMPTZ
            );

            -- Issues table
            CREATE TABLE IF NOT EXISTS issues (
                id TEXT PRIMARY KEY,
                delivery_id TEXT REFERENCES deliveries(id),
                bike_id TEXT NOT NULL REFERENCES bikes(id),
                reporter_type TEXT NOT NULL,
                category TEXT NOT NULL,
                description TEXT NOT NULL,
                resolved BOOLEAN NOT NULL DEFAULT FALSE,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );

            -- Indexes for performance
            CREATE INDEX IF NOT EXISTS idx_bikes_status ON bikes(status);
            CREATE INDEX IF NOT EXISTS idx_trips_bike_id ON trips(bike_id);
            CREATE INDEX IF NOT EXISTS idx_deliveries_bike_id ON deliveries(bike_id);
            CREATE INDEX IF NOT EXISTS idx_deliveries_status ON deliveries(status);
            CREATE INDEX IF NOT EXISTS idx_issues_bike_id ON issues(bike_id);
            CREATE INDEX IF NOT EXISTS idx_issues_delivery_id ON issues(delivery_id);
            CREATE INDEX IF NOT EXISTS idx_issues_resolved ON issues(resolved);

            -- Function to update updated_at timestamp
            CREATE OR REPLACE FUNCTION update_updated_at_column()
            RETURNS TRIGGER AS $$
            BEGIN
                NEW.updated_at = NOW();
                RETURN NEW;
            END;
            $$ language 'plpgsql';

            -- Trigger for bikes table
            DROP TRIGGER IF EXISTS update_bikes_updated_at ON bikes;
            CREATE TRIGGER update_bikes_updated_at
                BEFORE UPDATE ON bikes
                FOR EACH ROW
                EXECUTE FUNCTION update_updated_at_column();
            "#,
            )
            .await?;

        // Seed mock data if empty
        self.seed_mock_data().await?;

        Ok(())
    }

    /// Seed the database with mock Amsterdam bike data
    async fn seed_mock_data(&self) -> Result<(), DatabaseError> {
        let client = self.pool.get().await?;

        // Check if we already have data
        let row = client
            .query_one("SELECT COUNT(*)::INTEGER as count FROM bikes", &[])
            .await?;
        let count: i32 = row.get("count");

        if count > 0 {
            return Ok(());
        }

        // Amsterdam coordinates for various locations
        let amsterdam_locations = vec![
            ("Central Station", 52.3791, 4.9003),
            ("Dam Square", 52.3731, 4.8932),
            ("Vondelpark", 52.3579, 4.8686),
            ("Rijksmuseum", 52.3600, 4.8852),
            ("Anne Frank House", 52.3752, 4.8840),
            ("Jordaan", 52.3747, 4.8797),
            ("De Pijp", 52.3533, 4.8936),
            ("Oost", 52.3614, 4.9366),
            ("Noord", 52.3907, 4.9228),
            ("Amstel", 52.3632, 4.9039),
        ];

        let statuses = ["available", "available", "available", "in_use", "charging"];

        for (i, (name, lat, lon)) in amsterdam_locations.iter().enumerate() {
            let id = format!("BIKE-{:04}", i + 1);
            let bike_name = format!("Amsterdam {} Bike", name);
            let status = statuses[i % statuses.len()];
            let battery = (20 + (i * 8) % 80) as i32;

            client
                .execute(
                    r#"INSERT INTO bikes (id, name, status, latitude, longitude, battery_level, total_trips, total_distance_km)
                       VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
                    &[
                        &id,
                        &bike_name,
                        &status,
                        lat,
                        lon,
                        &battery,
                        &((i * 17) % 200) as &(dyn ToSql + Sync),
                        &((i as f64 * 12.5) % 500.0),
                    ],
                )
                .await?;
        }

        // Seed deliveries and issues
        self.seed_deliveries_and_issues().await?;

        Ok(())
    }

    /// Seed deliveries and issues for demonstration
    async fn seed_deliveries_and_issues(&self) -> Result<(), DatabaseError> {
        let client = self.pool.get().await?;
        let now = Utc::now();

        let customer_names = [
            "P. de Vries", "M. Jansen", "A. Bakker", "J. van Dijk", "S. Visser",
            "L. Smit", "K. Mulder", "R. de Boer", "T. Bos", "E. van den Berg",
            "H. Dekker", "F. Vermeer", "B. van Leeuwen", "N. Kok", "D. Peters",
        ];

        let restaurant_names = [
            "De Pizzabakker", "Wok to Walk", "Febo", "New York Pizza", "Dominos",
            "Thai Express", "Sushi Time", "Burger King", "McDonalds", "Subway",
            "La Place", "Vapiano", "Bagels & Beans", "De Italiaan", "Ramen Ya",
        ];

        let streets = [
            "Damrak", "Rokin", "Kalverstraat", "Leidsestraat", "Utrechtsestraat",
            "Overtoom", "Kinkerstraat", "Ferdinand Bolstraat", "Javastraat", "Plantage",
        ];

        // Create 50 deliveries
        for i in 0..50 {
            let bike_id = format!("BIKE-{:04}", (i % 10) + 1);
            let delivery_id = format!("DEL-{:04}", i + 1);

            let status = match i % 10 {
                0..=5 => "completed",
                6..=7 => "ongoing",
                _ => "upcoming",
            };

            let rating: Option<i32> = if status == "completed" && i % 3 == 0 {
                Some(((i % 5) + 1) as i32)
            } else {
                None
            };

            let complaint: Option<&str> = if status == "completed" && i % 7 == 0 {
                Some("Order arrived cold")
            } else {
                None
            };

            let days_ago = (50 - i) as i64 / 7;
            let created_at = now - chrono::Duration::days(days_ago);
            let completed_at: Option<DateTime<Utc>> = if status == "completed" {
                Some(created_at + chrono::Duration::hours(1))
            } else {
                None
            };

            client
                .execute(
                    r#"INSERT INTO deliveries (id, bike_id, status, customer_name, customer_address,
                       restaurant_name, restaurant_address, rating, complaint, created_at, completed_at)
                       VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"#,
                    &[
                        &delivery_id,
                        &bike_id,
                        &status,
                        &customer_names[i % customer_names.len()],
                        &format!("{} {}", streets[i % streets.len()], (i % 200) + 1),
                        &restaurant_names[i % restaurant_names.len()],
                        &format!("{} {}", streets[(i + 3) % streets.len()], (i % 150) + 1),
                        &rating,
                        &complaint,
                        &created_at,
                        &completed_at,
                    ],
                )
                .await?;
        }

        let issue_descriptions: [(&str, &str); 6] = [
            ("late", "Delivery arrived 30 minutes late"),
            ("damaged", "Food container was crushed"),
            ("wrong_order", "Received someone else's order"),
            ("rude", "Deliverer was impolite"),
            ("bike_problem", "Flat tire during delivery"),
            ("other", "General complaint about service"),
        ];

        let reporter_types = ["customer", "deliverer", "restaurant"];

        // Create 20 issues
        for i in 0..20 {
            let issue_id = format!("ISS-{:04}", i + 1);
            let bike_id = format!("BIKE-{:04}", (i % 10) + 1);

            let delivery_id: Option<String> = if i % 3 != 0 {
                Some(format!("DEL-{:04}", (i % 50) + 1))
            } else {
                None
            };

            let (category, description) = issue_descriptions[i % issue_descriptions.len()];
            let reporter_type = reporter_types[i % reporter_types.len()];
            let resolved = i % 3 == 0;

            let days_ago = (i as i64) % 14;
            let created_at = now - chrono::Duration::days(days_ago);

            client
                .execute(
                    r#"INSERT INTO issues (id, delivery_id, bike_id, reporter_type, category,
                       description, resolved, created_at)
                       VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
                    &[
                        &issue_id,
                        &delivery_id,
                        &bike_id,
                        &reporter_type,
                        &category,
                        &description,
                        &resolved,
                        &created_at,
                    ],
                )
                .await?;
        }

        Ok(())
    }

    // ========================================================================
    // Bike Queries
    // ========================================================================

    /// Get all bikes from the database
    pub async fn get_all_bikes(&self) -> Result<Vec<Bike>, DatabaseError> {
        let client = self.pool.get().await?;

        let rows = client
            .query(
                r#"SELECT id, name, status, latitude, longitude, battery_level,
                          last_maintenance, total_trips, total_distance_km, created_at, updated_at
                   FROM bikes ORDER BY name"#,
                &[],
            )
            .await?;

        let bikes = rows.iter().map(|row| self.map_bike_row(row)).collect();
        Ok(bikes)
    }

    /// Get a bike by ID
    pub async fn get_bike_by_id(&self, bike_id: &str) -> Result<Option<Bike>, DatabaseError> {
        let client = self.pool.get().await?;

        let row = client
            .query_opt(
                r#"SELECT id, name, status, latitude, longitude, battery_level,
                          last_maintenance, total_trips, total_distance_km, created_at, updated_at
                   FROM bikes WHERE id = $1"#,
                &[&bike_id],
            )
            .await?;

        Ok(row.map(|r| self.map_bike_row(&r)))
    }

    /// Add a new bike to the fleet
    pub async fn add_bike(
        &self,
        name: &str,
        lat: f64,
        lon: f64,
        battery: Option<u8>,
    ) -> Result<Bike, DatabaseError> {
        let client = self.pool.get().await?;
        let id = format!("BIKE-{}", uuid_v4_simple());
        let now = Utc::now();

        client
            .execute(
                r#"INSERT INTO bikes (id, name, status, latitude, longitude, battery_level,
                   total_trips, total_distance_km, created_at, updated_at)
                   VALUES ($1, $2, 'available', $3, $4, $5, 0, 0.0, $6, $7)"#,
                &[
                    &id,
                    &name,
                    &lat,
                    &lon,
                    &battery.map(|b| b as i32),
                    &now,
                    &now,
                ],
            )
            .await?;

        Ok(Bike {
            id,
            name: name.to_string(),
            status: BikeStatus::Available,
            latitude: lat,
            longitude: lon,
            battery_level: battery,
            last_maintenance: None,
            total_trips: 0,
            total_distance_km: 0.0,
            created_at: now,
            updated_at: now,
        })
    }

    /// Update bike status
    pub async fn update_bike_status(
        &self,
        bike_id: &str,
        status: &BikeStatus,
        lat: Option<f64>,
        lon: Option<f64>,
        battery: Option<u8>,
    ) -> Result<(), DatabaseError> {
        let client = self.pool.get().await?;

        // PostgreSQL handles the updated_at via trigger
        match (lat, lon, battery) {
            (Some(lat_val), Some(lon_val), Some(bat_val)) => {
                client
                    .execute(
                        "UPDATE bikes SET status = $1, latitude = $2, longitude = $3, battery_level = $4 WHERE id = $5",
                        &[&status.as_str(), &lat_val, &lon_val, &(bat_val as i32), &bike_id],
                    )
                    .await?;
            }
            (Some(lat_val), Some(lon_val), None) => {
                client
                    .execute(
                        "UPDATE bikes SET status = $1, latitude = $2, longitude = $3 WHERE id = $4",
                        &[&status.as_str(), &lat_val, &lon_val, &bike_id],
                    )
                    .await?;
            }
            (None, None, Some(bat_val)) => {
                client
                    .execute(
                        "UPDATE bikes SET status = $1, battery_level = $2 WHERE id = $3",
                        &[&status.as_str(), &(bat_val as i32), &bike_id],
                    )
                    .await?;
            }
            _ => {
                client
                    .execute(
                        "UPDATE bikes SET status = $1 WHERE id = $2",
                        &[&status.as_str(), &bike_id],
                    )
                    .await?;
            }
        }

        Ok(())
    }

    fn map_bike_row(&self, row: &tokio_postgres::Row) -> Bike {
        let status_str: String = row.get("status");
        let status = BikeStatus::from_str(&status_str).unwrap_or(BikeStatus::Offline);
        let battery_level: Option<i32> = row.get("battery_level");

        Bike {
            id: row.get("id"),
            name: row.get("name"),
            status,
            latitude: row.get("latitude"),
            longitude: row.get("longitude"),
            battery_level: battery_level.map(|v| v as u8),
            last_maintenance: row.get("last_maintenance"),
            total_trips: row.get::<_, i32>("total_trips") as u32,
            total_distance_km: row.get("total_distance_km"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }

    // ========================================================================
    // Delivery Queries
    // ========================================================================

    /// Get all deliveries, optionally filtered by bike_id and/or status
    pub async fn get_deliveries(
        &self,
        bike_id: Option<&str>,
        status: Option<&str>,
    ) -> Result<Vec<Delivery>, DatabaseError> {
        let client = self.pool.get().await?;

        // Build dynamic query
        let mut sql = String::from(
            r#"SELECT id, bike_id, status, customer_name, customer_address,
                      restaurant_name, restaurant_address, rating, complaint,
                      created_at, completed_at
               FROM deliveries WHERE true"#,
        );

        let mut params: Vec<&(dyn ToSql + Sync)> = Vec::new();
        let mut param_idx = 1;

        // Temporary variables to extend lifetime
        let bike_id_str: String;
        let status_str: String;

        if let Some(b) = bike_id {
            sql.push_str(&format!(" AND bike_id = ${}", param_idx));
            bike_id_str = b.to_string();
            params.push(&bike_id_str);
            param_idx += 1;
        }
        if let Some(s) = status {
            sql.push_str(&format!(" AND status = ${}", param_idx));
            status_str = s.to_string();
            params.push(&status_str);
        }
        sql.push_str(" ORDER BY created_at DESC");

        let rows = client.query(&sql, &params).await?;

        let deliveries = rows.iter().map(|row| self.map_delivery_row(row)).collect();
        Ok(deliveries)
    }

    /// Get a single delivery by ID
    pub async fn get_delivery_by_id(
        &self,
        delivery_id: &str,
    ) -> Result<Option<Delivery>, DatabaseError> {
        let client = self.pool.get().await?;

        let row = client
            .query_opt(
                r#"SELECT id, bike_id, status, customer_name, customer_address,
                          restaurant_name, restaurant_address, rating, complaint,
                          created_at, completed_at
                   FROM deliveries WHERE id = $1"#,
                &[&delivery_id],
            )
            .await?;

        Ok(row.map(|r| self.map_delivery_row(&r)))
    }

    /// Get deliveries for a specific bike (for force graph)
    pub async fn get_deliveries_by_bike(&self, bike_id: &str) -> Result<Vec<Delivery>, DatabaseError> {
        self.get_deliveries(Some(bike_id), None).await
    }

    fn map_delivery_row(&self, row: &tokio_postgres::Row) -> Delivery {
        let status_str: String = row.get("status");
        let status = DeliveryStatus::from_str(&status_str).unwrap_or(DeliveryStatus::Upcoming);
        let rating: Option<i32> = row.get("rating");

        Delivery {
            id: row.get("id"),
            bike_id: row.get("bike_id"),
            status,
            customer_name: row.get("customer_name"),
            customer_address: row.get("customer_address"),
            restaurant_name: row.get("restaurant_name"),
            restaurant_address: row.get("restaurant_address"),
            rating: rating.map(|r| r as u8),
            complaint: row.get("complaint"),
            created_at: row.get("created_at"),
            completed_at: row.get("completed_at"),
        }
    }

    // ========================================================================
    // Issue Queries
    // ========================================================================

    /// Get all issues, optionally filtered
    pub async fn get_issues(
        &self,
        bike_id: Option<&str>,
        resolved: Option<bool>,
        category: Option<&str>,
    ) -> Result<Vec<Issue>, DatabaseError> {
        let client = self.pool.get().await?;

        let mut sql = String::from(
            r#"SELECT id, delivery_id, bike_id, reporter_type, category,
                      description, resolved, created_at
               FROM issues WHERE true"#,
        );

        let mut params: Vec<Box<dyn ToSql + Sync + Send>> = Vec::new();
        let mut param_idx = 1;

        if let Some(b) = bike_id {
            sql.push_str(&format!(" AND bike_id = ${}", param_idx));
            params.push(Box::new(b.to_string()));
            param_idx += 1;
        }
        if let Some(r) = resolved {
            sql.push_str(&format!(" AND resolved = ${}", param_idx));
            params.push(Box::new(r));
            param_idx += 1;
        }
        if let Some(c) = category {
            sql.push_str(&format!(" AND category = ${}", param_idx));
            params.push(Box::new(c.to_string()));
        }
        sql.push_str(" ORDER BY created_at DESC");

        let param_refs: Vec<&(dyn ToSql + Sync)> =
            params.iter().map(|p| p.as_ref() as &(dyn ToSql + Sync)).collect();
        let rows = client.query(&sql, &param_refs).await?;

        let issues = rows.iter().map(|row| self.map_issue_row(row)).collect();
        Ok(issues)
    }

    /// Get a single issue by ID
    pub async fn get_issue_by_id(&self, issue_id: &str) -> Result<Option<Issue>, DatabaseError> {
        let client = self.pool.get().await?;

        let row = client
            .query_opt(
                r#"SELECT id, delivery_id, bike_id, reporter_type, category,
                          description, resolved, created_at
                   FROM issues WHERE id = $1"#,
                &[&issue_id],
            )
            .await?;

        Ok(row.map(|r| self.map_issue_row(&r)))
    }

    /// Get issues for a specific bike (for force graph)
    pub async fn get_issues_by_bike(&self, bike_id: &str) -> Result<Vec<Issue>, DatabaseError> {
        self.get_issues(Some(bike_id), None, None).await
    }

    fn map_issue_row(&self, row: &tokio_postgres::Row) -> Issue {
        let reporter_str: String = row.get("reporter_type");
        let category_str: String = row.get("category");

        Issue {
            id: row.get("id"),
            delivery_id: row.get("delivery_id"),
            bike_id: row.get("bike_id"),
            reporter_type: IssueReporterType::from_str(&reporter_str)
                .unwrap_or(IssueReporterType::Customer),
            category: IssueCategory::from_str(&category_str).unwrap_or(IssueCategory::Other),
            description: row.get("description"),
            resolved: row.get("resolved"),
            created_at: row.get("created_at"),
        }
    }

    // ========================================================================
    // Statistics
    // ========================================================================

    /// Get database statistics
    pub async fn get_stats(&self) -> Result<DatabaseStats, DatabaseError> {
        let client = self.pool.get().await?;

        let total_bikes: i64 = client
            .query_one("SELECT COUNT(*) FROM bikes", &[])
            .await?
            .get(0);

        let total_trips: i64 = client
            .query_one("SELECT COALESCE(SUM(total_trips), 0) FROM bikes", &[])
            .await?
            .get(0);

        // Get database size (PostgreSQL specific)
        let db_size: i64 = client
            .query_one(
                "SELECT pg_database_size(current_database())",
                &[],
            )
            .await?
            .get(0);

        Ok(DatabaseStats {
            total_bikes: total_bikes as u32,
            total_trips: total_trips as u32,
            database_size_bytes: db_size as u64,
            last_sync: Some(Utc::now()),
        })
    }

    // ========================================================================
    // Health Check
    // ========================================================================

    /// Check database connectivity and replication status
    ///
    /// # Returns
    /// - Ok(true) if connected to primary (read-write)
    /// - Ok(false) if connected to replica (read-only)
    /// - Err if connection failed
    pub async fn health_check(&self) -> Result<bool, DatabaseError> {
        let client = self.pool.get().await?;

        // Check if we're on primary or replica
        let row = client
            .query_one("SELECT pg_is_in_recovery()", &[])
            .await?;
        let is_replica: bool = row.get(0);

        Ok(!is_replica) // Returns true if primary (not in recovery)
    }

    /// Get replication lag (useful for monitoring)
    ///
    /// # Returns
    /// Replication lag in bytes, or None if not applicable
    pub async fn get_replication_lag(&self) -> Result<Option<i64>, DatabaseError> {
        let client = self.pool.get().await?;

        let row = client
            .query_opt(
                r#"SELECT pg_wal_lsn_diff(pg_current_wal_lsn(), replay_lsn)::bigint as lag
                   FROM pg_stat_replication
                   LIMIT 1"#,
                &[],
            )
            .await?;

        Ok(row.map(|r| r.get("lag")))
    }
}

/// Generate a simple UUID-like string
fn uuid_v4_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", now)
}

// ============================================================================
// Thread-safe wrapper for Tauri state management
// ============================================================================

/// Thread-safe database wrapper for use with Tauri's state management
///
/// # Why Arc?
/// - Tauri state needs to be Send + Sync
/// - Arc allows multiple threads to share the database pool
/// - Pool handles concurrent connections internally
pub type SharedDatabase = Arc<Database>;

/// Create a shared database instance for Tauri
pub async fn create_shared_database(config: DatabaseConfig) -> Result<SharedDatabase, DatabaseError> {
    let db = Database::new(config).await?;
    Ok(Arc::new(db))
}
