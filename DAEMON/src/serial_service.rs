use std::collections::HashMap;

use crate::sensors::{Node, PirEvent, PlantData, Registry, SensorEvent, LD2450Event, MovementTarget};


const TYPE_PLANT:u8 = 1;
const TYPE_PIR: u8 = 2;
const TYPE_LD2450: u8 = 3;

pub fn load_nodes() -> HashMap<u8, Node> {
    let txt = std::fs::read_to_string("/home/compiledtheory/daemon/nodes.toml").unwrap();
    let reg: Registry = toml::from_str(&txt).unwrap();
    return reg.node.into_iter().map(|n| (n.id, n)).collect();
}

pub fn handle_frame(
    frame: &[u8], 
    nodes: &HashMap<u8, Node>,
    tx: &tokio::sync::broadcast::Sender<SensorEvent>
) {
    if frame.len() < 2 { return ;}
    let (ptype, id) = (frame[0], frame[1]);

    let Some(node) = nodes.get(&id) else { return };

    match ptype {
        TYPE_PLANT => {
            if frame.len() < 3 { return; }
            let _ = tx.send(SensorEvent::Plant(parse_plant(frame, node)));

        },
        TYPE_PIR => { let _ = tx.send(SensorEvent::Pir(parse_pir(node))); },
        TYPE_LD2450 => { let _ = tx.send(SensorEvent::LD2450(parse_ld2450(frame[2], &frame[3..]))); },
        _ => {},
    }
}

fn parse_plant(frame: &[u8], node: &Node) -> PlantData {
    PlantData { sensor: node.id, plant: node.name.clone(), moisture: frame[2].into(), threshold: node.moisture_threshold.unwrap_or(50).into() }
}

fn parse_pir(node: &Node) -> PirEvent {
    PirEvent { location: node.name.clone() }
}

fn parse_ld2450(count: u8, frame: &[u8]) -> LD2450Event {
    let mut targets = Vec::<MovementTarget>::with_capacity(count.into());

    for (_, chunk) in frame.chunks_exact(6).enumerate() {
        let x = u16::from_le_bytes([chunk[0], chunk[1]]) as i16;
        let y = u16::from_le_bytes([chunk[2], chunk[3]]) as i16;
        let speed = u16::from_le_bytes([chunk[4], chunk[5]]) as i16;
        targets.push( MovementTarget { x, y, speed })
    }
    LD2450Event { targets }
}
