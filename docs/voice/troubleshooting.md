# Voice Troubleshooting

## Wrong Microphone or Silent Input

List devices first:

```bash
cargo run -p memz-voice --example list_audio_devices
```

Then force the microphone you want:

```bash
MEMZ_VOICE_INPUT_DEVICE="Your Microphone Name" \
  cargo run -p memz-voice --example test_microphone --release
```

The STT layer prefers the system default device unless it looks virtual, in which case it searches for a physical fallback.

## Whisper Model Not Found

Download the default model:

```bash
bash download_whisper.sh
```

The Veloren adapter searches these locations automatically:

- `models/`
- `../models/`
- `../../models/`
- executable-relative `models/`
- `$HOME/.local/share/veloren-memz/models`

You can improve low-confidence transcripts with a stronger verifier model by setting:

```bash
MEMZ_STT_VERIFY_MODEL=models/ggml-base.en.bin
```

## Ollama Is Not Running

```bash
ollama serve
ollama pull llama3.2:1b
```

To use a different model in the Veloren adapter:

```bash
MEMZ_OLLAMA_MODEL=llama3.2:3b
```

## No Real Spoken TTS

The default library config does not enable macOS `say` fallback. For real spoken output, run one of the local TTS servers:

```bash
python3 blitz_tts_server.py
```

or

```bash
python3 kokoro_server.py
```

If neither server is available, `memz-voice` falls back to placeholder audio so the pipeline still completes.

## HTTP STT Not Starting

The optional MLX STT path needs a Python environment with `mlx_whisper`.

Typical launch:

```bash
MEMZ_ENABLE_HTTP_STT=1 ./veloren/run_veloren.sh
```

Or force a specific interpreter:

```bash
MEMZ_PYTHON_BIN=/path/to/python3 MEMZ_ENABLE_HTTP_STT=1 ./veloren/run_veloren.sh
```

You can also point directly at an external STT endpoint:

```bash
MEMZ_STT_URL=http://127.0.0.1:8891 ./veloren/run_veloren.sh
```

## Veloren Launcher Issues

`veloren/run_veloren.sh` writes helper service logs to:

- `/tmp/ollama_veloren.log`
- `/tmp/mlx_stt_server.log`
- `/tmp/blitz_tts_server.log`

If the launcher skips a service, it usually means the dependency is missing in your current environment rather than the Rust code failing.
