# 🎮 Veloren NPC System - Comprehensive Analysis

**Analysis Date:** March 22, 2026  
**Game Version:** 48e3b283 [2026-02-21]  
**Mode Analyzed:** Singleplayer  
**Your Game Log:** Generated 1976 rtsim NPCs

---

## Executive Summary

**YES, Veloren has extensive NPC characters in singleplayer mode!** 

Your game log shows:
```
Generated 16 rtsim factions.
Registering 201 rtsim sites from world sites.
Generated 1976 rtsim NPCs to be spawned.
```

This means your singleplayer world has **1,976 NPCs** that are actively simulated by Veloren's rtsim (Real-Time Simulation) system.

---

## What is Veloren's rtsim System?

**rtsim** = **Real-Time Simulation**

It's a sophisticated NPC simulation system that runs **independently of the game engine**, simulating NPCs even when they're not loaded in chunks. This is similar to how Dwarf Fortress simulates the entire world.

### Key Features

| Feature | Description |
|---------|-------------|
| **Persistent NPCs** | Each NPC has a unique ID, personality, profession, and state |
| **Dual-Mode Simulation** | NPCs are either "Simulated" (unloaded) or "Loaded" (in-game entity) |
| **Autonomous Behavior** | NPCs have goals, routines, and can make decisions |
| **Social Network** | NPCs have relationships, sentiments, and can gossip |
| **Memory System** | NPCs remember events, reports, and interactions |
| **Faction System** | NPCs belong to factions with relationships |

---

## NPC Population Breakdown

Based on the code analysis, here's what NPCs are generated in your world:

### Town NPCs (Based on Settlement Size)

For each settlement (town/village):
- **Guards:** ~25% of town population (1 guard per 4 plots)
- **Adventurers:** ~20% of town population (1 per 5 plots)
- **Merchants:** ~17% of town population (1 per 6 plots + 1)
- **Other Town NPCs:** ~38% (farmers, hunters, herbalists, blacksmiths, chefs, alchemists)

**Example:** A town with 40 plots would have:
- 10 Guards
- 8 Adventurers
- 7-8 Merchants
- 15 Other NPCs
- **Total: ~40 NPCs per town**

### Pirates

- **Pirate Captains:** 1 per pirate hideout
- **Pirates:** 10 per pirate hideout

### Flying Creatures (World-Scale)

Based on world size, distributed across the map:
- Cloud Wyverns
- Frost Wyverns
- Sea Wyverns
- Flame Wyverns
- Weald Wyverns
- Phoenix
- Roc
- Cockatrice

### Boss Monsters

- **Gigas Frost:** 1 per world
- **Gigas Fire:** 1 per world
- **Other Monsters:** 5-1000 depending on world size

### Airship Captains

- 1 captain + 1 airship per airship spawning location
- Captains are humanoid NPCs with the "Captain" profession
- They pilot airships between settlements

---

## NPC Data Structure

Each NPC in Veloren has the following properties:

```rust
pub struct Npc {
    pub uid: u64,                    // Unique identifier
    pub seed: u32,                   // For deterministic generation
    pub wpos: Vec3<f32>,            // World position
    pub dir: Vec2<f32>,             // Direction facing
    pub body: comp::Body,           // Physical body type
    pub role: Role,                 // Civilised/Monster/Wild/Vehicle
    pub home: Option<SiteId>,       // Home settlement
    pub faction: Option<FactionId>, // Faction membership
    pub health_fraction: f32,       // Current health (0.0-1.0)
    pub personality: Personality,   // OCEAN personality traits
    pub sentiments: Sentiments,     // Feelings toward other actors
    pub known_reports: HashSet<ReportId>, // Known events/gossip
    pub job: Option<Job>,           // Current occupation
    pub mode: SimulationMode,       // Simulated or Loaded
    // ... and more
}
```

---

## NPC Roles & Professions

### Civilised NPCs (Humanoids)

**Professions:**
- **Farmer** - Works fields, gathers crops
- **Hunter** - Hunts animals, gathers meat
- **Herbalist** - Gathers herbs and plants
- **Captain** - Pilots airships between settlements
- **Merchant** - Trades goods, has inventory
- **Guard** - Protects settlements, patrols
- **Adventurer** - Ranks 0-3, explores world
- **Blacksmith** - Crafts weapons and armor
- **Chef** - Prepares food
- **Alchemist** - Crafts potions
- **Pirate** - Hostile, found at hideouts
- **Cultist** - Found in dungeons

### Monster NPCs

- Various hostile creatures
- Boss monsters (Gigas)
- Flying creatures (Wyverns, Phoenix, Roc, etc.)

### Wild NPCs

- Passive animals
- Ambient wildlife

### Vehicle NPCs

- Airships (piloted by captains)

---

## NPC Personality System

Veloren NPCs have **OCEAN personality traits** (Big Five model):

```rust
pub struct Personality {
    openness: u8,           // 0-255: Closed-minded ↔ Open-minded
    conscientiousness: u8,  // 0-255: Careless ↔ Careful
    extraversion: u8,       // 0-255: Introverted ↔ Extraverted
    agreeableness: u8,      // 0-255: Antagonistic ↔ Agreeable
    neuroticism: u8,        // 0-255: Calm ↔ Neurotic
}
```

These traits affect:
- How NPCs react to events
- Their dialogue choices
- Their behavior patterns
- Their relationships with others

---

## NPC Sentiment System

NPCs track their feelings toward other actors (players and NPCs):

```rust
pub struct Sentiments {
    // Maps Actor → Sentiment value
    // Sentiment ranges from -1.0 (enemy) to +1.0 (friend)
}
```

**Sentiment Levels:**
- **+0.8 to +1.0:** Hero (deeply admired)
- **+0.6 to +0.8:** Friend (close friend)
- **+0.3 to +0.6:** Ally (trusted ally)
- **+0.1 to +0.3:** Positive (somewhat liked)
- **-0.1 to +0.1:** Neutral
- **-0.3 to -0.1:** Negative (somewhat disliked)
- **-0.6 to -0.3:** Rival (distrusted rival)
- **-0.8 to -0.6:** Enemy (despised enemy)
- **-1.0 to -0.8:** Villain (hated villain)

**Sentiments change based on:**
- Combat interactions (damage dealt/received)
- Helpful actions (healing, saving from danger)
- Dialogue choices
- Witnessed events
- Gossip from other NPCs

**Capacity Limit:** NPCs can track up to **128 sentiments** (Dunbar-inspired limit)

---

## NPC Memory & Knowledge System

### Reports System

NPCs are aware of **Reports** - significant events in the world:

```rust
pub enum Report {
    Death { actor, killer },
    Theft { thief, site, sprite },
    // ... more event types
}
```

- NPCs track `known_reports: HashSet<ReportId>`
- Reports have lifespans (1.5-15 game days)
- NPCs share reports with each other (gossip)
- Reports influence NPC behavior and dialogue

### Knowledge Sharing

- NPCs at their home site share reports with the site
- Sites maintain `known_reports` that all residents can access
- When NPCs interact, they can exchange information
- Trust in the source affects belief in the report

---

## NPC Simulation Modes

### Simulated Mode (Unloaded)

When NPCs are **outside loaded chunks**:
- Position updated based on activity
- Basic AI runs (pathfinding, goal selection)
- No physics simulation
- No rendering
- Minimal CPU cost

**Activities in Simulated Mode:**
- **Goto:** Move toward a position
- **Gather:** Collect resources
- **HuntAnimals:** Hunt nearby creatures
- **Talk:** Converse with another actor
- **Sit/Dance/Cheer:** Social activities

### Loaded Mode (In-Game Entity)

When NPCs are **inside loaded chunks** (near players):
- Full ECS entity with all components
- Physics simulation active
- Rendered on screen
- Agent AI system controls behavior
- Can interact with players
- Full combat system active

**Transition:** NPCs automatically switch between modes as players move around the world.

---

## NPC AI System

Veloren uses a **combinator-based AI system** for NPCs:

### Brain Structure

```rust
pub struct Brain {
    pub action: Box<dyn Action<(), !>>,
}
```

NPCs have a "brain" that executes **Actions** - composable behavior units similar to coroutines.

### Controller

```rust
pub struct Controller {
    pub actions: Vec<NpcAction>,      // Queued actions
    pub activity: Option<NpcActivity>, // Current activity
    pub look_dir: Option<Dir>,        // Where to look
    pub job: Option<Job>,             // Current job
    // ...
}
```

**NPC Actions:**
- `Say(target, content)` - Speak to someone
- `Attack(target)` - Attack an actor
- `Dialogue(target, dialogue)` - Start/continue dialogue
- `Msg { to, msg }` - Send message to another NPC

**NPC Activities:**
- `Goto(wpos, speed)` - Move to position
- `Talk(actor)` - Talk to someone
- `Gather(resources)` - Gather resources
- `HuntAnimals` - Hunt nearby animals
- `Sit/Dance/Cheer` - Social activities

---

## NPC Dialogue System

NPCs can engage in **structured dialogue** with players and each other:

### Dialogue Types

```rust
pub enum DialogueKind {
    Start,                          // Begin dialogue
    Question { tag, msg, responses }, // Ask with multiple choice
    Statement { msg, given_item, tag }, // Make statement, optionally give item
    Response { tag, response, response_id }, // Respond to question
    Marker(marker),                 // Provide location marker
    End,                            // End dialogue
}
```

### Dialogue Flow

1. NPC starts dialogue session
2. NPC can ask questions with multiple response options
3. Player/other NPC selects response
4. NPC reacts based on response
5. Dialogue continues or ends

**Example Dialogue:**
```
Guard: "Halt! What's your business here?"
  → [1] "Just passing through"
  → [2] "I'm looking for work"
  → [3] "None of your concern"

[Player selects 2]

Guard: "Work, eh? Talk to the merchant in the square."
[Dialogue ends, location marker added to map]
```

---

## NPC Jobs & Occupations

NPCs can have **Jobs** - long-running occupations:

```rust
pub enum Job {
    Hired(Actor, Time),  // Temporarily hired by an actor
    Quest(QuestId),      // Helping with a quest
}
```

- Jobs are persistent across sessions
- NPCs prioritize job-related tasks
- Jobs can be interrupted but are resumed
- Hired NPCs follow their employer

---

## NPC Mounting System

NPCs can **mount** other NPCs (e.g., riding horses, piloting airships):

```rust
pub struct NpcLink {
    pub mount: NpcId,      // The mount (vehicle/animal)
    pub rider: Actor,      // The rider
    pub is_steering: bool, // Is this rider controlling?
}
```

**Rules:**
- Each mount can have 1 steerer + multiple riders
- Riders move with the mount
- Steerer controls the mount's movement
- Dismounting breaks the link

**Example:** Airship captains are **steering** their airship NPCs.

---

## NPC Spawning Process

### 1. World Generation

When you start a new singleplayer world:

```rust
// Generate factions
let factions = generate_factions(16);

// Register sites from world
let sites = register_sites_from_world(201);

// Calculate wanted population
let population = wanted_population(world, sites);

// Generate NPCs
let npcs = generate_npcs(population); // 1976 NPCs
```

### 2. NPC Placement

NPCs are placed at:
- **Settlements:** Guards, merchants, farmers, etc.
- **Pirate Hideouts:** Pirates and captains
- **Airship Routes:** Captains piloting airships
- **World Roaming:** Adventurers, monsters, wildlife

### 3. Chunk Loading

When a player enters a chunk:
- Nearby NPCs switch to **Loaded** mode
- NPCs are spawned as ECS entities
- Full rendering and physics activated
- Agent AI takes over from rtsim AI

### 4. Chunk Unloading

When a player leaves a chunk:
- NPCs switch back to **Simulated** mode
- ECS entities despawned
- rtsim continues simulating position/behavior
- State persisted for next load

---

## NPC Persistence

### Save Data

NPCs are saved in: `userdata/singleplayer_worlds/world-{timestamp}/rtsim/data.dat`

**Saved Data:**
- NPC position, health, inventory
- Personality, sentiments, known reports
- Home, faction, profession
- Job status

**NOT Saved:**
- Current activity (recalculated on load)
- Brain state (rebuilt on load)
- Temporary controller state

### Database Migrations

Your game log shows **70 database migrations** applied:
```
Applied 70 database migrations
```

These migrations handle:
- Character data evolution
- Inventory/loadout changes
- Skill system updates
- Item stat changes
- Recipe book updates

---

## How to Find NPCs in Singleplayer

### 1. Visit Settlements

Press **M** to open the map. Look for:
- 🏘️ **Towns** - Large settlements with many NPCs
- 🏠 **Villages** - Smaller settlements
- 🏰 **Castles** - Guarded locations
- ⚓ **Pirate Hideouts** - Hostile NPCs

### 2. Look for Airships

Airships fly between settlements. Each has a captain NPC you can talk to.

### 3. Explore the World

- **Adventurer NPCs** roam the world
- **Monsters** spawn in various biomes
- **Wildlife** is everywhere

### 4. Use Debug Info

Press **F3** to see debug info, including nearby entities.

---

## NPC Interaction Examples

### Talking to NPCs

1. Approach an NPC
2. Press **E** to interact
3. Dialogue menu appears
4. Select responses
5. NPCs remember the interaction

### Trading with Merchants

1. Find a merchant NPC (usually in town squares)
2. Press **E** to interact
3. Select "Trade" option
4. Buy/sell items
5. Prices affected by economy simulation

### Combat with NPCs

- **Guards** will attack if you commit crimes
- **Monsters** are hostile by default
- **Pirates** attack on sight
- NPCs remember combat interactions (sentiments change)

---

## NPC Behavior Patterns

### Daily Routines

NPCs have routines based on profession:
- **Farmers:** Work fields during day, rest at night
- **Guards:** Patrol routes, defend settlement
- **Merchants:** Stand at stalls, trade with players
- **Adventurers:** Explore, travel between sites

### Social Behavior

- NPCs can **talk** to each other
- NPCs **share information** (gossip)
- NPCs **form relationships** (sentiments)
- NPCs **react to events** (deaths, thefts)

### Goal-Driven AI

NPCs have goals:
- **Survive:** Avoid danger, heal when hurt
- **Work:** Perform profession-related tasks
- **Socialize:** Interact with other NPCs
- **Defend:** Protect home settlement

---

## Comparison: Veloren vs MEMZ

| Feature | Veloren (Current) | MEMZ (Your Project) |
|---------|-------------------|---------------------|
| **NPC Count** | 1976 in your world | Same (uses Veloren NPCs) |
| **Personality** | OCEAN traits (5 dimensions) | PersonalityTraits (5 dimensions) |
| **Memory** | Reports (event-based) | 7 memory types (episodic, semantic, etc.) |
| **Sentiments** | Simple -1 to +1 scale | PAD emotional model + trust-weighted |
| **Gossip** | Basic report sharing | Bayesian belief update, trust decay |
| **Persistence** | SQLite (basic state) | SQLite (rich memory history) |
| **LLM Integration** | None | Ollama/OpenAI for dialogue/reflection |
| **Memory Decay** | Reports expire after days | Ebbinghaus forgetting curve |
| **Consolidation** | None | Episodic→Semantic, Procedural learning |

**MEMZ adds:**
- ✅ Persistent episodic memory ("I remember when...")
- ✅ Semantic knowledge extraction ("I know that...")
- ✅ Emotional memory with PAD model
- ✅ Trust-weighted gossip propagation
- ✅ LLM-powered reflection and dialogue
- ✅ Player memory injection
- ✅ Memory decay and consolidation

---

## Evidence from Your Game Log

Your singleplayer session shows active NPC simulation:

```
2026-03-22T13:11:57.789232Z  INFO veloren_rtsim::generate: Generated 16 rtsim factions.
2026-03-22T13:11:57.789262Z  INFO veloren_rtsim::generate: Registering 201 rtsim sites from world sites.
2026-03-22T13:11:57.789338Z  INFO veloren_rtsim::generate: Generated 1976 rtsim NPCs to be spawned.
```

**Rtsim Rules Started:**
```
2026-03-22T13:11:57.789368Z  INFO veloren_rtsim: Initiating 'veloren_rtsim::rule::migrate::Migrate' rule...
2026-03-22T13:11:57.789370Z  INFO veloren_rtsim: Initiating 'veloren_rtsim::rule::architect::Architect' rule...
2026-03-22T13:11:57.789372Z  INFO veloren_rtsim: Initiating 'veloren_rtsim::rule::replenish_resources::ReplenishResources' rule...
2026-03-22T13:11:57.789373Z  INFO veloren_rtsim: Initiating 'veloren_rtsim::rule::report::ReportEvents' rule...
2026-03-22T13:11:57.789375Z  INFO veloren_rtsim: Initiating 'veloren_rtsim::rule::sync_npcs::SyncNpcs' rule...
2026-03-22T13:11:57.789376Z  INFO veloren_rtsim: Initiating 'veloren_rtsim::rule::simulate_npcs::SimulateNpcs' rule...
2026-03-22T13:11:57.789377Z  INFO veloren_rtsim: Initiating 'veloren_rtsim::rule::npc_ai::NpcAi' rule...
2026-03-22T13:11:57.789379Z  INFO veloren_rtsim: Initiating 'veloren_rtsim::rule::cleanup::CleanUp' rule...
```

All 8 rtsim rules are active, meaning **NPCs are being fully simulated** in your singleplayer world!

---

## Conclusion

**Veloren has a sophisticated NPC system with 1,976 NPCs in your singleplayer world.**

These NPCs:
- ✅ Have unique personalities and professions
- ✅ Remember events and form relationships
- ✅ Simulate even when unloaded (rtsim)
- ✅ Can dialogue, trade, and fight
- ✅ Belong to factions and settlements
- ✅ Have daily routines and goals
- ✅ Share information through gossip
- ✅ Persist across game sessions

**Your MEMZ project will enhance these NPCs with:**
- Richer memory (7 types vs basic reports)
- LLM-powered dialogue and reflection
- Scientific memory decay (Ebbinghaus curve)
- Trust-weighted belief updates
- Player memory injection
- Emotional modeling (PAD)

The foundation is already excellent - MEMZ will make it extraordinary! 🧠

---

## Next Steps

1. **Play the game** - Visit towns, talk to NPCs, see the system in action
2. **Explore settlements** - Press M to find towns on the map
3. **Interact with NPCs** - Press E to talk, trade, and learn
4. **Observe behaviors** - Watch NPCs patrol, work, and socialize
5. **Plan MEMZ integration** - See `docs/veloren-rtsim-hooks.md` for integration points

**Enjoy your 1,976 NPCs!** 🎮⚔️🏰
