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
mod home_state;
mod morning;
mod calendar;
mod plants;
mod webhooks;

use tokio::sync::{broadcast, watch};
#[cfg(unix)]
use tokio::signal::unix::{signal, SignalKind};

use std::sync::Arc;
use crate::state::SystemState;
use crate::home_state::{home_state_manager, StateCommand};
use crate::sensors::SensorEvent;
use crate::calendar::CalendarCommand;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let (system_state_tx, system_state_rx) = watch::channel(SystemState::Idle);
    let (home_state_tx, home_state_rx) = tokio::sync::mpsc::channel::<StateCommand>(32);
    let (serial_tx, _) = broadcast::channel::<SensorEvent>(64);
    let (record_tx, record_rx) = tokio::sync::mpsc::channel::<Arc<[i16]>>(100);
    let (agent_tx, agent_rx) = tokio::sync::mpsc::channel::<llm::LLMContext>(100);
    let (speak_tx, speak_rx) = tokio::sync::mpsc::channel::<String>(100);
    let (calendar_tx, calendar_rx) = tokio::sync::mpsc::channel::<CalendarCommand>(32);
    let (discord_tx, discord_rx) = tokio::sync::mpsc::channel::<String>(100);

    tokio::spawn(state::state_watchdog(system_state_rx.clone(), system_state_tx.clone()));
    tokio::spawn(calendar::calendar_manager(calendar_rx));
    db::create_dbs().expect("Failed to initialize databases");
    
    tokio::spawn(webhooks::discord::discord_webhook_worker(discord_rx));
    tokio::spawn(home_state_manager(home_state_rx));
    tokio::spawn(db::db_consumer(serial_tx.subscribe()));
    tokio::spawn(events::event_handler(serial_tx.subscribe(), home_state_tx, agent_tx.clone(), calendar_tx.clone()));
    
    let nodes = serial_service::load_nodes();
    tokio::spawn(async move { serial_speaker::read("/dev/ttyACM0", nodes, serial_tx).await; });

    let _record_stream = audio_service::record(&std::env::var("MICROPHONE").unwrap(), record_tx).await;

    stt::stt_worker(record_rx, agent_tx, system_state_rx.clone(), system_state_tx.clone());
    tokio::spawn(llm::llm_worker(agent_rx, speak_tx, discord_tx, system_state_rx.clone(), system_state_tx.clone()));
    tts::tts_worker(speak_rx, system_state_tx.clone());

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