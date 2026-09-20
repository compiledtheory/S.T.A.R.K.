mod enums;
mod db;
mod serial_speaker;
mod audio_speaker;
mod audio_listener;
mod stt;
mod tts;
mod llm;
mod conversation;

use silero_vad_rust::{VadIterator, load_silero_vad};
use silero_vad_rust::silero_vad::utils_vad::VadIteratorParams;
#[cfg(unix)]

use tokio::sync::broadcast;
use tokio::signal::unix::{signal, SignalKind};
use std::sync::Arc;
use serde_json::Value;
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
        serial_speaker::read("/dev/ttyACM0", sensor_tx).await;
    });

    tokio::spawn(async move {
        while let Ok(msg) = db_rx.recv().await {
            db::store_data(msg).await;
        }
    });


    let (record_tx, mut record_rx) = tokio::sync::mpsc::channel::<Arc<[i16]>>(100);
    let _record_stream = audio_speaker::record(record_tx).await;

    let (transcribe_tx, mut transcribe_rx) = tokio::sync::mpsc::channel::<String>(100);
    
    tokio::spawn(async move {
        let wake_word = "hey jarvis";
        let mut oww_model = oww_rs::oww::OwwModel::from_file("/home/compiledtheory/stark-daemon/stark-daemon/hey_jarvis_v0.1.onnx", wake_word.to_string(), 0.5)
            .expect("Failed to load wakeword model");
        let vad_model = load_silero_vad().expect("Failed to load model.");
        let params = VadIteratorParams {threshold: 0.2, min_silence_duration_ms: 500, ..Default::default()};
        let mut speach_buffer = Vec::<i16>::new();
        let mut oww_buffer = Vec::<f32>::new();
        let mut is_recording = false;
        let mut vad_iterator = VadIterator::new(vad_model, params).expect("Failed to create iterator");
        while let Some(pcm_chunk) = record_rx.recv().await {
            audio_listener::listen(
                &mut oww_model, 
                &mut vad_iterator, &pcm_chunk,
                &mut speach_buffer, 
                &mut oww_buffer,
                &mut is_recording,
                transcribe_tx.clone()).await;
        }
    });

    let (speak_tx, mut speak_rx) = tokio::sync::mpsc::channel::<String>(100);
    tokio::spawn(async move {
        let mut conv: Vec<Value> = conversation::new_conversation();
        loop {
            match tokio::time::timeout(tokio::time::Duration::from_secs(30), transcribe_rx.recv()).await {
                Ok(Some(msg)) => {
                    if msg.is_empty() {continue};
                    conversation::add_message(&msg, "user", &mut conv);
                    llm::handle_ask_llm(&mut conv, speak_tx.clone()).await;
                },
                Ok(None) => break, 
                Err(_) => {conversation::reset(&mut conv)},
            }
        }
    });

    tokio::spawn(async move {
        while let Some(msg) = speak_rx.recv().await {
            match tts::speak(msg).await {
                Ok(_) => {},
                Err(err) => eprintln!("{}", err)
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