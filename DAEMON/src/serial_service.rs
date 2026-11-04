use std::collections::HashMap;

use crate::{db, sensors::{Node, PirEvent, PlantData, Registry, SensorEvent}};


const TYPE_PLANT:u8 = 1;
const TYPE_PIR: u8 = 2;

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
        TYPE_PIR => { tx.send(SensorEvent::Pir(parse_pir(frame, node))); },
        _ => {},
    }
}

fn parse_plant(frame: &[u8], node: &Node) -> PlantData {
    PlantData { sensor: node.id, plant: node.name.clone(), moisture: frame[2].into() }
}

fn parse_pir(frame: &[u8], node: &Node) -> PirEvent {
    PirEvent { x: 3500, y: 4000 }
}