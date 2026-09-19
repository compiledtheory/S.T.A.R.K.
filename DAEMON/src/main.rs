mod enums;
mod db;
mod serial_speaker;
mod audio_speaker;
mod audio_listener;
mod stt;
mod llm;

use silero_vad_rust::{VadIterator, load_silero_vad};
use silero_vad_rust::silero_vad::utils_vad::VadIteratorParams;
#[cfg(unix)]

use tokio::sync::broadcast;
use tokio::signal::unix::{signal, SignalKind};
use std::sync::Arc;
use serde_json::json;

use crate::enums::SensorMessage;


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let (sensor_tx, mut db_rx) = broadcast::channel::<SensorMessage>(32);
    
    match db::create_dbs() {
        Ok(_) => {println!("Databases created successfully.")}
        Err(e) => {
            eprintln!("Failed to create databases: {}", e);
            return Err(e.into());
        }
    }

    tokio::spawn(async move {
        serial_speaker::speak("/dev/ttyACM0", sensor_tx).await;
    });

    tokio::spawn(async move {
        while let Ok(msg) = db_rx.recv().await {
            db::store_data(msg).await;
        }
    });

    let (record_tx, mut record_rx) = tokio::sync::mpsc::channel::<Arc<[i16]>>(100);
    let _record_stream = audio_speaker::speak(record_tx).await;

    let model = load_silero_vad()?;
    let params = VadIteratorParams {threshold: 0.2, min_silence_duration_ms: 500, ..Default::default()};
    let mut vad_iterator = VadIterator::new(model, params).expect("Failed to create iterator");
    
    let (speak_tx, mut speak_rx) = tokio::sync::mpsc::channel::<String>(100);
    tokio::spawn(async move {
        let mut speach_buffer = Vec::<i16>::new();
        let mut is_recording = false;
        while let Some(pcm_chunk) = record_rx.recv().await {
            audio_listener::listen(
                &mut vad_iterator, 
                &pcm_chunk,
                &mut speach_buffer, 
                &mut is_recording,
                speak_tx.clone(),  
                ).await;
        }
    });

    tokio::spawn(async move {
        while let Some(msg) = speak_rx.recv().await {
            if msg.is_empty() {continue};

            let payload = vec![
                json!({
                    "role": "system",
                    "content": "You are S.T.A.R.K., an autonomous AI smart home daemon running on a Raspberry Pi. Keep your answers extremely concise, conversational, and designed to be spoken out loud. Never use markdown formatting.",
                }),
                json!({
                    "role": "user",
                    "content": msg.trim(),
                })
            ];

            match llm::ask_llm(&payload).await {
                Ok(response) => println!("{}", response),
                Err(err) => eprintln!("{}", err),
            }
        }
    });


    println!("S.T.A.R.K. daemon is running. Press Ctrl+C to exit.");
    shutdown_signal().await;
    
    println!("\nShutting down S.T.A.R.K....");
    drop(_record_stream);

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal(SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    tokio::select! {
        _ = ctrl_c => {
            println!("\nReceived Ctrl+C (SIGINT)");
        },
        _ = terminate => {
            println!("\nReceived Systemd Stop (SIGTERM)");
        },
    }
}