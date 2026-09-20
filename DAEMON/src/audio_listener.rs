use silero_vad_rust::{VadIterator, silero_vad::utils_vad::VadEvent};

use crate::stt;

pub async fn listen(
    oww_model: &mut oww_rs::oww::OwwModel,
    vad_iterator: &mut VadIterator,     
    chunk: &[i16],
    speach_buffer: &mut Vec<i16>,
    oww_buffer: &mut Vec<f32>,
    is_recording: &mut bool,
    tx: tokio::sync::mpsc::Sender<String>,
    ) {

    match *is_recording {
        false => {check_wake_word(oww_model, chunk, speach_buffer, oww_buffer, is_recording).await},
        true => {manage_recording(vad_iterator, chunk, speach_buffer, is_recording, tx).await},
    }
}

async fn check_wake_word(oww_model: &mut oww_rs::oww::OwwModel, chunk: &[i16], speach_buffer: &mut Vec<i16>, oww_buffer: &mut Vec<f32>, is_recording: &mut bool) {
    speach_buffer.extend(chunk);
    if speach_buffer.len() > 32000 {speach_buffer.drain(0..speach_buffer.len() - 32000);}
    
    let chunkf32: Vec<f32> = chunk.iter().map(|&s| s as f32).collect();
    oww_buffer.extend(chunkf32);

    while oww_buffer.len() >= 1280 {
        let current_chunk: Vec<f32> = oww_buffer.drain(0..1280).collect();
        let result = oww_model.detection(current_chunk);
        if result.detected {
            *is_recording = true;
            speach_buffer.extend(chunk);
            oww_buffer.clear();
            let silence = vec![0.0; 1280];
            for _ in 0..20 {
                let _ = oww_model.detection(silence.clone());
            }
            break;
        }
    }
}

async fn manage_recording(vad_iterator: &mut VadIterator, chunk: &[i16], speach_buffer: &mut Vec<i16>, is_recording: &mut bool, tx: tokio::sync::mpsc::Sender<String> ) {
    let chunkf32: Vec<f32> = chunk.iter().map(|&s| s as f32 / 32768.0).collect();
    match vad_iterator.process_chunk(&chunkf32, true, 1) {
        Ok(Some(VadEvent::Start(_ts))) => {speach_buffer.extend(chunk);},
        Ok(Some(VadEvent::End(_ts))) => {
            *is_recording = false;
            let audio = std::mem::take(speach_buffer);
            if audio.len() > 16000 {tokio::spawn(async move {send_recorded_speach(&audio, tx).await});}
        },
        Ok(None) => {if *is_recording {speach_buffer.extend(chunk);}},
        Err(err) => eprintln!("VAD Error: {}", err)
    }
}

async fn send_recorded_speach(audio: &Vec<i16>, tx: tokio::sync::mpsc::Sender<String>) {
    match stt::speach_to_text(audio).await {
        Ok(text) => {
            println!("User: {}", text);
            let _ = tx.send(text).await;},
        Err(err) => eprintln!("Something went wrong!: {}", err)
    }
}