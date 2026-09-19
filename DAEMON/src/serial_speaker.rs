use serial2_tokio::SerialPort;
use crate::enums::SensorMessage;
use tokio::{io::{AsyncBufReadExt, BufReader}, sync::broadcast};

fn handle_read(json: &str, tx: &broadcast::Sender<SensorMessage>) {
    if json.is_empty() {return;}
    
    match serde_json::from_str::<SensorMessage>(json) {
        Ok(msg) => {let _ = tx.send(msg);}
        Err(e) => {eprintln!("Invalid JSON: {} - Error: {}", json, e)}
    }
}

pub async fn speak(port: &str, tx: broadcast::Sender<SensorMessage>) {
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
            Ok(_) => {handle_read(line.trim(), &tx);}
            Err(e) => {
                eprintln!("Read error: {}", e);
                break;
            }
        }
    }

}