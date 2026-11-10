use reqwest::Client;
use serde_json::json;

pub async fn discord_webhook_worker(mut rx: tokio::sync::mpsc::Receiver<String>) {
    let webhook_url = std::env::var("DISCORD_WEBHOOK_URL")
        .expect("DISCORD_WEBHOOK_URL must be set in the environment");

    let client = Client::new();
    while let Some(msg) = rx.recv().await {
        let payload = json!({
            "content": msg,
            "username": "S.T.A.R.K.",
        });

        match client.post(&webhook_url).json(&payload).send().await {
            Ok(res) => {
                if !res.status().is_success() {
                    eprintln!("Discord Webhook error status: {:?}", res.status());
                } else {
                    println!("S.T.A.R.K. sent a message to Discord.");
                }
            }
            Err(e) => {
                eprintln!("Failed to execute Discord webhook POST: {}", e);
            }
        }
    }
}