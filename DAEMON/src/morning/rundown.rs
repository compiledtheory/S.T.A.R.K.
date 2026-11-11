use chrono::{DateTime, Local};
use crate::morning::prompt::STARK_MORNING_CONTEXT;
use crate::calendar::CalendarEvent;

pub struct MorningContext {
    date: DateTime<Local>,
    location: String,
    calendar_events: Vec<CalendarEvent>,
}

impl MorningContext {
    pub fn new(date: DateTime<Local>, location: String, calendar_events: Vec<CalendarEvent>) -> Self {
        Self { date, location, calendar_events }
    }

    pub fn format(&self) -> String {
        STARK_MORNING_CONTEXT
            .replace("{current_date}", &self.date.format("%Y-%m-%d %H:%M:%S").to_string())
            .replace("{current_location}", &self.location)
            .replace("{calendar_events}", &self.format_events())
    }

    fn format_events(&self) -> String {
        if self.calendar_events.is_empty() {
            return "No events scheduled for today.".to_string();
        }

        self.calendar_events
            .iter()
            .map(|event| event.as_string())
            .collect::<Vec<String>>()
            .join("\n")
    }
}