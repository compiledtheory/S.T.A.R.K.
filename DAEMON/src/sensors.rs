use serde::Deserialize;

#[derive(Clone, Deserialize, Debug)]
pub struct PlantData {
    pub sensor: u8,
    pub plant: String,
    pub moisture: u16,
}

#[derive(Deserialize, Clone)]
pub struct Node {
    pub id: u8,
    pub kind: String,
    pub name: String,
}

#[derive(Deserialize)]
pub struct Registry {
    pub node: Vec<Node>,
}

#[derive(Clone, Deserialize)]
pub struct PirEvent{
    pub x: i16,
    pub y: i16,
}

#[derive(Clone)]
pub enum SensorEvent {
    Plant(PlantData),
    Pir(PirEvent)
}