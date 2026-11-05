use hound::{WavSpec, WavWriter, SampleFormat};
use reqwest::Client;
use std::io::Cursor;
use std::sync::Arc;
use tokio::sync::{mpsc, watch};

use crate::state::SystemState;
use crate::audio_listener::{Listener, MAX_RECORDING_SILENCE_FRAMES};
use crate::audio_service;

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

pub fn stt_worker(
    mut record_rx: mpsc::Receiver<Arc<[i16]>>,
    transcribe_tx: mpsc::Sender<String>,
    mut state_rx: watch::Receiver<SystemState>,
    state_tx: watch::Sender<SystemState>
) {
    let stt_handler = Arc::new(STTHandler::new());
    let rt_handle = tokio::runtime::Handle::current();

    std::thread::spawn(move || {
        let mut listener = Listener::new(
            std::env::var("ONNX_PATH").expect("ONNX path not set").as_str(),
            std::env::var("ONNX_NAME").expect("ONNX name not set"),
            0.5
        );

        while let Some(pcm_chunk) = record_rx.blocking_recv() {
            if state_rx.has_changed().unwrap_or(false) {
                let new_state = *state_rx.borrow_and_update();
                if new_state == SystemState::Idle {
                    listener.stop_recording();
                    listener.clear_buffers();
                    println!("System returned to Idle. Buffers cleared.");
                } else if new_state == SystemState::AwaitingReply {
                    listener.reset_silence();
                }
            } 

            let current_state = *state_rx.borrow();
                
            if current_state == SystemState::Speaking {
                listener.clear_buffers();
                listener.stop_recording();
                continue;
            }

            let rms = audio_service::check_silence(&pcm_chunk);
            listener.manage_silence(rms);

            if listener.is_recording && (current_state == SystemState::Idle || current_state == SystemState::AwaitingReply) {
                let _ = state_tx.send(SystemState::Listening);
            }
            
            if !listener.is_recording && current_state == SystemState::Listening {
                let _ = state_tx.send(SystemState::Idle);
            }

            if current_state == SystemState::AwaitingReply && listener.silence_frames >= MAX_RECORDING_SILENCE_FRAMES {
                let _ = state_tx.send(SystemState::Idle);
            }
            
            if let Some(audio) = audio_service::listen(&mut listener, &pcm_chunk, current_state) {
                let stt = Arc::clone(&stt_handler);
                let tx = transcribe_tx.clone();
                let tx_state = state_tx.clone();

                rt_handle.spawn(async move {
                    let _ = tx_state.send(SystemState::Thinking);
                    match stt.speech_to_text(&audio).await {
                        Ok(text) => {
                            println!("User: {}", text);
                            let _ = tx.send(text).await;
                        }
                        Err(err) => {
                            eprintln!("STT Error: {}", err);
                            let _ = tx_state.send(SystemState::Idle);
                        }
                    }
                });
            }
        }
    });
}