use tokio::sync::watch;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemState {
    Idle,
    Listening,
    Thinking,
    Speaking,
    AwaitingReply,
}

pub async fn state_watchdog(
    mut watchdog_rx: watch::Receiver<SystemState>,
    watchdog_tx: watch::Sender<SystemState>,
) {
    loop {
        while *watchdog_rx.borrow() != SystemState::AwaitingReply {
            if watchdog_rx.changed().await.is_err() { return; }
        }

        match tokio::time::timeout(Duration::from_secs(8), watchdog_rx.changed()).await {
            Ok(_) => {},
            Err(_) => {
                println!("Hotmic timeout. Reverting to idle.");
                let _ = watchdog_tx.send(SystemState::Idle);
            }
        }
    }
}
