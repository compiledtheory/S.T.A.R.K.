use std::sync::LazyLock;

use crate::{db, sensors::SensorMessage};
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub static TOOLS: LazyLock<Value> = LazyLock::new(|| {
    json!([
        tool::<GetMoistureOnePlantArgs>(
            "get_moisture_one_plant",
            "Fetches the most recent soil moisture reading for a single specific indoor plant from the database, along with the date."
        ),
        tool::<GetMoistureMultipleArgs>(
            "get_moisture_multiple_plants",
            "Fetches the most recent soil moisture readings for a list of multiple plants at once, along with the dates."
        ),
        tool::<WateringEventOneArgs>(
            "get_watering_event_for_plant",
            "Fetches the most recent watering date for a single specific indoor plant from the database."
        ),
        tool::<WateringEventOneArgs>(
            "post_watering_event_for_plant",
            "Adds a new entity into watering history database for the provided plant using current timestamp."
        )
    ])
});

fn tool<T: JsonSchema>(name: &str, description: &str) -> Value {
    let mut params = serde_json::to_value(schema_for!(T)).unwrap();
    if let Some(obj) = params.as_object_mut() {
        obj.remove("$schema");
        obj.remove("title");
    }

    return json!({
        "type": "function",
        "function": {
            "name": name,
            "description": description,
            "parameters": params,
        }
    })
}

pub async fn handle_tool_call(tool_name: &str, args: &str) -> String {
    let tool_args = serde_json::from_str(args).unwrap();
    match tool_name {
        "get_moisture_one_plant" => return get_moisture_one(tool_args).await,
        "get_moisture_multiple_plants" => return get_moisture_multiple(tool_args).await,
        "get_watering_event_for_plant" => return get_watering_event_for_plant(tool_args).await,
        "post_watering_event_for_plant" => return post_watering_event_for_plant(tool_args).await,
        _ => return format!("{{\"error\": \"Unknown tool {}\"}}", tool_name),
    }
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct GetMoistureOnePlantArgs {
    pub plant: String,
}

async fn get_moisture_one(args: serde_json::Value) -> String {
    let plant_name = args["plant"].as_str().unwrap().to_string();
    let (moisture, timestamp) = tokio::task::spawn_blocking(move || {
        db::get_moisture_for_plant(plant_name)
    }).await.unwrap_or_else(|_| (0.0, String::new()));

    return format!("{{\"moisture\": {}, \"timestamp\": \"{}\"}}", moisture, timestamp)
}

#[derive(Serialize, Deserialize, JsonSchema)]
pub struct GetMoistureMultipleArgs {
    pub plants: Vec<String>,
}

async fn get_moisture_multiple(args: serde_json::Value) -> String {
    let plant_list: Vec<String> = args["plants"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    let moisture_map = tokio::task::spawn_blocking(move || {
        db::get_moisture_for_plants(plant_list)
    }).await.unwrap_or_default();

    let formatted_map: std::collections::HashMap<String, serde_json::Value> = moisture_map
        .into_iter()
        .map(|(plant, (moisture, timestamp))| {
            (
                plant, 
                serde_json::json!({ "moisture": moisture, "timestamp": timestamp })
            )
        })
        .collect();

    return serde_json::to_string(&formatted_map).unwrap_or_else(|_| "{}".to_string());
}


#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct WateringEventOneArgs {
    pub plant: String,
}

async fn post_watering_event_for_plant(args: serde_json::Value) -> String {
    let plant_name = args["plant"].as_str().unwrap().to_uppercase();
    let watering_args = WateringEventOneArgs {
        plant: plant_name,
    };

    let payload = SensorMessage::Watering(watering_args);
    let result = db::store_data(payload).await;
    match result {
        Ok(_) => {
            return format!("{{\"status\": {}}}", "success") 
        },
        Err(err) => {
            return format!("{{\"status\": {}, \"reason\": \"{}\"}}", "error", err)
        }

    }
}

async fn get_watering_event_for_plant(args: serde_json::Value) -> String {
    let plant_name = args["plant"].as_str().unwrap().to_uppercase();
    let timestamp = tokio::task::spawn_blocking(move || {
        db::get_last_watering_for_plant(plant_name)
    }).await.unwrap_or(String::new());

    return format!("{{\"timestamp\": {}}}", timestamp) 
}
