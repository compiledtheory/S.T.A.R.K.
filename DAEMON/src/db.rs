use rusqlite::{params, Connection};
use tokio::task::JoinError;
use crate::enums::{PlantData, SensorMessage};

fn add_plant(data: &PlantData) -> rusqlite::Result<()> {
    let conn = Connection::open("stark.db")?;
    conn.execute(
        "INSERT INTO plant_history (sensor_id, plant_name, moisture)
            VALUES (?1, ?2, ?3)", params![data.sensor, data.plant, data.moisture],)?;

    Ok(())
}

fn handle_write(result: Result<rusqlite::Result<()>, JoinError>) {
    match result {
        Ok(Ok(())) => {},
        Ok(Err(db_err)) => {eprintln!("SQLite error while saving data: {}", db_err)},
        Err(join_err) => {eprintln!("Database thread panicked: {}", join_err);}
    }
}

pub async fn store_data(msg: SensorMessage) {
    match msg {
        SensorMessage::Plant(msg) => {
            handle_write(tokio::task::spawn_blocking(move || add_plant(&msg)).await);
        }
    }
}

pub fn create_dbs() -> rusqlite::Result<()> {
    let conn = Connection::open("stark.db")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS plant_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            sensor_id INTEGER,
            plant_name TEXT,
            moisture REAL,
            timestamp DATETIME DEFAULT CURRENT_TIMESTAMP)",[],)?;

    Ok(())
}

