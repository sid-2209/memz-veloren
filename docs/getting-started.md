# Getting Started with MEMZ

## Prerequisites

- **Rust** 1.80+ (tested with 1.93.0)
- **SQLite** (bundled via `rusqlite`, no system install needed)
- **Ollama** (optional, for LLM features) — [ollama.ai](https://ollama.ai)

## Quick Start

```bash
# Clone the repository
git clone https://github.com/your-org/memz.git
cd memz

# Build all crates
cargo build

# Run all tests (52+ tests)
cargo test

# Run benchmarks
cargo bench --bench memory_system
```

## Using memz-core as a Library

Add to your `Cargo.toml`:

```toml
[dependencies]
memz-core = { path = "../memz/memz-core" }
```

### Basic Usage

```rust
use memz_core::{MemoryBank, MemoryConfig, EntityId, MemoryId, GameTimestamp};
use memz_core::memory::episodic::EpisodicMemory;
use memz_core::types::Location;
use memz_core::persistence::PersistenceEngine;
use memz_core::config::PersistenceConfig;
use chrono::Utc;

// Create a memory bank for an NPC
let mut bank = MemoryBank::new();

// Add an episodic memory
bank.episodic.push(EpisodicMemory {
    id: MemoryId::new(),
    event: "A traveler bought a fine iron sword".to_string(),
    participants: vec![EntityId::new()],
    location: Location { x: 100.0, y: 50.0, z: 0.0 },
    timestamp: GameTimestamp { tick: 1000, real_time: Utc::now() },
    emotional_valence: 0.5,
    importance: 0.6,
    decay_rate: 0.02,
    strength: 1.0,
    access_count: 0,
    last_accessed: GameTimestamp { tick: 1000, real_time: Utc::now() },
    is_first_meeting: true,
    embedding: None,
});

// Persist to SQLite
let engine = PersistenceEngine::open("my_game.db", &PersistenceConfig::default())?;
let entity = EntityId::new();
engine.save_bank(&entity, &bank)?;

// Load later
let loaded = engine.load_bank(&entity)?.expect("bank exists");
assert_eq!(loaded.episodic.len(), 1);
```

## Configuration

Copy `memz.toml` to your project and customize:

```bash
cp memz.toml my_server/config/memz.toml
```

See the [example memz.toml](../memz.toml) for all available options.

## Running with Veloren

```bash
# The Veloren repo is cloned at veloren/
# See docs/veloren-rtsim-hooks.md for integration details
cd veloren
cargo run --bin veloren-server-cli
```

## Project Structure

| Crate | Purpose |
|-------|---------|
| `memz-core` | Game-agnostic memory library (types, storage, retrieval, decay) |
| `memz-llm` | LLM abstraction (Ollama, OpenAI, prompt templates) |
| `memz-veloren` | Veloren-specific integration (ECS components, hooks) |
| `memz-bench` | Criterion benchmarks |

## Next Steps

1. Read [Architecture](architecture.md) for the full system design
2. Read [Veloren rtsim Hooks](veloren-rtsim-hooks.md) for game integration points
3. Read [Project Memz.md](../Project%20Memz.md) for the complete design specification
