# S.T.A.R.K. Daemon
**A High-Performance, Event-Driven AI & Smart Home Engine**

S.T.A.R.K. is a decoupled, highly concurrent central nervous system for localized smart home automation. Built entirely in Rust and orchestrated by Tokio, it seamlessly bridges bi-directional AI voice interaction with a custom ESP-NOW sensor telemetry network, Google Calendar integration, and external webhooks.

All subsystems—from the microphone hardware buffer to the SQLite database, the LLM context window, and webhook routing—run asynchronously, passing messages through memory-safe channels synchronized by a central `watch` state machine.

---

## Architecture Overview

The system is designed around specialized, independent workers that communicate strictly through asynchronous message passing.

### The Message Bus
Data flows through highly specific channels, completely decoupling producers from consumers:

1. **`state_rx` / `state_tx`** (`tokio::sync::watch`)
   The global state machine. Broadcasts the current operational phase to all workers instantly, muting microphones, or triggering timeouts.
2. **`serial_tx`** (`tokio::sync::broadcast`)
   Fans out incoming ESP-NOW `SensorEvent` payloads (PIR motion, mmWave spatial coordinates, plant moisture) to the database and the automation event handler.
3. **`home_state_tx`** (`tokio::sync::mpsc`)
   Manages the global home context (e.g., active room presence, time-based conditions) so the LLM and automation engine are aware of the physical environment.
4. **`calendar_tx`** (`tokio::sync::mpsc`)
   Routes commands to the Google Calendar manager for fetching daily schedules or creating events asynchronously.
5. **`record_tx`** (`tokio::sync::mpsc`)
   Streams raw, 16-bit PCM audio chunks directly from the ALSA hardware buffer to the speech-to-text pipeline.
6. **`transcribe_tx`** (`tokio::sync::mpsc`) *(Internal to audio pipeline)*
   Passes fully transcribed user prompts from the STT engine to the LLM context manager.
7. **`agent_tx`** (`tokio::sync::mpsc`)
   Pushes the finalized contextual prompt (audio transcription + home state) into the LLM execution worker.
8. **`speak_tx`** (`tokio::sync::mpsc`)
   Streams raw text from the LLM directly into the local Piper TTS engine for sub-second audio generation.
9. **`discord_tx`** (`tokio::sync::mpsc`)
   Routes silent text payloads to the webhook worker for off-screen notifications (e.g., plant moisture alerts when the room is empty).

---

## The State Machine

At the core of the daemon is the `SystemState` enum.

* `Idle`: The baseline state. System quietly listens for the openWakeWord (ONNX) trigger and logs sensor data.
* `Listening`: Triggered by the wake word or the Hot Mic. Silero VAD actively captures user speech to a memory buffer.
* `Thinking`: Audio is dispatched to the Whisper STT API; the LLM processes the transcription and begins streaming a response to the selected output route.
* `Speaking`: The Piper TTS engine synthesizes audio. The microphone is automatically suppressed to prevent audio feedback.
* `AwaitingReply` *(Hot Mic)*: After every TTS response, the system automatically bypasses the wake word pipeline and waits for a follow-up.

**The Watchdog Task:** An isolated async task guarantees the system never gets stuck. If the system enters `AwaitingReply` and the user remains silent for 8 seconds, the watchdog gracefully resets the system to `Idle`.

---

## Core Subsystems

### Audio & Speech Pipeline
* **Wake Word & VAD:** Local, offline detection using `openWakeWord` and `Silero VAD`. Only clean, chunked speech is ever processed.
* **STT (Speech-to-Text):** Connects to Groq's high-speed Whisper Large v3 API for instant, highly accurate transcription.
* **TTS (Text-to-Speech):** Fully local voice synthesis using a Piper ONNX model. Piper runs as a managed subprocess, streaming raw bytes directly to the system's `aplay` buffer for near-zero latency playback.

### LLM Engine & Routing
* Tracks conversational context, maintaining a rolling memory window.
* Inactivity timeouts safely wipe memory after 60 seconds of silence, ensuring fresh context for new interactions.
* **Dynamic Output Routing:** The `LLMOutputRoute` enum dictates whether the LLM's response goes to the `speak_tx` channel for audio playback or the `discord_tx` channel for silent notifications based on real-time room presence.

### ESP-NOW Sensor Network & Automation
* **Serial Speaker:** Binds to `/dev/ttyACM0` to receive data from a central ESP32 receiver node.
* **Low Overhead:** Uses a custom byte protocol for maximum speed and minimal microcontroller overhead.
* **Decoupled Handlers:** Sensor events are broadcast to an SQLite `db_consumer` for historical tracking and an `event_handler` for real-time automation.
* **Context-Aware Modules:** Dedicated subsystems for specific domains like Google Calendar integration and plant monitoring.

---

## Project Structure

```text
src/
├── morning/             # Morning routine logic
│   ├── mod.rs           
│   ├── prompts.rs       
│   └── rundown.rs       
├── plants/              # Plant telemetry and moisture thresholds
│   ├── context.rs       
│   ├── mod.rs           
│   └── prompt.rs        
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