use tokio::sync::broadcast;
use crate::sensors::{SensorEvent, MotionEvent};
use crate::spatial::position::Positions;

pub async fn start(
    mut rx: broadcast::Receiver<SensorEvent>,
    frontend_tx: broadcast::Sender<String>,
) {
    loop {
        match rx.recv().await {
            Ok(SensorEvent::Pir(event)) | Ok(SensorEvent::LD2450(event)) => {
                let _ = frontend_tx.send(Positions::create(event).as_string());
            },
            Ok(_) => {},
            Err(err) => { eprintln!("error: {err}"); } 
        }
    }
}

