# S.T.A.R.K. Daemon
**A High-Performance, Event-Driven AI & Smart Home Engine**

S.T.A.R.K. is a decoupled, highly concurrent central nervous system for localized smart home automation. Built entirely in Rust and orchestrated by Tokio, it seamlessly bridges bi-directional AI voice interaction with a custom ESP-NOW sensor telemetry network.

All subsystems---from the microphone hardware buffer to the SQLite database and the LLM context window---run asynchronously, passing messages through memory-safe channels synchronized by a central `watch` state machine.

---

## Architecture Overview

The system is designed around specialized, independent workers that communicate strictly through asynchronous message passing.

### The Message Bus
Data flows through five primary channels, decoupling producers from consumers:

1. **`state_rx` / `state_tx`** (`tokio::sync::watch`)
   The global state machine. Broadcasts the current operational phase to all workers instantly, muting microphones, or triggering timeouts.
2. **`serial_tx`** (`tokio::sync::broadcast`)
   Fans out incoming ESP-NOW `SensorEvent` payloads (PIR motion, plant moisture, telemetry) to the `db_consumer` and the `event_handler`.
3. **`record_tx`** (`tokio::sync::mpsc`)
   Streams raw, 16-bit PCM audio chunks directly from the ALSA hardware buffer to the speech-to-text pipeline.
4. **`transcribe_tx`** (`tokio::sync::mpsc`)
   Passes fully transcribed user prompts from the STT engine to the LLM context manager.
5. **`speak_tx`** (`tokio::sync::mpsc`)
   Streams raw text from the LLM directly into the local Piper TTS engine for sub-second audio generation.

---

## The State Machine

At the core of the daemon is the `SystemState` enum.

* `Idle`: The baseline state. System quietly listens for the openWakeWord (ONNX) trigger and logs sensor data.
* `Listening`: Triggered by the wake word or the Hot Mic. Silero VAD actively captures user speech to a memory buffer.
* `Thinking`: Audio is dispatched to the Whisper STT API; the LLM processes the transcription and begins streaming a response.
* `Speaking`: The Piper TTS engine synthesizes audio. The microphone is automatically suppressed to prevent audio feedback.
* `AwaitingReply` *(Hot Mic)*: After every TTS response, the system automatically bypasses the wake word pipeline and waits for a follow-up.

**The Watchdog Task:** An isolated async task guarantees the system never gets stuck. If the system enters `AwaitingReply` and the user remains silent for 8 seconds, the watchdog gracefully resets the system to `Idle`.

---

## Core Subsystems

### Audio & Speech Pipeline
* **Wake Word & VAD:** Local, offline detection using `openWakeWord` and `Silero VAD`. Only clean, chunked speech is ever processed.
* **STT (Speech-to-Text):** Connects to Groq's high-speed Whisper Large v3 API for instant, highly accurate transcription.
* **TTS (Text-to-Speech):** Fully local voice synthesis using a Piper ONNX model. Piper runs as a managed subprocess, streaming raw bytes directly to the system's `aplay` buffer for near-zero latency playback.

### LLM Engine
* Tracks conversational context, maintaining a rolling memory window.
* Inactivity timeouts safely wipe memory after 60 seconds of silence, ensuring fresh context for new interactions.

### ESP-NOW Sensor Network
* **Serial Speaker:** Binds to `/dev/ttyACM0` to receive data from a central ESP32 receiver node.
* **Low Overhead:** Uses a custom byte protocol for maximum speed and minimal microcontroller overhead.
* **Decoupled Handlers:** Sensor events are broadcast to an SQLite `db_consumer` for historical tracking and an `event_handler` for real-time automation.

---

## Project Structure

```text
src/
├── main.rs              # Dependency injection, channel instantiation, and task spawning
├── state.rs             # SystemState enum and the 8-second Watchdog timeout task
├── audio_service.rs     # cpal microphone interfacing and audio chunk routing
├── audio_listener.rs    # Wake word and Silero VAD state management buffers
├── stt.rs               # Groq Whisper API integration and STT worker loop
├── llm.rs               # LLM interaction, prompting, and conversation memory loop
├── tts.rs               # Subprocess management for Piper TTS and aplay
├── serial_service.rs    # Byte-parsing protocol for ESP-NOW radio frames
├── serial_speaker.rs    # TTY serial reader broadcasting events to the system
├── events.rs            # Automation rule engine responding to sensor states
├── db.rs                # Database connections and background telemetry writes
├── sensors.rs           # Core data structures (Node, PirEvent, PlantData, etc.)
└── tools.rs             # External tool bindings available to the LLM (WIP)