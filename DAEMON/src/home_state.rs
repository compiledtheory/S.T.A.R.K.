use chrono::{DateTime, Local, NaiveDate};
use tokio::sync::oneshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeZone {
    Bedroom,
    Kitchen,
    LivingRoom,
    Desk,
    Couch,
    Bathroom,
}

pub struct HomeState {
    last_motion_detected: DateTime<Local>,
    last_morning_rundown: Option<NaiveDate>,
    last_seen: Option<HomeZone>,
}

impl HomeState {
    pub fn new() -> Self {
        Self {
            last_motion_detected: Local::now(),
            last_morning_rundown: None,
            last_seen: None,
        }
    }

    pub fn update_last_zone(&mut self, location: &str) {
        self.last_motion_detected = Local::now();
        let zone = match location {
            "living room" => Some(HomeZone::LivingRoom),
            "bedroom" => Some(HomeZone::Bedroom),
            "bathroom" => Some(HomeZone::Bathroom),
            "couch" => Some(HomeZone::Couch),
            "kitchen" => Some(HomeZone::Kitchen),
            "desk" => Some(HomeZone::Desk),
            _ => None,
        };

        if zone != self.last_seen {
            self.last_seen = zone;
        }
        
    }

    pub fn register_rundown_complete(&mut self) {
        self.last_morning_rundown = Some(Local::now().date_naive());
    }

    pub fn get_last_morning_rundown(&self) -> Option<NaiveDate> {
        self.last_morning_rundown
    }
}

pub enum StateCommand {
    UpdateZone(String),
    RegisterRundown,
    CheckLastRundownDate(oneshot::Sender<Option<NaiveDate>>),
}

pub async fn home_state_manager(mut rx: tokio::sync::mpsc::Receiver<StateCommand>) {
    let mut state = HomeState::new();

    while let Some(cmd) = rx.recv().await {
        match cmd {
            StateCommand::UpdateZone(loc) => state.update_last_zone(&loc),
            StateCommand::RegisterRundown => state.register_rundown_complete(),
            StateCommand::CheckLastRundownDate(reply) => { let _ = reply.send(state.get_last_morning_rundown()); },
        }
    }
}