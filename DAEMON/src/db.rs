use rusqlite::{Connection, params};
use tokio::task::JoinError;
use crate::sensors::{PlantData, SensorMessage};
use std::collections::HashMap;
use crate::tools::WateringEventOneArgs;


fn add_plant(data: &PlantData) -> rusqlite::Result<()> {
    let conn = open_connection();
    conn.execute(
        "INSERT INTO plant_history (sensor_id, plant, moisture)
            VALUES (?1, ?2, ?3)", params![data.sensor, data.plant, data.moisture],)?;

    Ok(())
}

fn add_watering_event(data: &WateringEventOneArgs) -> rusqlite::Result<()> {
    let conn = open_connection();
    conn.execute(
        "INSERT INTO watering_history (plant)
        VALUES (?1)", params![data.plant],)?;

    Ok(())
}

fn handle_write(result: Result<rusqlite::Result<()>, JoinError>) -> Result<(), String>{
    match result {
        Ok(Ok(())) => {Ok(())},
        Ok(Err(err)) => {
            let msg = format!("SQLite error while saving data: {}", err);
            Err(msg)
        },
        Err(err) => {
            let msg = format!("Database thread panicked: {}", err);
            Err(msg)
        }
    }
}

pub async fn store_data(msg: SensorMessage) -> Result<(), String>{
    match msg {
        SensorMessage::Plant(msg) => handle_write(tokio::task::spawn_blocking(move || add_plant(&msg)).await),
        SensorMessage::Watering(msg) => handle_write(tokio::task::spawn_blocking(move || add_watering_event(&msg)).await)  
    }
}

pub fn create_dbs() -> rusqlite::Result<()> {
    let conn = open_connection();

    conn.execute(
        "CREATE TABLE IF NOT EXISTS plant_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            sensor_id INTEGER,
            plant TEXT,
            moisture REAL,
            timestamp DATETIME DEFAULT CURRENT_TIMESTAMP)",[],)?;


    conn.execute(
        "CREATE TABLE IF NOT EXISTS watering_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            plant TEXT,
            timestamp DATEMTIME DEFAULT CURRENT_TIMESTAMP)", [],)?;

    Ok(())
}

pub fn get_last_watering_for_plant(plant: String) -> String {
    let conn = open_connection();
    let query = "
        SELECT timestamp
        FROM watering_history
        WHERE plant = ?1 COLLATE NOCASE
        ORDER BY timestamp DESC
        LIMIT 1
        ";

    let result = conn.query_row(query, params![plant], |row| {
        Ok(row.get(0)?)
    });

    return result.unwrap();
}

pub fn get_moisture_for_plant(plant: String) -> (f32, String){
    let conn = open_connection();
    let query = "
        SELECT moisture, timestamp
        FROM plant_history
        WHERE plant = ?1 COLLATE NOCASE
        ORDER BY timestamp DESC
        LIMIT 1";

    let result = conn.query_row(query, params![plant], |row| {
        Ok((row.get(0)?, row.get(1)?))
    });

    return result.unwrap_or_else(|_| (0.0, String::new()))
}

pub fn get_moisture_for_plants(plants: Vec<String>) -> HashMap<String, (f32, String)> {
    let mut results: HashMap<String, (f32, String)> = plants.iter().map(|p| (p.clone(), (0.0, String::new()))).collect();
    if plants.is_empty() {
        return results;
    }

    let conn = open_connection();
    let placeholders = vec!["?"; plants.len()].join(", ");
    let query = format!("
        SELECT plant, moisture, MAX(timestamp)
        FROM plant_history
        WHERE plant COLLATE NOCASE IN ({})
        GROUP BY plant COLLATE NOCASE
    ", placeholders);

    let mut stmt = conn.prepare(&query).expect("Failed to prepare statement");
    let mut rows = stmt.query(rusqlite::params_from_iter(&plants)).expect("Failed to execute query");
    while let Ok(Some(row)) = rows.next() {
        let db_name: String = row.get(0).unwrap();
        let moisture: f32 = row.get(1).unwrap_or(0.0);
        let timestamp: String = row.get(2).unwrap_or_default();
        if let Some(requested) = plants.iter().find(|p| p.eq_ignore_ascii_case(&db_name)){
            results.insert(requested.clone(), (moisture, timestamp));
        }
    }

    return results;
}

fn open_connection() -> Connection {
    return Connection::open("stark.db").expect("Failed to establish connection with database.")
}
