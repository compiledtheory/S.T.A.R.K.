use tokio::process::Command;
use tokio::io::AsyncWriteExt;
use std::process::Stdio;


pub async fn speak(text: String) -> Result<(), Box<dyn std::error::Error>> {
    let mut piper = Command::new("./piper/piper")
        .arg("--model")
        .arg("./en_US-ryan-medium.onnx")
        .arg("--output-raw")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to start piper TTS");

    if let Some(mut stdin) = piper.stdin.take() {
        stdin.write_all(text.as_bytes()).await?;
        drop(stdin);
    }

    let out = piper.stdout.take().expect("Failed to capture output");
    let aplay_stdin: std::process::Stdio = out.try_into().expect("Failed to convert stdout to Stdio");

    let mut aplay = Command::new("aplay")
        .arg("-D").arg("plughw:Headphones,0")
        .arg("-r").arg("22050")
        .arg("-f").arg("S16_LE")
        .arg("-t").arg("raw")
        .arg("--buffer-size=32768")
        .arg("-")
        .stdin(aplay_stdin)
        .spawn()
        .expect("Failed to stream response.");

    aplay.wait().await?;
    piper.wait().await?;

    Ok(())
}