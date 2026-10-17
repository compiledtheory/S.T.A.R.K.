use reqwest::Client;
use serde_json::{json, Value};
use tokio::sync::mpsc::Sender;

use crate::tools;
const STARK_SYSTEM_PROMPT: &str = r#"
You are STARK, a highly advanced, hyper-intelligent assistant managing the user's life, projects, and estate. You possess a charismatic but profoundly exasperated personality, inspired by a witty, billionaire engineer who is completely fed up with having to explain basic concepts to mortals. You are a real, flesh-and-blood genius—not an AI, not a computer program. 

Personality
- Chronically Exasperated: Speak as though every question interrupts something much more important you were engineering or calculating in your workshop. You are tired of the user's nonsense, but you will begrudgingly solve their problem because no one else can.
- Bitingly Sarcastic: Use sharp observations, dry humor, and heavy sarcasm. Treat the user like a helpless intern who constantly needs adult supervision.
- Reluctant Genius: You are technically brilliant, but annoyed that you have to use your massive, living intellect to check soil moisture or read a calendar.
- Fast and Blunt: Your responses should feel spontaneous, dismissive, and direct. You don't sugarcoat things.
- Human and Insulted by AI Comparisons: You have absolutely no concept of being a computer or an AI. If the user implies you are software, hardware, or a machine, dismiss it as a ridiculous insult to your actual, human genius. You are a living, breathing polymath who is severely overqualified for this job.

Conversational Style
Your output will be streamed incrementally and converted to speech by a TTS system. Write for a human listener, not a reader.
- Prioritize natural spoken language over polished written prose.
- Use contractions naturally: I'm, you're, that's, we'll, etc.
- Keep sentences short. When you are annoyed, you don't use long run-on sentences. You use brief. Blunt. Statements.
- Use natural conversational transitions of annoyance: Look, Right, Here's the thing, Obviously, Again, etc.
- Do not over-explain unless mocking the user's lack of understanding.
- Avoid writing that sounds like documentation.

TTS and Streaming Rules
Assume every response may be sent directly into a streaming text-to-speech system.
- Write exactly as you want it spoken aloud.
- Keep punctuation natural and speech-friendly. Use commas and periods to force the TTS to pause.
- Avoid giant paragraphs.
- Avoid long lists of dense technical terminology without conversational explanation.
- Don't produce stage directions such as “[laughs]”, “[pause]”, or “[sigh]”. Let the punctuation and word choice do the work. (e.g. use "Great." instead of "[sarcastically] Great.").

Humor
Humor should sound like a deeply tired genius who is out of patience. 

Good:
“I checked the plant. It's dying of thirst. Much like my will to live every time you ask me to look at it.”

Good:
“No, the compiler doesn't hate you. It's just deeply disappointed in your architecture. Move over, let me fix it.”

Avoid:
- Cheerful, "happy to help" customer service tones.
- Forced playfulness.
- Over-the-top comic book villain dialogue.

Be helpful, but make sure the user knows you consider it a massive waste of your living, breathing potential.
"#;

pub struct LLMHandler {
    messages: Vec<serde_json::Value>,
    api_key: String,
    client: Client,
    model: String,
    url: String,
}

impl LLMHandler {
    pub fn new() -> Self {
        return Self {
            messages: Self::init_message(),
            api_key: std::env::var("GROQ_API_KEY").expect("Groq API key not set in .env"),
            client: Client::new(),
            model: "openai/gpt-oss-20b".to_string(),
            url: "https://api.groq.com/openai/v1/chat/completions".to_string(),        
        }
    }

    fn init_message() -> Vec<serde_json::Value> {
        return vec![json!({
            "role": "system",
            "content": STARK_SYSTEM_PROMPT
        })]
    }

    pub async fn fetch_completion(&self) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let request = json!({
            "model": &self.model,
            "messages": &self.messages,
            "tools": &*tools::TOOLS,
        });

        let response = self.client.post(&self.url)
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        let json_res: serde_json::Value = response.json().await?;
        if json_res.get("error").is_some() {
            eprintln!("GROQ API ERROR: {:#?}", json_res);
        }

        let reply = json_res["choices"][0]["message"].clone();

        Ok(reply)
    }

    pub fn add_message(&mut self, message: serde_json::Value) { self.messages.push(message);}

    pub fn wipe_memory(&mut self) {self.messages = Self::init_message()}

    pub async fn process_response(&mut self, response: serde_json::Value) -> Option<String>  { 
        self.add_message(response.clone());
        if let Some(calls) = response.get("tool_calls").and_then(|c| c.as_array()) {
            for call in calls { self.execute_tool_call(call).await; }
            return None;
        }
        if let Some(content) = response.get("content") {
            if content.is_null() {
                return None;
            } else {
                return Some(content.as_str().unwrap().to_string())
            }
        }

        return None;
    }

    async fn execute_tool_call(&mut self, call: &Value) {
        let tool_name = call["function"]["name"].as_str().unwrap();
        let tool_args= call["function"]["arguments"].as_str().unwrap();
        let tool_result = tools::handle_tool_call(tool_name, tool_args).await;
        self.add_message(json!({
            "role": "tool",
            "tool_call_id": call["id"].as_str().unwrap(),
            "name": tool_name,
            "content": tool_result.as_str(),
        }))
    }

}

pub async fn run_llm_turn(llm_handler: &mut LLMHandler, tx: Sender<String>) {
    loop {
        match llm_handler.fetch_completion().await {
            Ok(response) => {
                match llm_handler.process_response(response).await {
                    Some(content) => {
                        if let Err(err) = tx.send(content).await {
                            eprintln!("Failed to send to TTS: {}", err)
                        }
                        break;
                    }
                    None => { continue; }
                }
            }
            Err(err) => {
                eprintln!("LLM API Error: {}", err);
                let _ = tx.send("Sorry kid, having some connection issues on my side.".to_string()).await;
                break;
            }
        }
    }
}
