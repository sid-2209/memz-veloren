# MEMZ Architecture

This repository currently contains two closely related systems:

- The MEMZ memory stack: `memz-core`, `memz-llm`, `memz-veloren`, and `memz-bench`
- The voice NPC stack: `memz-voice`, `memz-veloren::voice_system`, and the in-tree `veloren/` integration

They live in the same workspace, but they are not at the same maturity level. The memory crates are the long-term product direction. The voice crates are the active integration layer for in-game spoken NPC dialogue.

## Workspace Map

```text
memz/
├── memz-core/      # Memory model, storage, retrieval, decay, safety, replay, reputation
├── memz-llm/       # Prompting, client abstractions, request queue
├── memz-veloren/   # Veloren adapter: bridge, memory rule, dialogue, voice system
├── memz-voice/     # Voice pipeline: STT, LLM dialogue, TTS, VAD, conversation history
├── memz-bench/     # Criterion benchmarks for performance-sensitive memory paths
├── veloren/        # Vendored Veloren tree used for live client integration work
└── docs/           # Curated project documentation
```

## Crate Responsibilities

### `memz-core`

Owns the game-agnostic memory model:

- `memory/` defines episodic, semantic, emotional, social, reflective, procedural, and injected memories
- `observation.rs`, `decay.rs`, `eviction.rs`, `consolidation.rs`, and `reflection.rs` drive memory lifecycle behavior
- `persistence.rs` handles durable storage
- `retrieval/` ranks memories for downstream behavior and dialogue
- `reputation.rs`, `replay.rs`, `conflict.rs`, `bard.rs`, and `first_five.rs` capture higher-level world features built on top of memory data

### `memz-llm`

Provides the shared LLM layer for memory-aware systems:

- `client.rs` wraps provider access
- `prompt.rs` renders prompts and templates
- `queue.rs` coordinates asynchronous request flow
- `types.rs` and `error.rs` define the transport surface

### `memz-veloren`

Bridges MEMZ and Veloren:

- `bridge.rs` maps Veloren concepts into MEMZ types
- `events.rs`, `hooks.rs`, and `memory_rule.rs` convert game activity into memory updates
- `dialogue.rs` assembles context for responses
- `voice_system.rs` adapts the standalone voice pipeline to Veloren's frame/update model

### `memz-voice`

Owns the real-time spoken dialogue pipeline:

- `stt.rs` selects the microphone, captures audio, applies VAD, and transcribes speech with local Whisper or an optional HTTP STT backend
- `llm.rs` builds NPC prompts and talks to Ollama
- `tts.rs` synthesizes speech through Blitz TTS, Kokoro, optional macOS fallback, or a placeholder fallback
- `conversation.rs` keeps short per-NPC dialogue history
- `voice_profile.rs` maps NPC traits and professions to voice parameters
- `lib.rs` ties the stages together with a threaded `VoicePipeline`

## Memory Flow

```text
Veloren event
  -> memz-veloren::events / hooks / memory_rule
  -> memz-core::MemoryBank update
  -> decay / eviction / consolidation / reflection / gossip
  -> retrieval and dialogue context
  -> player-facing behavior
```

The memory system is designed to degrade gracefully when embeddings or LLM features are unavailable.

## Voice Flow

```text
Push-to-talk input
  -> memz-veloren::VoiceSystem
  -> memz-voice::VoicePipeline
  -> SpeechToText (local Whisper or HTTP STT)
  -> DialogueLlm (Ollama)
  -> TextToSpeech (Blitz / Kokoro / fallback)
  -> VoiceGameEvent stream back to the game
```

This voice stack is currently context-aware through NPC metadata and short conversation history. It is not yet fully memory-aware in the MEMZ sense; that integration is future work.

## Integration Surface in `veloren/`

The vendored Veloren tree contains the current application-side hooks for voice work, primarily in:

- `veloren/voxygen/src/session/mod.rs`
- `veloren/voxygen/src/hud/mod.rs`
- `veloren/voxygen/src/audio/mod.rs`
- `veloren/voxygen/src/audio/channel.rs`
- `veloren/run_veloren.sh`

For the practical run flow, see [voice/veloren-integration.md](voice/veloren-integration.md).
