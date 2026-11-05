mod sensors;
mod db;
mod serial_speaker;
mod serial_service;
mod audio_service;
mod stt;
mod tts;
mod llm;
mod tools;
mod audio_listener;
mod events;
mod state;

use tokio::sync::{broadcast, watch};
#[cfg(unix)]
use tokio::signal::unix::{signal, SignalKind};

use std::sync::Arc;
use crate::state::SystemState;
use crate::sensors::SensorEvent;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let (state_tx, state_rx) = watch::channel(SystemState::Idle);
    let (serial_tx, _) = broadcast::channel::<SensorEvent>(64);
    let (record_tx, record_rx) = tokio::sync::mpsc::channel::<Arc<[i16]>>(100);
    let (transcribe_tx, transcribe_rx) = tokio::sync::mpsc::channel::<String>(100);
    let (speak_tx, speak_rx) = tokio::sync::mpsc::channel::<String>(100);

    tokio::spawn(state::state_watchdog(state_rx.clone(), state_tx.clone()));
    db::create_dbs().expect("Failed to initialize databases");
    tokio::spawn(db::db_consumer(serial_tx.subscribe()));
    tokio::spawn(events::event_handler(serial_tx.subscribe()));
    
    let nodes = serial_service::load_nodes();
    tokio::spawn(async move { serial_speaker::read("/dev/ttyACM0", nodes, serial_tx).await; });

    let _record_stream = audio_service::record(&std::env::var("MICROPHONE").unwrap(), record_tx).await;

    stt::stt_worker(record_rx, transcribe_tx, state_rx.clone(), state_tx.clone());
    tokio::spawn(llm::llm_worker(transcribe_rx, speak_tx, state_rx.clone(), state_tx.clone()));
    tts::tts_worker(speak_rx, state_tx.clone());

    println!("S.T.A.R.K. daemon is running. Press Ctrl+C to exit.");
    shutdown_signal().await;
    
    println!("\nShutting down S.T.A.R.K....");
    drop(_record_stream);

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal(SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {
            println!("\nReceived Ctrl+C (SIGINT)");
        },
        _ = terminate => {
            println!("\nReceived Systemd Stop (SIGTERM)");
        },
    }
}