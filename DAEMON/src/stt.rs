use hound::{WavSpec, WavWriter, SampleFormat};
use std::io::Cursor;

pub async fn speach_to_text(
    chunk: &[i16]
)-> Result<String, Box<dyn std::error::Error + Send + Sync>> {

    let mut cursor = Cursor::new(Vec::new());
    write_bytes(&mut cursor, chunk)?;

    let api_key = std::env::var("GROQ_API_KEY").expect("Groq API key not set in .env");
    let client = reqwest::Client::new();
    let part = reqwest::multipart::Part::bytes(cursor.into_inner())
                                    .file_name("command.wav")
                                    .mime_str("audio/wav")?;
    
    let form = reqwest::multipart::Form::new()
                                    .part("file", part)
                                    .text("model", "whisper-large-v3");

    let text = client.post("https://api.groq.com/openai/v1/audio/transcriptions")
                                    .bearer_auth(api_key)
                                    .multipart(form)
                                    .send()
                                    .await?;

    let json: serde_json::Value = text.json().await?;

    Ok(json["text"].as_str().unwrap_or("").to_string())

}

fn make_wavespec() -> WavSpec {
    return WavSpec { 
        channels: 1, 
        sample_rate: 16_000, 
        bits_per_sample: 16, 
        sample_format: SampleFormat::Int }
}

fn write_bytes(
    cursor: &mut Cursor<Vec<u8>>,
    chunk: &[i16],) -> Result<(), hound::Error> {
    
    let mut writer = WavWriter::new(cursor, make_wavespec())?;
    for &sample in chunk {
        writer.write_sample(sample)?;
    }

    writer.finalize()?;

    Ok(())
}