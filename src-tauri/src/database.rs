use crate::models::{Bike, BikeStatus, DatabaseStats};
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
            "#,
        )?;
        Ok(())
    }

    /// Seed the database with mock Amsterdam bike data
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

        let now = Utc::now().to_rfc3339();
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
                    now,
                    now
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
