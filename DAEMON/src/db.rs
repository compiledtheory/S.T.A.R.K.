use rusqlite::{params, Connection};
use crate::payloads::PlantData;
use crate::moisture_threshold::notify_moisture_levels;

pub async fn add_plant(data: &PlantData) -> rusqlite::Result<()> {
    let conn = Connection::open("stark.db")?;
    notify_moisture_levels(&data.plant, data.moisture);

    conn.execute(
        "CREATE TABLE IF NOT EXISTS plant_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            sensor_id INTEGER,
            plant_name TEXT,
            moisture REAL,
            timestamp DATETIME DEFAULT CURRENT_TIMESTAMP)",[],)?;

    conn.execute(
        "INSERT INTO plant_history (sensor_id, plant_name, moisture)
            VALUES (?1, ?2, ?3)", params![data.sensor, data.plant, data.moisture],)?;

    Ok(())
}