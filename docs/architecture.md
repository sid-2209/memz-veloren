# MEMZ Architecture

> Deep dive into the memory system architecture.

## Crate Structure

```
memz/
├── memz-core/       # Game-agnostic memory library
│   ├── config.rs        — Full configuration hierarchy (MemzConfig → 11 sub-configs)
│   ├── memory/          — 7 memory types + MemoryBank aggregate
│   │   ├── episodic.rs      — "What happened" (Tulving, 1972)
│   │   ├── semantic.rs      — "What I know" (Tulving, 1985)
│   │   ├── emotional.rs     — "How I feel" (PAD model, Russell & Mehrabian, 1977)
│   │   ├── social.rs        — "What I've heard" (Dunbar, 1996)
│   │   ├── reflective.rs    — "What I think" (Flavell, 1979)
│   │   ├── procedural.rs    — "What I know how to do" (Anderson ACT-R, 1993)
│   │   └── injected.rs      — "My backstory" (player-authored)
│   ├── retrieval/       — Scoring & ranking engine
│   ├── embedding.rs     — Vector embedding trait + stubs
│   ├── decay.rs         — Ebbinghaus forgetting curve
│   ├── eviction.rs      — Hot/Warm/Cold/Archive ring eviction
│   ├── consolidation.rs — Episodic→Semantic, Episodic→Procedural transforms
│   ├── reflection.rs    — Rule-based + LLM reflection triggers
│   ├── social.rs        — Bayesian gossip propagation
│   ├── safety.rs        — Content filtering, rate limiting
│   ├── persistence.rs   — SQLite storage with CRC-32 checksums
│   ├── error.rs         — Error types
│   └── types.rs         — Core types (EntityId, MemoryId, Embedding, etc.)
│
├── memz-llm/        # LLM abstraction layer
│   ├── client.rs        — Ollama/OpenAI client
│   ├── prompt.rs        — Template rendering engine
│   ├── types.rs         — LLM request/response types
│   ├── error.rs         — LLM errors
│   └── prompts/         — Versioned prompt templates + GBNF grammars
│       ├── v1/              — 7 prompt templates (TOML)
│       └── grammars/        — 4 GBNF grammars
│
├── memz-veloren/    # Veloren game integration
│   ├── components.rs    — ECS components (MemoryComponent, markers)
│   ├── events.rs        — GameEvent enum (9 variants)
│   ├── systems.rs       — observe_event, run_decay, enforce_limits
│   └── hooks.rs         — on_dialogue, on_combat, on_trade, etc.
│
├── memz-bench/      # Criterion benchmarks
│   └── benches/memory_system.rs
│
└── docs/            # Documentation
    ├── architecture.md        — This file
    ├── veloren-rtsim-hooks.md — Veloren integration map
    └── getting-started.md     — Quick start guide
```

## Memory Lifecycle

```
Game Event (combat, dialogue, trade, death, theft, observation)
    │
    ▼
Event Observation (memz-veloren/events.rs)
    │
    ▼
Memory Encoding (episodic, with emotional valence & importance)
    │
    ▼
Memory Storage (MemoryBank — in-memory Vec per type)
    │
    ├─→ Decay (Ebbinghaus curve, every ~60 ticks)
    ├─→ Eviction (Hot→Warm→Cold→Archive, by age & score)
    ├─→ Consolidation (Episodic→Semantic, Episodic→Procedural, async)
    ├─→ Reflection (LLM Tier 2, async, creates ReflectiveMemory)
    └─→ Gossip (Social propagation, trust-weighted, Bayesian belief update)
         │
         ▼
Retrieval (top-K scoring: recency × relevance × importance × emotion × social)
    │
    ▼
Behavior Output (dialogue generation, price adjustment, relationship change)
```

## Retrieval Algorithm

Score = w₁·Recency + w₂·Relevance + w₃·Importance + w₄·Emotional + w₅·Social

| Factor | Formula | Default Weight |
|--------|---------|---------------|
| Recency | e^(-λ · ΔT) | 0.20 |
| Relevance | cosine_similarity(query_embed, memory_embed) | 0.30 |
| Importance | pre-computed 0-1 score | 0.20 |
| Emotional | |valence| × volatility | 0.20 |
| Social | trust_in_source × recency_of_transmission | 0.10 |

## Performance Budget

| Operation | Budget | Frequency |
|-----------|--------|-----------|
| Event observation + memory creation | < 10 μs | Per event |
| Memory decay pass (50 NPCs) | < 50 μs | Every 60 frames |
| Memory retrieval (top-5 from 200) | < 500 μs | On interaction |
| Serialization (100 memories) | < 2 ms | On save |
| Full frame budget (20 active NPCs) | < 2 ms | Every frame |

## Graceful Degradation

```
LLM + Embeddings Available
    → Full experience: contextual dialogue, reflections, semantic search
LLM Unavailable
    → Template-based dialogue referencing stored memories
Embeddings Unavailable
    → Keyword-match retrieval + templates
Everything Offline
    → Static personality-based responses (game still fully playable)
```
