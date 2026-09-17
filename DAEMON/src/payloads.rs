use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum SensorMessage {
    Plant(PlantData),
}

#[derive(Deserialize, Debug)]
pub struct PlantData {
    pub sensor: i32,
    pub plant: String,
    pub moisture: f32,
}   