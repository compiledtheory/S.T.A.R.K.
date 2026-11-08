use tokio::sync::broadcast;
use crate::sensors::{PirEvent, SensorEvent};
use chrono::{DateTime, Local, NaiveDate};
use crate::home_state::StateCommand;
use crate::morning::rundown::MorningContext;

pub async fn event_handler(
    mut rx: broadcast::Receiver<SensorEvent>,
    home_state_tx: tokio::sync::mpsc::Sender<StateCommand>,
    agent_tx: tokio::sync::mpsc::Sender<String>,
) {
    loop {
        match rx.recv().await {
            Ok(SensorEvent::Pir(event)) => { handle_pir_events(event, &home_state_tx, &agent_tx).await; },
            Ok(SensorEvent::LD2450(event)) => {
                println!("{} Targets detected", event.targets.len());
            },
            Ok(SensorEvent::Plant(event)) => {

            },
            Err(err) => { eprintln!("error: {err}"); } 
        }
    }
}

async fn handle_pir_events(
    event: PirEvent, 
    home_state_tx: &tokio::sync::mpsc::Sender<StateCommand>,
    agent_tx: &tokio::sync::mpsc::Sender<String>,
) { 
    if let Err(e) = home_state_tx.send(StateCommand::UpdateZone(event.location.clone())).await {
        eprintln!("Failed to send state update: {}", e);
    }

    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();

    if let Err(e) = home_state_tx.send(StateCommand::CheckLastRundownDate(reply_tx)).await {
        eprintln!("Failed to request rundown date: {}", e);
    } else {
        if let Ok(last_rundown) = reply_rx.await {
            if last_rundown != Some(Local::now().date_naive()) {
                trigger_morning_rundown(home_state_tx, agent_tx, event).await;
            }           
        }
    }
}

async fn trigger_morning_rundown(
    home_state_tx: &tokio::sync::mpsc::Sender<StateCommand>,
    agent_tx: &tokio::sync::mpsc::Sender<String>,
    event: PirEvent,
) {
    if let Err(e) = home_state_tx.send(StateCommand::RegisterRundown).await {
        eprintln!("Failed to register rundown: {}", e);
    }
    let context = MorningContext::new(Local::now(), event.location, Vec::new());
    if let Err(e) = agent_tx.send(context.format()).await {
        eprintln!("Failed to send morning prompt to LLM: {}", e);
    }
}