use crate::models::{
    Bike, BikeStatus, DatabaseStats,
    Delivery, DeliveryStatus,
    Issue, IssueCategory, IssueReporterType,
};
use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, Result as SqliteResult};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Database not initialized")]
    NotInitialized,
    #[error("Invalid data: {0}")]
    InvalidData(String),
}

impl serde::Serialize for DatabaseError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Database wrapper for SQLite operations
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Initialize a new database connection
    pub fn new(path: PathBuf) -> Result<Self, DatabaseError> {
        let conn = Connection::open(&path)?;
        let db = Database { conn };
        db.initialize_schema()?;
        db.seed_mock_data()?;
        Ok(db)
    }

    /// Initialize the database schema
    fn initialize_schema(&self) -> Result<(), DatabaseError> {
        self.conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS bikes (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'available',
                latitude REAL NOT NULL,
                longitude REAL NOT NULL,
                battery_level INTEGER,
                last_maintenance TEXT,
                total_trips INTEGER NOT NULL DEFAULT 0,
                total_distance_km REAL NOT NULL DEFAULT 0.0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS trips (
                id TEXT PRIMARY KEY,
                bike_id TEXT NOT NULL,
                start_time TEXT NOT NULL,
                end_time TEXT,
                start_latitude REAL NOT NULL,
                start_longitude REAL NOT NULL,
                end_latitude REAL,
                end_longitude REAL,
                distance_km REAL,
                FOREIGN KEY (bike_id) REFERENCES bikes(id)
            );

            CREATE INDEX IF NOT EXISTS idx_bikes_status ON bikes(status);
            CREATE INDEX IF NOT EXISTS idx_trips_bike_id ON trips(bike_id);

            -- ================================================================
            -- Deliveries table
            -- ================================================================
            -- Why this schema?
            -- - bike_id links to the courier for force graph relationships
            -- - status enables filtering (completed/ongoing/upcoming)
            -- - rating/complaint only populated for completed deliveries
            -- - Timestamps enable time-series analytics
            CREATE TABLE IF NOT EXISTS deliveries (
                id TEXT PRIMARY KEY,
                bike_id TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'upcoming',
                customer_name TEXT NOT NULL,
                customer_address TEXT NOT NULL,
                restaurant_name TEXT NOT NULL,
                restaurant_address TEXT NOT NULL,
                rating INTEGER,
                complaint TEXT,
                created_at TEXT NOT NULL,
                completed_at TEXT,
                FOREIGN KEY (bike_id) REFERENCES bikes(id)
            );

            -- ================================================================
            -- Issues table
            -- ================================================================
            -- Why this schema?
            -- - delivery_id is optional: issues can be standalone (bike problems)
            -- - bike_id always present: every issue links to a deliverer
            -- - This dual-linking enables force graph to show:
            --   * Issues connected to specific deliveries
            --   * Standalone issues connected directly to deliverer
            CREATE TABLE IF NOT EXISTS issues (
                id TEXT PRIMARY KEY,
                delivery_id TEXT,
                bike_id TEXT NOT NULL,
                reporter_type TEXT NOT NULL,
                category TEXT NOT NULL,
                description TEXT NOT NULL,
                resolved INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                FOREIGN KEY (delivery_id) REFERENCES deliveries(id),
                FOREIGN KEY (bike_id) REFERENCES bikes(id)
            );

            -- Indexes for efficient querying
            CREATE INDEX IF NOT EXISTS idx_deliveries_bike_id ON deliveries(bike_id);
            CREATE INDEX IF NOT EXISTS idx_deliveries_status ON deliveries(status);
            CREATE INDEX IF NOT EXISTS idx_issues_bike_id ON issues(bike_id);
            CREATE INDEX IF NOT EXISTS idx_issues_delivery_id ON issues(delivery_id);
            CREATE INDEX IF NOT EXISTS idx_issues_resolved ON issues(resolved);
            "#,
        )?;
        Ok(())
    }

    /// Seed the database with mock Amsterdam bike data
    ///
    /// # Why seed data?
    /// - Enables immediate demo/testing without external data source
    /// - Provides realistic Dutch names and Amsterdam addresses
    /// - Creates interconnected deliveries and issues for force graph demo
    fn seed_mock_data(&self) -> Result<(), DatabaseError> {
        // Check if we already have data
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM bikes", [], |row| row.get(0))?;

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

        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let statuses = ["available", "available", "available", "in_use", "charging"];

        for (i, (name, lat, lon)) in amsterdam_locations.iter().enumerate() {
            let id = format!("BIKE-{:04}", i + 1);
            let bike_name = format!("Amsterdam {} Bike", name);
            let status = statuses[i % statuses.len()];
            let battery = 20 + (i * 8) % 80;

            self.conn.execute(
                r#"INSERT INTO bikes (id, name, status, latitude, longitude, battery_level,
                   total_trips, total_distance_km, created_at, updated_at)
                   VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"#,
                rusqlite::params![
                    id,
                    bike_name,
                    status,
                    lat,
                    lon,
                    battery as i32,
                    (i * 17) % 200,
                    (i as f64 * 12.5) % 500.0,
                    now_str,
                    now_str
                ],
            )?;
        }

        // Seed deliveries and issues
        self.seed_deliveries_and_issues()?;

        Ok(())
    }

    /// Seed deliveries and issues for demonstration
    ///
    /// # Why separate method?
    /// - Keeps seed_mock_data focused on bikes
    /// - Deliveries/issues are dependent on bikes existing first
    fn seed_deliveries_and_issues(&self) -> Result<(), DatabaseError> {
        let now = Utc::now();

        // Dutch customer names
        let customer_names = [
            "P. de Vries", "M. Jansen", "A. Bakker", "J. van Dijk", "S. Visser",
            "L. Smit", "K. Mulder", "R. de Boer", "T. Bos", "E. van den Berg",
            "H. Dekker", "F. Vermeer", "B. van Leeuwen", "N. Kok", "D. Peters",
        ];

        // Restaurant names
        let restaurant_names = [
            "De Pizzabakker", "Wok to Walk", "Febo", "New York Pizza", "Dominos",
            "Thai Express", "Sushi Time", "Burger King", "McDonalds", "Subway",
            "La Place", "Vapiano", "Bagels & Beans", "De Italiaan", "Ramen Ya",
        ];

        // Amsterdam streets
        let streets = [
            "Damrak", "Rokin", "Kalverstraat", "Leidsestraat", "Utrechtsestraat",
            "Overtoom", "Kinkerstraat", "Ferdinand Bolstraat", "Javastraat", "Plantage",
        ];

        // Create 50 deliveries across 10 bikes
        for i in 0..50 {
            let bike_id = format!("BIKE-{:04}", (i % 10) + 1);
            let delivery_id = format!("DEL-{:04}", i + 1);

            // Deterministic but varied status distribution
            let status = match i % 10 {
                0..=5 => "completed",
                6..=7 => "ongoing",
                _ => "upcoming",
            };

            // Only completed deliveries have ratings/complaints
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

            // Timestamps: older deliveries completed, newer ones ongoing/upcoming
            let days_ago = (50 - i) as i64 / 7;
            let created_at = now - chrono::Duration::days(days_ago);
            let completed_at = if status == "completed" {
                Some((created_at + chrono::Duration::hours(1)).to_rfc3339())
            } else {
                None
            };

            self.conn.execute(
                r#"INSERT INTO deliveries (
                    id, bike_id, status, customer_name, customer_address,
                    restaurant_name, restaurant_address, rating, complaint,
                    created_at, completed_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"#,
                rusqlite::params![
                    delivery_id,
                    bike_id,
                    status,
                    customer_names[i % customer_names.len()],
                    format!("{} {}", streets[i % streets.len()], (i % 200) + 1),
                    restaurant_names[i % restaurant_names.len()],
                    format!("{} {}", streets[(i + 3) % streets.len()], (i % 150) + 1),
                    rating,
                    complaint,
                    created_at.to_rfc3339(),
                    completed_at
                ],
            )?;
        }

        // Issue descriptions by category
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

            // 70% of issues linked to a delivery, 30% standalone
            let delivery_id: Option<String> = if i % 3 != 0 {
                Some(format!("DEL-{:04}", (i % 50) + 1))
            } else {
                None
            };

            let (category, description) = issue_descriptions[i % issue_descriptions.len()];
            let reporter_type = reporter_types[i % reporter_types.len()];
            let resolved = i % 3 == 0; // 33% resolved

            let days_ago = (i as i64) % 14;
            let created_at = now - chrono::Duration::days(days_ago);

            self.conn.execute(
                r#"INSERT INTO issues (
                    id, delivery_id, bike_id, reporter_type, category,
                    description, resolved, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"#,
                rusqlite::params![
                    issue_id,
                    delivery_id,
                    bike_id,
                    reporter_type,
                    category,
                    description,
                    resolved as i32,
                    created_at.to_rfc3339()
                ],
            )?;
        }

        Ok(())
    }

    /// Get all bikes from the database
    pub fn get_all_bikes(&self) -> Result<Vec<Bike>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r#"SELECT id, name, status, latitude, longitude, battery_level,
                      last_maintenance, total_trips, total_distance_km, created_at, updated_at
               FROM bikes ORDER BY name"#,
        )?;

        let bikes = stmt
            .query_map([], |row| {
                let status_str: String = row.get(2)?;
                let status =
                    BikeStatus::from_str(&status_str).unwrap_or(BikeStatus::Offline);

                Ok(Bike {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    status,
                    latitude: row.get(3)?,
                    longitude: row.get(4)?,
                    battery_level: row.get::<_, Option<i32>>(5)?.map(|v| v as u8),
                    last_maintenance: row
                        .get::<_, Option<String>>(6)?
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc)),
                    total_trips: row.get::<_, i32>(7)? as u32,
                    total_distance_km: row.get(8)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })?
            .collect::<SqliteResult<Vec<_>>>()?;

        Ok(bikes)
    }

    /// Get a bike by ID
    pub fn get_bike_by_id(&self, bike_id: &str) -> Result<Option<Bike>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r#"SELECT id, name, status, latitude, longitude, battery_level,
                      last_maintenance, total_trips, total_distance_km, created_at, updated_at
               FROM bikes WHERE id = ?1"#,
        )?;

        let bike = stmt
            .query_row([bike_id], |row| {
                let status_str: String = row.get(2)?;
                let status =
                    BikeStatus::from_str(&status_str).unwrap_or(BikeStatus::Offline);

                Ok(Bike {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    status,
                    latitude: row.get(3)?,
                    longitude: row.get(4)?,
                    battery_level: row.get::<_, Option<i32>>(5)?.map(|v| v as u8),
                    last_maintenance: row
                        .get::<_, Option<String>>(6)?
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc)),
                    total_trips: row.get::<_, i32>(7)? as u32,
                    total_distance_km: row.get(8)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?)
                        .map(|dt| dt.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                })
            })
            .optional()?;

        Ok(bike)
    }

    /// Add a new bike to the fleet
    pub fn add_bike(&self, name: &str, lat: f64, lon: f64, battery: Option<u8>) -> Result<Bike, DatabaseError> {
        let id = format!("BIKE-{}", uuid_v4_simple());
        let now = Utc::now();
        let now_str = now.to_rfc3339();

        self.conn.execute(
            r#"INSERT INTO bikes (id, name, status, latitude, longitude, battery_level,
               total_trips, total_distance_km, created_at, updated_at)
               VALUES (?1, ?2, 'available', ?3, ?4, ?5, 0, 0.0, ?6, ?7)"#,
            rusqlite::params![id, name, lat, lon, battery.map(|b| b as i32), now_str, now_str],
        )?;

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
    pub fn update_bike_status(
        &self,
        bike_id: &str,
        status: &BikeStatus,
        lat: Option<f64>,
        lon: Option<f64>,
        battery: Option<u8>,
    ) -> Result<(), DatabaseError> {
        let now = Utc::now().to_rfc3339();

        // Build update based on provided values
        match (lat, lon, battery) {
            (Some(lat_val), Some(lon_val), Some(bat_val)) => {
                self.conn.execute(
                    "UPDATE bikes SET status = ?1, updated_at = ?2, latitude = ?3, longitude = ?4, battery_level = ?5 WHERE id = ?6",
                    rusqlite::params![status.as_str(), now, lat_val, lon_val, bat_val as i32, bike_id],
                )?;
            }
            (Some(lat_val), Some(lon_val), None) => {
                self.conn.execute(
                    "UPDATE bikes SET status = ?1, updated_at = ?2, latitude = ?3, longitude = ?4 WHERE id = ?5",
                    rusqlite::params![status.as_str(), now, lat_val, lon_val, bike_id],
                )?;
            }
            (None, None, Some(bat_val)) => {
                self.conn.execute(
                    "UPDATE bikes SET status = ?1, updated_at = ?2, battery_level = ?3 WHERE id = ?4",
                    rusqlite::params![status.as_str(), now, bat_val as i32, bike_id],
                )?;
            }
            _ => {
                self.conn.execute(
                    "UPDATE bikes SET status = ?1, updated_at = ?2 WHERE id = ?3",
                    rusqlite::params![status.as_str(), now, bike_id],
                )?;
            }
        }

        Ok(())
    }

    // ========================================================================
    // Delivery Queries
    // ========================================================================

    /// Get all deliveries, optionally filtered by bike_id and/or status
    ///
    /// # Why filtering at database level?
    /// - More efficient than fetching all and filtering in Rust
    /// - Reduces data transfer over IPC
    /// - Enables pagination in the future
    pub fn get_deliveries(
        &self,
        bike_id: Option<&str>,
        status: Option<&str>,
    ) -> Result<Vec<Delivery>, DatabaseError> {
        let mut sql = String::from(
            r#"SELECT id, bike_id, status, customer_name, customer_address,
                      restaurant_name, restaurant_address, rating, complaint,
                      created_at, completed_at
               FROM deliveries WHERE 1=1"#,
        );

        // Dynamic query building for optional filters
        if bike_id.is_some() {
            sql.push_str(" AND bike_id = ?1");
        }
        if status.is_some() {
            sql.push_str(if bike_id.is_some() {
                " AND status = ?2"
            } else {
                " AND status = ?1"
            });
        }
        sql.push_str(" ORDER BY created_at DESC");

        let mut stmt = self.conn.prepare(&sql)?;

        // Execute with appropriate params based on filters
        let rows = match (bike_id, status) {
            (Some(b), Some(s)) => stmt.query(rusqlite::params![b, s])?,
            (Some(b), None) => stmt.query(rusqlite::params![b])?,
            (None, Some(s)) => stmt.query(rusqlite::params![s])?,
            (None, None) => stmt.query([])?,
        };

        self.map_delivery_rows(rows)
    }

    /// Get a single delivery by ID
    pub fn get_delivery_by_id(&self, delivery_id: &str) -> Result<Option<Delivery>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r#"SELECT id, bike_id, status, customer_name, customer_address,
                      restaurant_name, restaurant_address, rating, complaint,
                      created_at, completed_at
               FROM deliveries WHERE id = ?1"#,
        )?;

        let delivery = stmt
            .query_row([delivery_id], |row| self.map_delivery_row(row))
            .optional()?;

        Ok(delivery)
    }

    /// Get deliveries for a specific bike (for force graph)
    ///
    /// # Why a dedicated method?
    /// - Force graph needs all deliveries for a single bike
    /// - Simpler API than using get_deliveries with filter
    pub fn get_deliveries_by_bike(&self, bike_id: &str) -> Result<Vec<Delivery>, DatabaseError> {
        self.get_deliveries(Some(bike_id), None)
    }

    /// Map SQLite rows to Delivery structs
    fn map_delivery_rows(&self, mut rows: rusqlite::Rows) -> Result<Vec<Delivery>, DatabaseError> {
        let mut deliveries = Vec::new();
        while let Some(row) = rows.next()? {
            deliveries.push(self.map_delivery_row(row)?);
        }
        Ok(deliveries)
    }

    /// Map a single SQLite row to Delivery
    fn map_delivery_row(&self, row: &rusqlite::Row) -> rusqlite::Result<Delivery> {
        let status_str: String = row.get(2)?;
        let status = DeliveryStatus::from_str(&status_str).unwrap_or(DeliveryStatus::Upcoming);

        Ok(Delivery {
            id: row.get(0)?,
            bike_id: row.get(1)?,
            status,
            customer_name: row.get(3)?,
            customer_address: row.get(4)?,
            restaurant_name: row.get(5)?,
            restaurant_address: row.get(6)?,
            rating: row.get::<_, Option<i32>>(7)?.map(|r| r as u8),
            complaint: row.get(8)?,
            created_at: row
                .get::<_, String>(9)?
                .parse::<chrono::DateTime<Utc>>()
                .unwrap_or_else(|_| Utc::now()),
            completed_at: row
                .get::<_, Option<String>>(10)?
                .and_then(|s| s.parse::<chrono::DateTime<Utc>>().ok()),
        })
    }

    // ========================================================================
    // Issue Queries
    // ========================================================================

    /// Get all issues, optionally filtered
    ///
    /// # Filter options
    /// - bike_id: Issues for a specific deliverer
    /// - resolved: Filter by resolution status
    /// - category: Filter by issue category
    pub fn get_issues(
        &self,
        bike_id: Option<&str>,
        resolved: Option<bool>,
        category: Option<&str>,
    ) -> Result<Vec<Issue>, DatabaseError> {
        let mut sql = String::from(
            r#"SELECT id, delivery_id, bike_id, reporter_type, category,
                      description, resolved, created_at
               FROM issues WHERE 1=1"#,
        );

        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        let mut param_idx = 1;

        if let Some(b) = bike_id {
            sql.push_str(&format!(" AND bike_id = ?{}", param_idx));
            params.push(Box::new(b.to_string()));
            param_idx += 1;
        }
        if let Some(r) = resolved {
            sql.push_str(&format!(" AND resolved = ?{}", param_idx));
            params.push(Box::new(r as i32));
            param_idx += 1;
        }
        if let Some(c) = category {
            sql.push_str(&format!(" AND category = ?{}", param_idx));
            params.push(Box::new(c.to_string()));
        }
        sql.push_str(" ORDER BY created_at DESC");

        let mut stmt = self.conn.prepare(&sql)?;

        // Convert params to references for execution
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query(param_refs.as_slice())?;

        self.map_issue_rows(rows)
    }

    /// Get a single issue by ID
    pub fn get_issue_by_id(&self, issue_id: &str) -> Result<Option<Issue>, DatabaseError> {
        let mut stmt = self.conn.prepare(
            r#"SELECT id, delivery_id, bike_id, reporter_type, category,
                      description, resolved, created_at
               FROM issues WHERE id = ?1"#,
        )?;

        let issue = stmt
            .query_row([issue_id], |row| self.map_issue_row(row))
            .optional()?;

        Ok(issue)
    }

    /// Get issues for a specific bike (for force graph)
    pub fn get_issues_by_bike(&self, bike_id: &str) -> Result<Vec<Issue>, DatabaseError> {
        self.get_issues(Some(bike_id), None, None)
    }

    /// Map SQLite rows to Issue structs
    fn map_issue_rows(&self, mut rows: rusqlite::Rows) -> Result<Vec<Issue>, DatabaseError> {
        let mut issues = Vec::new();
        while let Some(row) = rows.next()? {
            issues.push(self.map_issue_row(row)?);
        }
        Ok(issues)
    }

    /// Map a single SQLite row to Issue
    fn map_issue_row(&self, row: &rusqlite::Row) -> rusqlite::Result<Issue> {
        let reporter_str: String = row.get(3)?;
        let category_str: String = row.get(4)?;
        let resolved: i32 = row.get(6)?;

        Ok(Issue {
            id: row.get(0)?,
            delivery_id: row.get(1)?,
            bike_id: row.get(2)?,
            reporter_type: IssueReporterType::from_str(&reporter_str)
                .unwrap_or(IssueReporterType::Customer),
            category: IssueCategory::from_str(&category_str).unwrap_or(IssueCategory::Other),
            description: row.get(5)?,
            resolved: resolved != 0,
            created_at: row
                .get::<_, String>(7)?
                .parse::<chrono::DateTime<Utc>>()
                .unwrap_or_else(|_| Utc::now()),
        })
    }

    // ========================================================================
    // Statistics
    // ========================================================================

    /// Get database statistics
    pub fn get_stats(&self) -> Result<DatabaseStats, DatabaseError> {
        let total_bikes: u32 = self
            .conn
            .query_row("SELECT COUNT(*) FROM bikes", [], |row| row.get(0))?;

        let total_trips: u32 = self
            .conn
            .query_row("SELECT COALESCE(SUM(total_trips), 0) FROM bikes", [], |row| {
                row.get(0)
            })?;

        Ok(DatabaseStats {
            total_bikes,
            total_trips,
            database_size_bytes: 0, // Would need file system access
            last_sync: Some(Utc::now()),
        })
    }
}

/// Generate a simple UUID-like string (not cryptographically secure, for demo purposes)
fn uuid_v4_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", now)
}
