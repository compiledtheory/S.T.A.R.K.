use hound::{WavSpec, WavWriter, SampleFormat};
use reqwest::{Client, multipart};
use std::io::Cursor;

pub struct STTHandler {
    client: Client,
    api_key: String,
    url: String,
    model: String,
}

impl STTHandler {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            api_key: std::env::var("GROQ_API_KEY").expect("Groq API key not set in .env"),
            url: "https://api.groq.com/openai/v1/audio/transcriptions".to_string(),
            model: "whisper-large-v3".to_string(),
        }
    }

    pub async fn speech_to_text(&self, chunk: &[i16]) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let exact_size = 44 + (chunk.len() * 2);
        let mut cursor = Cursor::new(Vec::with_capacity(exact_size));
        Self::write_bytes(&mut cursor, chunk)?;

        let part = reqwest::multipart::Part::bytes(cursor.into_inner())
            .file_name("command.wav")
            .mime_str("audio/wav")?;

        let form = reqwest::multipart::Form::new()
            .part("file", part)
            .text("model", self.model.clone());

        let response = self.client.post(&self.url)
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .await?;

        let json: serde_json::Value = response.json().await?;
        Ok(json["text"].as_str().unwrap_or("").to_string())
    }

    fn write_bytes(cursor: &mut Cursor<Vec<u8>>, chunk: &[i16]) -> Result<(), hound::Error> {
        let mut writer = WavWriter::new(cursor, Self::wavespec())?;
        for &sample in chunk {
            writer.write_sample(sample)?;
        }

        writer.finalize()?;
        Ok(())
    }

    fn wavespec() -> WavSpec {
        WavSpec { 
            channels: 1, 
            sample_rate: 16_000, 
            bits_per_sample: 16, 
            sample_format: SampleFormat::Int
        }
    }
}
