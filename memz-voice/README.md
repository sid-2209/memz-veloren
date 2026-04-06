# memz-voice

`memz-voice` is the standalone spoken dialogue pipeline used by the wider MEMZ project.

It owns:

- microphone capture and device selection
- voice activity detection
- speech-to-text
- short-form NPC dialogue generation
- text-to-speech synthesis
- per-NPC conversation history and voice profiles

The Veloren-facing adapter lives in `memz-veloren::VoiceSystem`.

## Current Architecture

```text
microphone -> STT -> Ollama dialogue -> TTS -> audio buffer
```

More specifically:

- STT uses local Whisper by default and can optionally prefer an HTTP STT backend
- dialogue generation uses Ollama over HTTP
- TTS prefers Blitz TTS, then Kokoro, then an optional macOS fallback if a caller enables it, then a placeholder fallback

## Quick Start

From the repository root:

```bash
ollama serve
ollama pull llama3.2:1b
bash download_whisper.sh

cargo run -p memz-voice --example test_full --release
```

If you want real spoken TTS rather than placeholder audio, run a supported TTS server first:

```bash
python3 blitz_tts_server.py
```

or

```bash
python3 kokoro_server.py
```

## Examples

| Command | Purpose |
|---|---|
| `cargo run -p memz-voice --example list_audio_devices` | List available input devices |
| `cargo run -p memz-voice --example test_microphone --release` | Record and inspect microphone levels |
| `cargo run -p memz-voice --example test_stt --release` | STT only |
| `cargo run -p memz-voice --example test_llm --release` | Text-only NPC dialogue |
| `cargo run -p memz-voice --example test_tts --release` | TTS only |
| `cargo run -p memz-voice --example test_full --release` | End-to-end interactive loop |
| `cargo run -p memz-voice --example test_e2e_voice --release` | Extended end-to-end test |

## Important Environment Variables

| Variable | Effect |
|---|---|
| `MEMZ_VOICE_INPUT_DEVICE` | Force a specific microphone by device name |
| `MEMZ_MODELS_DIR` | Override where examples look for Whisper models |
| `MEMZ_STT_URL` | Use an external/local HTTP STT endpoint |
| `MEMZ_STT_VERIFY_MODEL` | Add a stronger local Whisper verifier model |
| `MEMZ_OLLAMA_MODEL` | Override the Ollama model used by the Veloren adapter |

## Notes on Scope

This crate already supports:

- NPC-specific voice profiles
- conversation history
- local or HTTP STT selection
- multiple TTS backends

It does not yet do full MEMZ memory retrieval on its own. Memory-aware behavior belongs to higher-level integration code in `memz-veloren`.

## Related Docs

- [Voice system overview](../docs/voice/README.md)
- [Voice testing guide](../docs/voice/testing.md)
- [Veloren integration guide](../docs/voice/veloren-integration.md)
