use serde_json::json;

pub async fn ask_llm(messages: &[serde_json::Value]) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let api_key = std::env::var("GROQ_API_KEY").expect("Groq API key not set in .env");
    let client = reqwest::Client::new();

    let request = json!({
        "model": "openai/gpt-oss-20b",
        "messages": messages,
    });

    let response = client.post("https://api.groq.com/openai/v1/chat/completions")
        .bearer_auth(api_key)
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .await?;

    let json_res: serde_json::Value = response.json().await?;

    if json_res.get("error").is_some() {
        eprintln!("❌ GROQ API ERROR: {:#?}", json_res);
    }

    let reply = json_res["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("I encountered an error!")
        .to_string();

    Ok(reply)
}