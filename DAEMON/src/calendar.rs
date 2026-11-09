use chrono::{Local, TimeZone};
use tokio::sync::oneshot;
use reqwest::Client;
use yup_oauth2::{read_service_account_key, ServiceAccountAuthenticator};
use yup_oauth2::authenticator::Authenticator;
use serde_json::Value;
use yup_oauth2::hyper_rustls::HttpsConnector;
use hyper_util::client::legacy::connect::HttpConnector;

pub enum CalendarCommand {
    GetTodaysEvents(oneshot::Sender<Vec<CalendarEvent>>),
}

#[derive(Debug, Clone)]
pub struct CalendarEvent {
    pub summary: String,
    pub start_time: String,
}

impl CalendarEvent {
    pub fn as_string(&self) -> String {
        
        let time = if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&self.start_time) {
            dt.format("%H:%M").to_string()
        } else {
            self.start_time.clone()
        };

        format!("- {} at {}", self.summary, time)
    }
}

pub async fn calendar_manager(mut rx: tokio::sync::mpsc::Receiver<CalendarCommand>) {
    let key_path = std::env::var("CALENDAR_API_KEY_PATH")
        .expect("CALENDAR_API_KEY_PATH must be set in .env");
    
    let secret = read_service_account_key(&key_path)
        .await
        .unwrap_or_else(|_| panic!("Failed to read calendar key at {}", key_path));

    let auth = yup_oauth2::ServiceAccountAuthenticator::builder(secret)
        .build()
        .await
        .expect("Failed to build calendar auth");

    let client = reqwest::Client::new();
    let calendar_id = std::env::var("CALENDAR_ID")
        .expect("CALENDAR_ID must be set in .env");

    while let Some(cmd) = rx.recv().await {
        match cmd {
            CalendarCommand::GetTodaysEvents(reply) => {
                let events = fetch_todays_events(&auth, &client, &calendar_id).await.unwrap_or_default();
                let _ = reply.send(events);
            }
        }
    }
}

async fn fetch_todays_events(
    auth: &Authenticator<HttpsConnector<HttpConnector>>,
    client: &Client,
    calendar_id: &str,
) -> Result<Vec<CalendarEvent>, Box<dyn std::error::Error + Send + Sync>> {

    let token: yup_oauth2::AccessToken = auth
        .token(&["https://www.googleapis.com/auth/calendar"])
        .await?;

    let access_token = token
        .token()
        .ok_or("OAuth returned an empty access token")?;
    
    let now = Local::now();
    let start_of_day = now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Local).unwrap();
    let end_of_day = now.date_naive().and_hms_opt(23, 59, 59).unwrap().and_local_timezone(Local).unwrap();

    let encoded_cal_id = calendar_id.replace("@", "%40");
    let encoded_start = start_of_day.to_rfc3339().replace("+", "%2B").replace(":", "%3A");
    let encoded_end = end_of_day.to_rfc3339().replace("+", "%2B").replace(":", "%3A");

    let url = format!(
        "https://www.googleapis.com/calendar/v3/calendars/{}/events?timeMin={}&timeMax={}&orderBy=startTime&singleEvents=true",
        encoded_cal_id,
        encoded_start,
        encoded_end
    );

    let response = client.get(&url)
        .bearer_auth(access_token)
        .send()
        .await?;

    let mut events = Vec::<CalendarEvent>::new();
    if response.status().is_success() {
        let json_data: Value = response.json().await?;
        if let Some(items) = json_data["items"].as_array() {
            for item in items {
                let summary = item["summary"].as_str().unwrap_or("Untitled Event").to_string();
                let start_time = item["start"]["dateTime"]
                    .as_str()
                    .or_else(|| item["start"]["date"].as_str())
                    .unwrap_or("All day")
                    .to_string();

                events.push(CalendarEvent { summary, start_time });
            }
        }
    } else {
        eprintln!("Calendar API Error: {:?}", response.text().await?);
    }

    Ok(events)
}