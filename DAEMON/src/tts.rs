use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

struct PiperProcess {
    _child: Child,
    stdin: ChildStdin,
    audio_rx: Receiver<Vec<u8>>,
    stderr_rx: Receiver<String>
}

impl PiperProcess {
    pub fn spawn(model_path: &str) -> Self {
        let mut child = Command::new("./piper/piper")
            .args(["--model", model_path, "--output_raw"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start Piper TTS");

        let stdin = child.stdin.take().expect("Failed to capture Piper stdin");
        let stdout = child.stdout.take().expect("Failed to capture Piper stdout");
        let stderr = child.stderr.take().expect("Failed to capture Piper stderr");
        
        let (audio_tx, audio_rx) = mpsc::channel::<Vec<u8>>();
        std::thread::spawn(move || {
            Self::stream_raw_chunks(audio_tx, stdout);
        });

        let (stderr_tx, stderr_rx) = mpsc::channel::<String>();
        std::thread::spawn(move || {
            Self::stream_error_diagnostics(stderr_tx, stderr);
        });

        return Self {
            _child: child,
            stdin,
            audio_rx,
            stderr_rx
        }

    }

    fn stream_raw_chunks(sender: Sender<Vec<u8>>, mut stdout: ChildStdout) {
        let mut buf = [0u8; 4096];
        while let Ok(n) = stdout.read(&mut buf) {
            if n == 0 { break; }

            let _ = sender.send(buf[..n].to_vec());
        }
    }

    fn stream_error_diagnostics(sender: Sender<String>, stderr: ChildStderr) {
        let mut reader = BufReader::new(stderr);
        let mut line = String::new();
        while let Ok(bytes) = reader.read_line(&mut line) {
            if bytes == 0 { break; }

            let _ = sender.send(line.clone());
            line.clear();
        }
    }

    pub fn drain_stale_data(&self) {
        while self.audio_rx.try_recv().is_ok() {}
        while self.stderr_rx.try_recv().is_ok() {}      
    }

    pub fn synthesize(&mut self, text: &str) -> Result<(), std::io::Error> {
        self.stdin.write_all(format!("{}\n", text).as_bytes())?;
        self.stdin.flush()
    }
}

pub fn tts_worker(mut rx: tokio::sync::mpsc::Receiver<String>, is_speaking: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let mut piper = PiperProcess::spawn("./en_US-ryan-medium.onnx");
        while let Some(raw_text) = rx.blocking_recv() {
            let clean = raw_text.replace(['\n', '\r'], " ").trim().to_string();
            if clean.is_empty() {
                if rx.is_empty() { is_speaking.store(false, Ordering::SeqCst); }
                continue;
            }

            is_speaking.store(true, Ordering::SeqCst);
            piper.drain_stale_data();

            if let Err(err) = piper.synthesize(&clean) {
                eprintln!("Failed to write text to Piper: {}", err);
                is_speaking.store(false, Ordering::SeqCst);
                break;
            }

            play_sentence(&piper.audio_rx, &piper.stderr_rx);
            
            if rx.is_empty() {
                is_speaking.store(false, Ordering::SeqCst);
            }
        }
    });
}

fn spawn_aplay() -> Child {
    return Command::new("aplay")
        .args([
            "-D",
            "plughw:Headphones,0",
            "-q",
            "-r",
            "22050",
            "-f",
            "S16_LE",
            "-t",
            "raw",
            "--buffer-size=8192",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start aplay")   
}

fn play_sentence(audio_rx: &Receiver<Vec<u8>>, stderr_rx: &Receiver<String>) {
    let mut aplay = spawn_aplay();
    let mut aplay_stdin = aplay.stdin.take().expect("Failed to capture aplay stdin");

    let mut finished_generation = false;
    let mut time_since_finish = Instant::now();
    let fail_safe_start = Instant::now();

    loop {
        if let Ok(chunk) = audio_rx.recv_timeout(Duration::from_millis(10)) {
            if aplay_stdin.write_all(&chunk).is_err() {
                break;
            }
        }

        if !finished_generation {
            while let Ok(line) = stderr_rx.try_recv() {
                if line.contains("Real-time factor") {
                    finished_generation = true;
                    time_since_finish = Instant::now();
                }
            }
        }

        if finished_generation && time_since_finish.elapsed() > Duration::from_millis(50) {
            while let Ok(chunk) = audio_rx.try_recv() {
                let _ = aplay_stdin.write_all(&chunk);
            }
            break;
        }

        if fail_safe_start.elapsed() > Duration::from_secs(30) {
            eprintln!("Warning: TTS playback hit safety timeout");
            break;
        }
    }

    drop(aplay_stdin);
    let _ = aplay.wait();
}