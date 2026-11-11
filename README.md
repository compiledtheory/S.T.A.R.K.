# S.T.A.R.K. Daemon

S.T.A.R.K. is an event-driven smart home daemon built in Rust using Tokio. It handles local voice interaction, receives sensor data via a custom ESP-NOW network, integrates with Google Calendar, and serves a spatial tracking frontend.

All subsystems—from the audio hardware buffer to the SQLite database, LLM processing, spatial coordinate streaming, and webhook routing—run asynchronously and communicate through channels managed by a central state machine.

---

## Architecture Overview

The system uses isolated async workers that communicate strictly through Tokio channels, decoupling producers from consumers:

1. **`state_rx` / `state_tx`** (`tokio::sync::watch`)
   The global state machine. Broadcasts the operational phase to workers (e.g., muting microphones, triggering timeouts).
2. **`serial_tx`** (`tokio::sync::broadcast`)
   Routes incoming ESP-NOW `SensorEvent` payloads (PIR, mmWave, moisture) to the database, spatial monitor, and event handler.
3. **`home_state_tx`** (`tokio::sync::mpsc`)
   Tracks global physical context (room presence, time) for the LLM and automation rules.
4. **`frontend_tx`** (`tokio::sync::broadcast`)
   Broadcasts normalized spatial telemetry to the Axum server for Server-Sent Events (SSE) streaming.
5. **`calendar_tx`** (`tokio::sync::mpsc`)
   Handles Google Calendar API requests.
6. **`record_tx`** (`tokio::sync::mpsc`)
   Streams PCM audio chunks from the ALSA buffer to the STT worker.
7. **`transcribe_tx`** (`tokio::sync::mpsc`)
   Passes text transcriptions from the STT engine to the LLM worker.
8. **`agent_tx`** (`tokio::sync::mpsc`)
   Pushes the prompt (transcription + home state) to the LLM.
9. **`speak_tx`** (`tokio::sync::mpsc`)
   Sends LLM text to the Piper TTS engine for audio synthesis.
10. **`discord_tx`** (`tokio::sync::mpsc`)
    Sends webhook payloads for off-screen notifications.

---

## The State Machine

The daemon operates on a `SystemState` enum:

* `Idle`: Baseline state. Listens for the openWakeWord (ONNX) trigger and logs sensor data.
* `Listening`: Triggered by the wake word. Silero VAD captures user speech to a memory buffer.
* `Thinking`: Audio is dispatched to the Whisper API; the LLM processes the transcription and streams a response.
* `Speaking`: Piper TTS synthesizes audio. The microphone is suppressed to prevent feedback.
* `AwaitingReply`: After a TTS response, the system bypasses the wake word pipeline and waits for a follow-up.

**Watchdog Task:** If the system enters `AwaitingReply` and no speech is detected for 8 seconds, the watchdog resets the state to `Idle`.

---

## Core Subsystems

### Audio & Speech Pipeline
* **Wake Word & VAD:** Local detection using `openWakeWord` and `Silero VAD`.
* **STT (Speech-to-Text):** Uses the Groq Whisper Large v3 API.
* **TTS (Text-to-Speech):** Local voice synthesis using a Piper ONNX model, streaming directly to the `aplay` buffer.

### LLM Engine & Routing
* Maintains a rolling conversational context window.
* Wipes memory after 60 seconds of inactivity.
* **Output Routing:** Dictates whether the LLM's response is spoken via TTS or sent silently to Discord based on active room presence.

### ESP-NOW Sensor Network & Automation
* **Serial Speaker:** Reads data from `/dev/ttyACM0` connected to a central ESP32 receiver.
* Uses a custom byte protocol to minimize microcontroller overhead.
* Sensor events are logged to SQLite and evaluated by the event handler for automation rules.

### Spatial Tracking & Kiosk UI
* **Telemetry Normalization:** Converts raw LD2450 mmWave coordinates to a bounded percentage grid.
* **SSE Streaming:** An embedded Axum web server streams the normalized coordinates to a React kiosk UI.

---

## Project Structure

```text
src/
├── frontend/            # Axum web server and UI hosting
│   ├── mod.rs          
│   └── server.rs       
├── morning/             # Morning routine logic
│   ├── mod.rs          
│   ├── prompts.rs      
│   └── rundown.rs      
├── plants/              # Plant telemetry and moisture thresholds
│   ├── context.rs      
│   ├── mod.rs          
│   └── prompt.rs       
├── spatial/             # radar telemetry and normalization
│   ├── mod.rs          
│   ├── monitor.rs      
│   └── position.rs     
├── webhooks/            # External API notification routing
│   ├── discord.rs      
│   └── mod.rs          
├── audio_listener.rs    # Wake word and Silero VAD state buffers
├── audio_service.rs     # cpal microphone interfacing
├── calendar.rs          # Google Calendar API integration
├── db.rs                # Database connections and background writes
├── events.rs            # Automation rule engine based on sensor states
├── home_state.rs        # Global physical context tracker
├── llm.rs               # LLM interaction and conversational memory
├── main.rs              # Dependency injection and channel instantiation
├── sensors.rs           # Core data structures (Node, PirEvent, etc.)
├── serial_service.rs    # Byte-parsing protocol for ESP-NOW frames
├── serial_speaker.rs    # TTY serial reader broadcasting events
├── state.rs             # SystemState enum and watchdog timeouts
├── stt.rs               # Groq Whisper API integration
├── tools.rs             # External tool bindings for LLM function calling
└── tts.rs               # Piper TTS and aplay subprocess management