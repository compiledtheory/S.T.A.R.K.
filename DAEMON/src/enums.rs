use serde::Deserialize;

#[derive(Clone, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum SensorMessage {
    Plant(PlantData),
}

#[derive(Clone, Deserialize, Debug)]
pub struct PlantData {
    pub sensor: i32,
    pub plant: String,
    pub moisture: f32,
}
