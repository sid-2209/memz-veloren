# Veloren Voice Integration

This repository contains two ways to exercise the current Veloren voice path.

## 1. Simulated In-Game Test

Use this first if you want to validate the Rust integration without building the full client:

```bash
cargo run -p memz-veloren --example test_voice_ingame --release
```

This drives `memz-veloren::VoiceSystem` directly and prints the same event types the game-facing integration uses.

## 2. Full Client Path

The in-tree Veloren launcher is:

```bash
cd veloren
./run_veloren.sh
```

That script currently:

- sets `VELOREN_ASSETS` and `VELOREN_USERDATA`
- appends useful voice logging to `RUST_LOG`
- auto-builds `veloren-voxygen` unless `VELOREN_AUTO_BUILD=0`
- detects a better macOS input device when possible
- optionally starts MLX Whisper HTTP STT
- starts Blitz TTS when the wheel and Python module are available
- starts Ollama when needed

## Environment Variables

| Variable | Effect |
|---|---|
| `MEMZ_VOICE_INPUT_DEVICE` | Force a specific microphone |
| `MEMZ_OLLAMA_MODEL` | Override the Ollama model name |
| `MEMZ_STT_URL` | Use an explicit external/local HTTP STT endpoint |
| `MEMZ_ENABLE_HTTP_STT=1` | Ask the launcher to start local MLX STT |
| `MEMZ_STT_MODEL` | Select the MLX Whisper model for the HTTP STT server |
| `MEMZ_STT_VERIFY_MODEL` | Provide a stronger local verifier model for low-confidence transcripts |
| `MEMZ_PYTHON_BIN` | Preferred Python interpreter for helper servers |
| `MEMZ_STT_PORT` | HTTP STT port override |
| `BLITZ_TTS_PORT` | Blitz TTS port override |
| `VELOREN_AUTO_BUILD=0` | Skip auto-building the Veloren client |

## Code Locations

The current application-side integration touches these areas:

- `memz-veloren/src/voice_system.rs`
- `veloren/voxygen/src/session/mod.rs`
- `veloren/voxygen/src/hud/mod.rs`
- `veloren/voxygen/src/audio/mod.rs`
- `veloren/voxygen/src/audio/channel.rs`

The shell helper `integrate_voice_into_veloren.sh` now points here instead of older milestone notes.

## Recommended Run Flow

```bash
ollama serve
ollama pull llama3.2:1b
bash download_whisper.sh

# Optional for higher-quality TTS and HTTP STT
python3 blitz_tts_server.py

cd veloren
./run_veloren.sh
```

If you want the launcher to prefer the local MLX HTTP STT service:

```bash
cd veloren
MEMZ_ENABLE_HTTP_STT=1 ./run_veloren.sh
```
