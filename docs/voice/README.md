# Voice NPC Guide

This section documents the current spoken NPC stack in the repo.

At a high level:

- `memz-voice` owns the reusable voice pipeline
- `memz-veloren::VoiceSystem` adapts that pipeline to Veloren's update loop
- `veloren/run_veloren.sh` is the practical entry point for running the in-tree client with the helper services

## Current Status

The voice path is working as a standalone pipeline and as an in-tree Veloren integration. It currently uses:

- local Whisper or optional HTTP STT
- Ollama for short NPC dialogue generation
- Blitz TTS as the preferred voice backend
- Kokoro as a neural TTS fallback
- short conversation history plus NPC metadata for context

It is not yet fully memory-aware in the MEMZ sense. The voice system is context-aware, but it does not yet retrieve and inject full `MemoryBank` state into every response.

## Quick Start

```bash
ollama serve
ollama pull llama3.2:1b
bash download_whisper.sh

# Standalone loop
cargo run -p memz-voice --example test_full --release

# Simulated in-game loop
cargo run -p memz-veloren --example test_voice_ingame --release
```

For the full client path:

```bash
cd veloren
./run_veloren.sh
```

## Key Files

- `memz-voice/src/lib.rs`: threaded pipeline orchestration
- `memz-voice/src/stt.rs`: microphone capture, device selection, Whisper/HTTP STT
- `memz-voice/src/llm.rs`: Ollama-backed NPC dialogue
- `memz-voice/src/tts.rs`: Blitz/Kokoro/fallback TTS
- `memz-veloren/src/voice_system.rs`: game-facing adapter
- `veloren/run_veloren.sh`: client launch helper
- `blitz_tts_server.py`: local neural TTS server
- `mlx_stt_server.py`: optional HTTP STT server

## Practical Guides

- [testing.md](testing.md)
- [troubleshooting.md](troubleshooting.md)
- [veloren-integration.md](veloren-integration.md)
