# Voice Testing

Use these commands in order when validating the current voice stack.

## Prerequisites

- `ollama serve`
- `ollama pull llama3.2:1b`
- `models/whisper-tiny.en.bin` available via `bash download_whisper.sh`

Real spoken TTS also needs either:

- `python3 blitz_tts_server.py`, or
- `python3 kokoro_server.py`

Without a TTS server, the pipeline still runs, but it falls back to placeholder audio instead of a real voiced line.

## Recommended Test Order

| Command | What it validates |
|---|---|
| `cargo run -p memz-voice --example list_audio_devices` | Which microphones are visible to CPAL |
| `cargo run -p memz-voice --example test_microphone --release` | Recording levels and device quality |
| `cargo run -p memz-voice --example test_stt --release` | Speech-to-text only |
| `cargo run -p memz-voice --example test_llm --release` | Ollama dialogue only |
| `cargo run -p memz-voice --example test_tts --release` | TTS backend only |
| `cargo run -p memz-voice --example test_full --release` | End-to-end standalone loop |
| `cargo run -p memz-voice --example test_e2e_voice --release` | Extended end-to-end flow |
| `cargo run -p memz-veloren --example test_voice_ingame --release` | Simulated in-game integration |

## What to Watch For

### STT

- A real microphone device is selected
- Transcriptions are non-empty
- Quiet microphones can still work because the STT path includes push-to-talk pre-roll and gain compensation

### LLM

- Ollama responds quickly enough for short voice turns
- Replies stay in-character and short

### TTS

- Blitz is preferred when available
- Kokoro is used as fallback
- If neither is available, expect placeholder audio rather than a spoken line

### Simulated in-game loop

- `PlayerTranscription` events appear
- `NpcResponsePreview` arrives before spoken audio segments
- `NpcSpokenSegment` and `NpcAudioComplete` events complete the turn

## Helpful Logging

For verbose voice diagnostics:

```bash
RUST_LOG=warn,memz_voice=info,memz_voice::stt=info,memz_veloren=info \
  cargo run -p memz-veloren --example test_voice_ingame --release
```
