# Veloren rtsim Integration — Hook Points for MEMZ

> This document maps the integration points between MEMZ and Veloren's `rtsim`
> (Real-Time Simulation) system. It was generated from an analysis of the
> Veloren source code (commit at time of clone, Feb 2026).

---

## 1. Architecture Overview

Veloren's rtsim is an **event-based, rule-driven** simulation that runs the
entire game world in real-time. It is NOT an ECS — it's more like a relational
database with business logic on top.

### Core abstractions

| Abstraction | Location | Description |
|------------|----------|-------------|
| `RtState` | `rtsim/src/lib.rs` | Central state container. Holds `Data`, `Rules`, and `EventHandlers`. |
| `Event` | `rtsim/src/event.rs` | Trait for events. Each event type carries `SystemData`. |
| `Rule` | `rtsim/src/rule/mod.rs` | Trait for composable simulation rules. Rules bind event handlers. |
| `Data` | `rtsim/src/data/mod.rs` | World data: NPCs, sites, factions, reports, sentiments, quests, nature. |
| `Action` | `rtsim/src/ai/mod.rs` | Combinator-based AI actions (like coroutines). |
| `NpcCtx` | `rtsim/src/ai/mod.rs` | Context passed to NPC AI actions (access to npc, data, controller). |

### Default rules (started in order)

1. `Migrate` — save file migration
2. `Architect` — builds/manages sites
3. `ReplenishResources` — spawns resources
4. `ReportEvents` — creates `Report`s from game events
5. `SyncNpcs` — syncs rtsim NPCs with ECS entities
6. `SimulateNpcs` — runs brain ticks for non-loaded NPCs
7. `NpcAi` — the big one — governs all NPC behavior
8. `CleanUp` — garbage collection

---

## 2. Key Events (Hook Points)

These are the events MEMZ should bind handlers to:

| Event | File | Fields | MEMZ Use |
|-------|------|--------|----------|
| `OnTick` | `event.rs` | `time_of_day, tick, time, dt` | Decay pass, consolidation, eviction, gossip propagation |
| `OnDeath` | `event.rs` | `actor, wpos, killer` | Create episodic memory for witnesses, social memory propagation |
| `OnHelped` | `event.rs` | `actor, saver` | Positive episodic + emotional memory for both parties |
| `OnHealthChange` | `event.rs` | `actor, cause, new_health_fraction, change` | Combat memory (if damage significant) |
| `OnTheft` | `event.rs` | `actor, wpos, sprite, site` | Negative social memory, gossip propagation to witnesses |
| `OnMountVolume` | `event.rs` | `actor, pos` | Procedural memory (vehicle riding skill) |

### MEMZ should add new events:

```rust
// Proposed new events for MEMZ
pub struct OnDialogueMemory {
    pub npc: NpcId,
    pub player: Actor,
    pub topic: String,
    pub sentiment_change: f32,
}

pub struct OnTradeMemory {
    pub buyer: Actor,
    pub seller: Actor,
    pub site: SiteId,
    pub fairness: f32, // +1 = good deal, -1 = ripped off
}

pub struct OnGossipExchange {
    pub source: NpcId,
    pub target: NpcId,
    pub about: Actor,
    pub claim: String,
}
```

---

## 3. NPC Data Model

The `Npc` struct (`data/npc.rs:281`) is the primary data structure:

| Field | Type | MEMZ Relevance |
|-------|------|----------------|
| `seed` | `u32` | Deterministic personality derivation |
| `wpos` | `Vec3<f32>` | Location for episodic memories |
| `body` | `comp::Body` | NPC type (affects memory capacity) |
| `role` | `Role` | Profession, affects personality & priorities |
| `home` | `Option<SiteId>` | Settlement affiliation |
| `faction` | `Option<FactionId>` | Faction loyalty |
| `personality` | `Personality` | **KEY** — already has personality traits! |
| `sentiments` | `Sentiments` | **KEY** — existing sentiment system |
| `known_reports` | `HashSet<ReportId>` | **KEY** — existing knowledge system |
| `job` | `Option<Job>` | Current occupation |
| `controller` | `Controller` | NPC actions (say, attack, goto, dialogue) |
| `inbox` | `VecDeque<NpcInput>` | Messages from other NPCs/players |

### Existing Personality System

Veloren already has `Personality` with traits (see `common::rtsim::Personality`).
MEMZ should extend, not replace, this system. Map Veloren personality traits to
MEMZ's `PersonalityTraits`:

```rust
// Mapping proposal:
// Veloren's PersonalityTrait → MEMZ PersonalityTraits
// PersonalityTrait::Open → openness
// PersonalityTrait::Agreeable → 1.0 - emotional_volatility
// PersonalityTrait::Conscientious → inversely proportional to gossip_tendency
// PersonalityTrait::Adventurous → bravery
// (credulity needs to be derived from multiple traits or added)
```

### Existing Sentiment System

Veloren's `Sentiments` (`data/sentiment.rs`) already tracks how NPCs feel about
actors. Key details:

- `NPC_MAX_SENTIMENTS = 128` — Dunbar-like cap already exists!
- Sentiments decay over time (configurable `DECAY_TIME_FACTOR`)
- Supports targets: `Character`, `Npc`, `Faction`
- Has predefined levels: `POSITIVE`, `NEGATIVE`, `ALLY`, `RIVAL`, `FRIEND`,
  `ENEMY`, `HERO`, `VILLAIN`

**Integration strategy:** MEMZ's `EmotionalMemory` should *drive* Veloren's
sentiment values. When MEMZ computes a new emotional valence for an NPC toward
an actor, it updates the Veloren `Sentiment` accordingly.

### Existing Report System

`Report`s (`data/report.rs`) are Veloren's existing information-sharing primitive:

- Types: `Death { actor, killer }`, `Theft { thief, site, sprite }`
- Reports have `remember_for()` durations (1.5-15 game days)
- NPCs track `known_reports: HashSet<ReportId>`

**Integration strategy:** MEMZ's `SocialMemory` should be created when an NPC
learns about a `Report`. MEMZ adds richer information (trust, propagation depth,
belief status) on top of the boolean `known_reports`.

---

## 4. Dialogue System

The dialogue system (`rule/npc_ai/dialogue.rs`) is combinator-based:

- `session.say_statement(Content)` — NPC says something
- `session.dialogue_question(msg, responses)` — NPC asks with options
- `session.dialogue_response(tag, response)` — respond to a question

**MEMZ integration point:** Before generating dialogue, query the NPC's
`MemoryBank` for relevant memories about the player. Modify the `Content`
to reference these memories.

```rust
// Proposed hook in dialogue.rs::general()
fn memory_aware_greeting(ctx: &NpcCtx, tgt: Actor) -> Content {
    let memories = memz_retrieve(ctx.npc_id, tgt, top_k=3);
    if let Some(first_memory) = memories.first() {
        // "Ah, I remember you! Last time you [memory summary]"
        Content::localized("npc-dialogue-remembers")
            .with_arg("memory", first_memory.summary())
    } else {
        Content::localized("npc-dialogue-stranger")
    }
}
```

---

## 5. NPC AI System

The NPC AI (`rule/npc_ai/mod.rs`, 1769 lines) uses a combinator-based action
system. Key action types for MEMZ integration:

| Action Pattern | Where | MEMZ Hook |
|---------------|-------|-----------|
| `socialize()` | npc_ai/mod.rs | Trigger gossip exchange, social memory creation |
| `idle()` | npc_ai/mod.rs | Run reflection during idle, consolidation |
| `travel_to_site()` | movement.rs | Procedural memory for route knowledge |
| `do_work()` | npc_ai/mod.rs | Procedural memory for profession skills |
| `general()` | dialogue.rs | Memory-driven dialogue generation |

### Adding a MEMZ rule

MEMZ should be implemented as a new `Rule` that binds to existing events:

```rust
pub struct MemoryRule;

impl Rule for MemoryRule {
    fn start(state: &mut RtState) -> Result<Self, RuleError> {
        // Bind to OnTick for periodic memory operations
        state.bind::<Self, OnTick>(|ctx| {
            // 1. Run decay pass (every 60 ticks = ~2 seconds)
            // 2. Run eviction pass (every 3600 ticks = ~2 minutes)
            // 3. Trigger consolidation for idle NPCs
            // 4. Process gossip exchanges for socializing NPCs
        });

        // Bind to OnDeath for combat memories
        state.bind::<Self, OnDeath>(|ctx| {
            // Create episodic memories for all NPCs near the death location
            // Create emotional memories (grief/satisfaction)
            // Propagate death report as social memory
        });

        // Bind to OnHelped for positive interaction memories
        state.bind::<Self, OnHelped>(|ctx| {
            // Create positive episodic + emotional memories
        });

        // Bind to OnTheft for crime memories
        state.bind::<Self, OnTheft>(|ctx| {
            // Create negative social memories
            // Trigger gossip propagation about the thief
        });

        Ok(Self)
    }
}
```

---

## 6. Persistence Integration

Veloren's rtsim data is serialized via `serde` (all `Npc` fields marked
`#[serde(default)]`). The save path:

```
world_save/
├── server_config/
│   └── settings.ron
└── saves/
    └── server.world_0/
        └── rtsim.dat       ← rtsim Data, serialized
```

**MEMZ strategy:** Store memory banks in a *separate* SQLite database alongside
the rtsim save, not inside it. This keeps MEMZ save data independent and allows
backup/restore of memories separately.

```
saves/server.world_0/
├── rtsim.dat           ← Veloren's existing rtsim save
└── memz/
    └── memories.db     ← MEMZ SQLite database
```

---

## 7. Performance Considerations

From `rtsim/src/lib.rs` philosophy section:

- Rtsim handles **tens of thousands** of NPCs simultaneously
- Tick rates are **variable** (closer NPCs tick more frequently)
- Expensive operations must be **spread across multiple ticks**
- Code must handle **defensive programming** (IDs can be invalidated)

**MEMZ must:**
1. Only process memories for NPCs in loaded chunks (near players)
2. Spread decay/consolidation across many ticks (amortize)
3. Handle NPC deletion gracefully (soft-delete memory banks)
4. Never block the rtsim tick (async LLM calls)
5. Cache frequently accessed memory banks (LRU)

---

## 8. Integration Checklist

### Phase 0 (Foundation)
- [ ] Add `memz-core` as dependency in `veloren/rtsim/Cargo.toml`
- [ ] Create `MemoryRule` implementing `Rule` trait
- [ ] Add `MemoryBank` to `Npc` struct (or as side-table in `Data`)
- [ ] Bind `OnDeath`, `OnHelped`, `OnTheft` → episodic memory creation
- [ ] Bind `OnTick` → periodic decay
- [ ] Serialize/deserialize memory banks alongside rtsim save

### Phase 1 (Intelligence)
- [ ] Integrate embedding generation (async, off-tick)
- [ ] Hook into dialogue system → memory-aware greetings
- [ ] Map Veloren `Personality` → MEMZ personality weights
- [ ] Drive Veloren `Sentiments` from MEMZ emotional memories

### Phase 2 (Social Network)
- [ ] Hook into `socialize()` action → trigger gossip exchange
- [ ] Create `OnGossipExchange` event
- [ ] Map `Report` learning → MEMZ `SocialMemory` creation
- [ ] Town-level reputation aggregation per settlement

### Phase 3 (Player Injection)
- [ ] Create Memory Journal UI in Veloren's `voxygen/` client
- [ ] Hook player dialogue into memory injection pipeline
- [ ] Behavioral hooks: NPC responses to injected memories

---

## 9. File Reference

| Veloren Path | Purpose | MEMZ Relevance |
|-------------|---------|----------------|
| `rtsim/src/lib.rs` | Core RtState, Rule, Event system | Where MemoryRule gets registered |
| `rtsim/src/event.rs` | Event definitions | Events to bind memory handlers to |
| `rtsim/src/data/npc.rs` | Npc struct, Controller, Brain | Where MemoryBank attaches |
| `rtsim/src/data/sentiment.rs` | Sentiment system | MEMZ emotional memories drive this |
| `rtsim/src/data/report.rs` | Report/knowledge system | MEMZ social memories extend this |
| `rtsim/src/data/faction.rs` | Faction data | Faction-level memory aggregation |
| `rtsim/src/data/site.rs` | Site/settlement data | Settlement-scoped reputation |
| `rtsim/src/rule/npc_ai/mod.rs` | NPC AI brain (1769 lines) | Hook points for memory-driven behavior |
| `rtsim/src/rule/npc_ai/dialogue.rs` | Dialogue system | Memory-aware dialogue generation |
| `rtsim/src/rule/report.rs` | Report event creation | Creates reports that feed social memory |
| `rtsim/src/rule/simulate_npcs.rs` | NPC simulation loop | Where memory tick gets called |
| `rtsim/src/ai/mod.rs` | Action combinator system | Understand how to write MEMZ actions |
| `common/src/rtsim.rs` | Shared rtsim types | NpcId, Actor, Personality, etc. |
| `server/src/rtsim.rs` | Server-side rtsim integration | Where game events are emitted to rtsim |
