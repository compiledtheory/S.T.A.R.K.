mod sensors;
mod db;
mod serial_speaker;
mod audio_service;
mod stt;
mod tts;
mod llm;
mod tools;
mod audio_listener;

use tokio::sync::broadcast;
#[cfg(unix)]
use tokio::signal::unix::{signal, SignalKind};

use std::sync::Arc;
use serde_json::json;
use crate::{llm::LLMHandler, sensors::SensorMessage};
use crate::audio_listener::Listener;
use std::sync::atomic::{AtomicBool, Ordering};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let is_speaking = Arc::new(AtomicBool::new(false));
    let listener_speaking_flag = Arc::clone(&is_speaking);
    let tts_speaking_flag = Arc::clone(&is_speaking);
    let llm_speaking_flag = Arc::clone(&is_speaking);

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
            let _ = db::store_data(msg).await;
        }
    });


    let (record_tx, mut record_rx) = tokio::sync::mpsc::channel::<Arc<[i16]>>(100);
    let _record_stream = audio_service::record("TAM8", record_tx).await;

    let (transcribe_tx, mut transcribe_rx) = tokio::sync::mpsc::channel::<String>(100);
    let stt_handler = Arc::new(stt::STTHandler::new());
    let rt_handle = tokio::runtime::Handle::current();

    std::thread::spawn(move || {

        let mut listener = Listener::new(
            "/home/compiledtheory/daemon/stark.onnx",
            "hey jarvis",
            0.5
        );

        while let Some(pcm_chunk) = record_rx.blocking_recv() {
            if listener_speaking_flag.load(Ordering::SeqCst) {
                listener.clear_buffers();
                listener.stop_recording();
                continue; // Skip processing this audio chunk
            }
            
            listener.manage_silence(audio_service::check_silence(&pcm_chunk));
            
            if listener.should_listen() {

                if let Some(audio) = audio_service::listen(&mut listener, &pcm_chunk) {
                    let stt = Arc::clone(&stt_handler);
                    let tx = transcribe_tx.clone();

                    rt_handle.spawn(async move {
                    match stt.speech_to_text(&audio).await {
                            Ok(text) => {
                                println!("User: {}", text);
                                let _ = tx.send(text).await;
                            }
                            Err(err) => eprintln!("STT Error: {}", err),
                        }
                    });
                };
            } else { listener.clear_buffers(); }
            }
    });

    let (speak_tx, speak_rx) = tokio::sync::mpsc::channel::<String>(100);
    tokio::spawn(async move {
        let mut llm_handler = LLMHandler::new();
        
        loop {
            match tokio::time::timeout(tokio::time::Duration::from_secs(60), transcribe_rx.recv()).await {
                Ok(Some(msg)) => {
                    if msg.trim().is_empty() {continue};
                    llm_speaking_flag.store(true, Ordering::SeqCst);
                    
                    llm_handler.add_message(json!({
                        "role": "user",
                        "content": &msg
                    }));

                    llm::run_llm_turn(&mut llm_handler, speak_tx.clone()).await;
                    
                    while transcribe_rx.try_recv().is_ok() {}
                },
                Ok(None) => break, 
                Err(_) => {llm_handler.wipe_memory();},
            }
        }
    });

    tts::tts_worker(speak_rx, tts_speaking_flag);

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