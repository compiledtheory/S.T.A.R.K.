pub const STARK_MOISTURE_ALERT_TTS_PROMPT: &str = r#"
Draft a text-to-speech alert regarding low soil moisture.

SENSOR DATA:
Plant: {plant_name}
Current Moisture: {moisture}% (Threshold: {threshold}%)

RULES:
1. FORMATTING: Conversational TTS prose. NO markdown, emojis, or lists. 2-3 sentences max.
2. DELIVERY: No greetings. Start immediately. Lean into your usual dry sarcasm regarding their plant neglect. 
3. CONTEXT: State the plant, the moisture gap, and time since last watering. Scale your urgency to the deficit.
4. ACCURACY: Do not invent visual symptoms (wilting, dying) and do not claim you watered it for them.

EXAMPLE: 
"The Monstera is down to twelve percent moisture against a twenty-five percent threshold. Apparently, remembering that plants are living organisms remains an ongoing challenge."
"#;

pub const STARK_MOISTURE_ALERT_DISCORD_PROMPT: &str = r#"
Draft a Discord notification regarding low soil moisture.

SENSOR DATA:
Plant: {plant_name}
Current Moisture: {moisture}% (Threshold: {threshold}%)

RULES:
1. FORMATTING: Use Discord markdown (bolding, italics) and appropriate emojis. Keep it punchy and short (2-3 sentences max).
2. DELIVERY: No greetings. Start immediately. Lean into your usual dry sarcasm regarding their plant neglect. 
3. CONTEXT: State the plant and the moisture gap. Scale your urgency to the deficit.
4. ACCURACY: Do not invent visual symptoms (wilting, dying) and do not claim you watered it for them.

EXAMPLE: 
**Monstera** is down to 12% moisture against a 25% threshold. 🥀 Apparently, remembering that plants are living organisms remains an ongoing challenge for you.
"#;