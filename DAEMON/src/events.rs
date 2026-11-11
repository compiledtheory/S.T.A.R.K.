use tokio::sync::broadcast;
use crate::sensors::{MotionEvent, SensorEvent, PlantData, MovementTarget};
use chrono::{Local, Duration};
use crate::home_state::StateCommand;
use crate::morning::rundown::MorningContext;
use crate::plants::context::PlantContext;
use crate::calendar::CalendarCommand;
use crate::llm::{LLMContext, LLMOutputRoute};

pub async fn event_handler(
    mut rx: broadcast::Receiver<SensorEvent>,
    home_state_tx: tokio::sync::mpsc::Sender<StateCommand>,
    agent_tx: tokio::sync::mpsc::Sender<LLMContext>,
    calendar_tx: tokio::sync::mpsc::Sender<CalendarCommand>,
) {
    loop {
        match rx.recv().await {
            Ok(SensorEvent::Pir(event)) => { handle_pir_events(&home_state_tx, &agent_tx, &calendar_tx, event).await; },
            Ok(SensorEvent::LD2450(event)) => { println!("{} Targets detected", event.targets.len());},
            Ok(SensorEvent::Plant(event)) => { handle_plant_events(&home_state_tx, &agent_tx, event).await },
            Err(err) => { eprintln!("error: {err}"); } 
        }
    }
}

async fn handle_pir_events(
    home_state_tx: &tokio::sync::mpsc::Sender<StateCommand>,
    agent_tx: &tokio::sync::mpsc::Sender<LLMContext>,
    calendar_tx: &tokio::sync::mpsc::Sender<CalendarCommand>,
    event: MotionEvent, 
) { 
    let target = &event.targets[0];
    if let Err(e) = home_state_tx.send(StateCommand::UpdateZone(target.location.clone())).await {
        eprintln!("Failed to send state update: {}", e);
    }

    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();

    if let Err(e) = home_state_tx.send(StateCommand::CheckLastRundownDate(reply_tx)).await {
        eprintln!("Failed to request rundown date: {}", e);
    } else {
        if let Ok(last_rundown) = reply_rx.await {
            if last_rundown != Some(Local::now().date_naive()) {
                trigger_morning_rundown(home_state_tx, agent_tx, calendar_tx, target).await;
            }           
        }
    }
}

async fn trigger_morning_rundown(
    home_state_tx: &tokio::sync::mpsc::Sender<StateCommand>,
    agent_tx: &tokio::sync::mpsc::Sender<LLMContext>,
    calendar_tx: &tokio::sync::mpsc::Sender<CalendarCommand>,
    target: &MovementTarget,
) {
    
    if let Err(e) = home_state_tx.send(StateCommand::RegisterRundown).await {
        eprintln!("Failed to register rundown: {}", e);
    }

    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    let mut todays_events = Vec::new();

    if let Err(e) = calendar_tx.send(CalendarCommand::GetTodaysEvents(reply_tx)).await {
        eprintln!("Failed to request calendar events: {}", e);
    } else {
        if let Ok(events) = reply_rx.await {
            todays_events = events;
        }
    }

    let context = MorningContext::new(Local::now(), target.location.clone(), todays_events);
    if let Err(e) = agent_tx.send(LLMContext::new(context.format(), true)).await {
        eprintln!("Failed to send morning prompt to LLM: {}", e);
    }
}

async fn handle_plant_events(
    home_state_tx: &tokio::sync::mpsc::Sender<StateCommand>,
    agent_tx: &tokio::sync::mpsc::Sender<LLMContext>,
    event: PlantData,
) {
    if event.moisture < event.threshold {
        let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
        let mut is_home = false;
        if let Err(e) = home_state_tx.send(StateCommand::CheckLastMotionTime(reply_tx)).await {
            eprintln!("Failed to check if somebody is home, defaulting to Discord: {}", e);
        } else {
            if let Ok(last_seen) = reply_rx.await {
                let ten_minutes_ago = Local::now() - Duration::try_minutes(1).expect("valid duration");
                if last_seen > ten_minutes_ago { is_home = true; }
            }
        }
        let plant_context = PlantContext::new(event.plant.clone(), event.moisture, event.threshold, is_home);

        if let Err(e) = agent_tx.send(LLMContext::new(plant_context.format() ,is_home)).await {
            eprintln!("Failed to send plant warning to LLM: {}", e);
        }

    }

}

