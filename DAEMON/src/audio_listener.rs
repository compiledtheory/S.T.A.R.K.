use silero_vad_rust::{VadIterator, silero_vad::utils_vad::VadEvent};

use crate::stt;

pub async fn listen(
    vad_iterator: &mut VadIterator,     
    chunk: &[i16],
    speach_buffer: &mut Vec<i16>,
    is_recording: &mut bool,
    tx: tokio::sync::mpsc::Sender<String>,
    ) {
    let chunk_f32: Vec<f32> = chunk
                    .iter()
                    .map(|&s| s as f32 / 32768.0)
                    .collect();

    match vad_iterator.process_chunk(
        &chunk_f32, 
        true, 
        1) {
            Ok(Some(VadEvent::Start(_ts))) => {
                *is_recording = true;
                speach_buffer.extend(chunk);
            },
            Ok(Some(VadEvent::End(_ts))) => {
                *is_recording = false;
                match stt::speach_to_text(speach_buffer).await {
                    Ok(text) => {let _ = tx.try_send(text);},
                    Err(err) => {eprintln!("Something went wrong!: {}", err)}
                }
                speach_buffer.clear();
            },
            Ok(None) => {
                if *is_recording {
                    speach_buffer.extend(chunk);
                }
            },
            Err(e) => eprintln!("VAD Error: {}", e)
        };
}