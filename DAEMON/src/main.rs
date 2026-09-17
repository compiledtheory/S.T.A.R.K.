mod payloads;
mod db;
mod moisture_threshold;

use serial2_tokio::SerialPort;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::payloads::SensorMessage;


async fn listen_to_port() -> Result<(), Box<dyn std::error::Error>> {
    let port_path = "/dev/ttyACM0";
    let baud_rate = 115200;

    let port = SerialPort::open(port_path, baud_rate)?;
    let mut reader = BufReader::new(port);
    let mut line = String::new();

    println!("Listening on {} at {} baud...", port_path, baud_rate);
    
    loop {
        line.clear();
        let bytes = reader.read_line(&mut line).await?;
        if bytes == 0 {
            break;
        }

        match serde_json::from_str::<SensorMessage>(&line) {
            Ok(SensorMessage::Plant(plant_data)) => {
                if let Err(e) = db::add_plant(&plant_data).await {
                    eprint!("Failed to save to database: {}", e);
                }
            }
            Err(e) => {println!("{}", e)}
        }
        
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let serial_task = tokio::spawn(async {
        if let Err(e) = listen_to_port().await {
            eprintln!("Serial task crashed: {}", e);
        }
    });

    let _ = serial_task.await;

    Ok(())
}
