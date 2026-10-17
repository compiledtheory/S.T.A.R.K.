use oww_rs::oww::OwwModel;
use silero_vad_rust::{VadIterator, load_silero_vad, silero_vad::utils_vad::{VadEvent, VadIteratorParams}};

pub struct Listener {
    pub speach_buffer: Vec<i16>,
    pub oww_buffer: Vec<f32>,
    pub is_recording: bool,
    pub vad_iterator: VadIterator,
    pub silence_frames: u32,
    pub oww_model: OwwModel,
}

impl Listener {
    pub fn new(path: &str, wake_word: &str, threshold: f32) -> Self {
        let params = VadIteratorParams { 
            threshold: 0.2, 
            min_silence_duration_ms: 500, 
            ..Default::default()};

        let vad_model = load_silero_vad().expect("Failed to load model.");
        let vad_iterator = VadIterator::new(vad_model, params).expect(
                "Failed to create iterator"
            );

        let oww_model = OwwModel::from_file(path, wake_word.to_string(), threshold).expect(
                "Failed to load wakeword model"
            );

        return Self {
            speach_buffer: Vec::<i16>::new(),
            oww_buffer: Vec::<f32>::new(),
            is_recording: false,
            vad_iterator,
            silence_frames: 15,
            oww_model,
        }
    }

    pub fn clear_buffers(&mut self) {
        self.speach_buffer.clear();
        self.oww_buffer.clear();
    }

    pub fn manage_silence(&mut self, rms: f32) {
        if rms > 150.0 { self.reset_silence();} else {self.increment_silence();}
    }

    pub fn reset_silence(&mut self) { self.silence_frames = 0; }

    pub fn increment_silence(&mut self) { self.silence_frames += 1; }

    pub fn should_listen(&self) -> bool { return self.is_recording || self.silence_frames < 15 }

    pub fn start_recording(&mut self) { self.is_recording = true; }

    pub fn stop_recording(&mut self) { self.is_recording = false; }

    pub fn process_idle_chunk(&mut self, chunk: &[i16]) {
        self.buffer_audio(chunk);
        while self.oww_buffer.len() >= 1280 {
            self.detect_wake_word(chunk);
        }
    }

    fn buffer_audio(&mut self, chunk: &[i16]) { 
        self.speach_buffer.extend(chunk); 
        let buffer_len = self.speach_buffer.len();
        if buffer_len > 32000 {
            self.speach_buffer.drain(0..buffer_len - 32000); 
        }

        self.oww_buffer.extend(chunk.iter().map(|&s| s as f32));
    }

    fn detect_wake_word(&mut self, chunk: &[i16]) {
       let current_chunk: Vec<f32> = self.oww_buffer.drain(0..1280).collect();
       let result = self.oww_model.detection(current_chunk);
       if result.detected {
            self.start_recording();
            self.speach_buffer.extend(chunk);
            self.oww_buffer.clear();
            for _ in 0..20 {
                let _ = self.oww_model.detection(vec![0.0; 1280]);
            }
            return;
       }
    }

    pub fn process_vad_chunk(&mut self, chunk: &[i16]) -> Option<Vec<i16>> {

            let chunkf32: Vec<f32> = chunk.iter().map(|&s| s as f32 / 32768.0).collect();
            match self.vad_iterator.process_chunk(&chunkf32, true, 1) {
                Ok(Some(VadEvent::Start(_ts))) => { 
                    self.speach_buffer.extend(chunk); 
                    return None;
                },
                Ok(Some(VadEvent::End(_ts))) => {
                    self.stop_recording();
                    let audio = std::mem::take(&mut self.speach_buffer);
                    
                    if audio.len() > 16000 {
                        return Some(audio);
                    } else {
                        return None;
                    }
                },
                Ok(None) => { if self.is_recording { 
                    self.speach_buffer.extend(chunk); 
                    }
                    return None;
                },
                Err(err) => {
                    eprintln!("VAD Error: {}", err);
                    return None;
                }
            }
        }
}