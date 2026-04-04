# memz-voice

Voice dialogue system for Veloren NPCs - Phase 1 (Simple voice I/O, no MEMZ integration)

## Features

- **Speech-to-Text (STT):** Whisper-tiny.en for transcribing player voice
- **Text-to-Speech (TTS):** macOS system TTS (temporary) or Kokoro TTS
- **LLM Dialogue:** Llama-3.2-1B for generating NPC responses
- **Simple Prompts:** Generic NPC personalities (no MEMZ context yet)

## Prerequisites

### System Requirements
- macOS (M4 or later recommended)
- Rust 1.93.0+
- 8GB RAM minimum
- Microphone

### Models Required

Download these models before running:

```bash
mkdir -p models
cd models

# Whisper-tiny (~50MB)
curl -L -o whisper-tiny.en.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin

# Llama-3.2-1B (~700MB)
curl -L -o llama-3.2-1b-q4.gguf \
  https://huggingface.co/bartowski/Llama-3.2-1B-Instruct-GGUF/resolve/main/Llama-3.2-1B-Instruct-Q4_K_M.gguf
```

## Building

```bash
cd memz-voice
cargo build --release
```

## Testing

### Test 1: Speech-to-Text
```bash
cargo run --example test_stt
```
Speak into your microphone for 5 seconds, see transcription.

### Test 2: Text-to-Speech
```bash
cargo run --example test_tts
```
Type text, hear synthesized voice.

### Test 3: LLM Dialogue
```bash
cargo run --example test_llm
```
Chat with an NPC via text.

### Test 4: Full Voice Dialogue
```bash
cargo run --example test_full
```
Complete voice loop: speak → NPC responds with voice.

## Usage

```rust
use memz_voice::VoiceDialogue;

// Initialize
let mut dialogue = VoiceDialogue::new(
    "models/whisper-tiny.en.bin",
    "models/Kokoro-82M",
    "models/llama-3.2-1b-q4.gguf",
)?;

// Start recording
dialogue.start_recording()?;

// ... player speaks ...

// Stop and get response
let (npc_text, npc_audio) = dialogue.stop_and_respond(
    "blacksmith",  // NPC profession
    "friendly",    // NPC mood
)?;

// Play audio
dialogue.play_audio(&npc_audio)?;
```

## Architecture

```
Player Voice → Whisper STT → Simple Prompt → Llama LLM → TTS → Audio Output
```

## Phase 1 Limitations

- ❌ No MEMZ memory integration
- ❌ No personality system (OCEAN traits)
- ❌ No sentiment tracking
- ❌ No context-aware responses
- ✅ Basic voice input/output works
- ✅ Generic NPC responses
- ✅ Spatial audio ready (for Veloren integration)

## Phase 2 (Future)

Phase 2 will add MEMZ integration:
- Episodic/semantic/emotional/social memories
- Personality-driven responses
- Sentiment tracking
- Context-aware greetings
- Gossip and social dynamics

## Troubleshooting

### "Model not found"
Download models as shown above.

### "No input device available"
Check microphone permissions:
- macOS: System Settings → Privacy & Security → Microphone

### "Failed to load Whisper model"
Ensure you downloaded the correct model file (ggml-tiny.en.bin).

### "LLM inference too slow"
- Ensure Metal GPU is enabled (should be automatic on M4)
- Try smaller model or reduce max_tokens

## License

Same as parent MEMZ project.
