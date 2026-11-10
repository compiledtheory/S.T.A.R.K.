use crate::plants::prompt::{STARK_MOISTURE_ALERT_DISCORD_PROMPT, STARK_MOISTURE_ALERT_TTS_PROMPT};

pub struct PlantContext {
    plant: String,
    moisture: u16,
    threshold: u16,
    is_home: bool
}

impl PlantContext {

    pub fn new(plant: String, moisture: u16, threshold: u16, is_home: bool) -> Self {
        Self { plant, moisture, threshold, is_home }
    }

    pub fn format(&self) -> String { 
        let template = if self.is_home { STARK_MOISTURE_ALERT_TTS_PROMPT } else { STARK_MOISTURE_ALERT_DISCORD_PROMPT };
        template
            .replace("{plant_name}", &self.plant)
            .replace("{moisture}", &self.moisture.to_string())
            .replace("{threshold}", &self.threshold.to_string())
    }
}