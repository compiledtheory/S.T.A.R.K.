use serial2_tokio::SerialPort;
use crate::sensors::{SensorEvent, Node};
use tokio::{io::{AsyncBufReadExt, BufReader}, sync::broadcast};
use crate::serial_service::handle_frame;
use std::collections::HashMap;


fn handle_read(
    line: &str, 
    nodes: &HashMap<u8, Node>,
    tx: &broadcast::Sender<SensorEvent>) {
    let line = line.trim();
    if line.is_empty() { return; }

    match hex::decode(line) {
        Ok(frame) => handle_frame(&frame, nodes, tx),
        Err(err) => eprintln!("Bad frame: {}, Error: {}", line, err),
    }
}

pub async fn read(
    port: &str,
    nodes: HashMap<u8, Node>, 
    tx: broadcast::Sender<SensorEvent>

) {
    let baud_rate = 115200;
    let serial_port = match SerialPort::open(port, baud_rate) {
        Ok(port) => port,
        Err(e) => {
            eprintln!("Failed to open serial port {}: {}", port, e);
            return; 
        }
    };

    let mut reader = BufReader::new(serial_port); 
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break,
            Ok(_) => {handle_read(line.trim(), &nodes, &tx);}
            Err(e) => {
                eprintln!("Read error: {}", e);
                break;
            }
        }
    }

}