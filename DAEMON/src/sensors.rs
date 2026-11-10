use serde::Deserialize;

#[derive(Clone, Deserialize, Debug)]
pub struct PlantData {
    pub sensor: u8,
    pub plant: String,
    pub moisture: u16,
    pub threshold: u16,
}

#[derive(Deserialize, Clone)]
pub struct Node {
    pub id: u8,
    pub kind: String,
    pub name: String,
    pub moisture_threshold: Option<u16>,
}

#[derive(Deserialize)]
pub struct Registry {
    pub node: Vec<Node>,
}

#[derive(Clone, Deserialize)]
pub struct PirEvent{
    pub location: String,
}

#[derive(Clone, Deserialize)]
pub struct LD2450Event{
    pub targets: Vec<MovementTarget>,
}

#[derive(Clone, Deserialize)]
pub struct MovementTarget {
    pub x: i16,
    pub y: i16,
    pub speed: i16,
}


#[derive(Clone)]
pub enum SensorEvent {
    Plant(PlantData),
    Pir(PirEvent),
    LD2450(LD2450Event),
}