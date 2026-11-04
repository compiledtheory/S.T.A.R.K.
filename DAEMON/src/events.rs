use tokio::{sync::broadcast};
use crate::sensors::{PirEvent, SensorEvent};

pub async fn event_handler(mut rx: broadcast::Receiver<SensorEvent>) {
    loop {
        match rx.recv().await {
            Ok(SensorEvent::Pir(event)) => {
                println!("Motion at x:{} y:{}", event.x, event.y)
            },
            Ok(_) => {},
            Err(err) => { eprintln!("error: {err}"); } 
        }
    }
}