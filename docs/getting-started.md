# Getting Started with MEMZ

Use this guide for the current repository as it exists today, not the earlier phase-by-phase implementation notes.

## What You Can Run Today

- The core MEMZ workspace: `memz-core`, `memz-llm`, `memz-veloren`, `memz-bench`, and `memz-voice`
- Standalone voice examples from `memz-voice/examples`
- A simulated in-game voice loop from `memz-veloren/examples/test_voice_ingame.rs`
- The vendored Veloren client through `veloren/run_veloren.sh`

## Prerequisites

- A recent Rust toolchain with edition 2024 support
- Ollama for the voice dialogue path
- `models/whisper-tiny.en.bin` for local STT
- Optional Python 3.10+ with `blitz_tts` and/or `mlx_whisper` if you want the helper servers used by the Veloren launcher

## Clone and Build

```bash
git clone https://github.com/sid-2209/memz-veloren.git
cd memz-veloren

# Build the Rust workspace
cargo build --workspace

# Run workspace tests
cargo test --workspace
```

The vendored `veloren/` tree is not part of the top-level Cargo workspace. It has its own build flow and helper launcher.

## Core Memory Workflow

If you are starting from the memory system rather than the voice stack:

```bash
# Focus on the core crate
cargo test -p memz-core

# Inspect configuration defaults
sed -n '1,220p' memz.toml
```

Then read:

1. [architecture.md](architecture.md)
2. [veloren-rtsim-hooks.md](veloren-rtsim-hooks.md)
3. [spec/project-memz.md](spec/project-memz.md)

## Voice Workflow

### 1. Start Ollama

```bash
ollama serve
ollama pull llama3.2:1b
```

### 2. Download the Whisper model

```bash
bash download_whisper.sh
```

This places `whisper-tiny.en.bin` under `models/`, which is where the voice examples and `memz-veloren::VoiceSystem` look by default.

### 3. Run the standalone voice loop

```bash
cargo run -p memz-voice --example test_full --release
```

Useful focused tests:

```bash
cargo run -p memz-voice --example list_audio_devices
cargo run -p memz-voice --example test_microphone --release
cargo run -p memz-voice --example test_stt --release
cargo run -p memz-voice --example test_llm --release
cargo run -p memz-voice --example test_tts --release
```

### 4. Run the simulated in-game loop

```bash
cargo run -p memz-veloren --example test_voice_ingame --release
```

### 5. Run the vendored Veloren client

```bash
cd veloren
./run_veloren.sh
```

The launcher auto-configures assets, can auto-build the client, and can start Ollama plus the optional STT/TTS helper servers when your local environment supports them.

## Where to Go Next

- [README.md](../README.md): high-level project overview
- [README.md](README.md): documentation index
- [architecture.md](architecture.md): current crate responsibilities and data flow
- [voice/README.md](voice/README.md): voice system overview
- [voice/testing.md](voice/testing.md): practical test commands
- [voice/veloren-integration.md](voice/veloren-integration.md): full client run path
