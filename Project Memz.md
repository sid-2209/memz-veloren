# 🧠 Project MEMZ — Persistent Memory Layer for Game Characters

### *"Every character remembers. Every interaction matters. Every world is alive."*

---

**Project Codename:** MEMZ  
**Author:** Siddhartha  
**Date:** 20 February 2026  
**Version:** 3.0 — Production-Ready Edition  
**Classification:** Production-Grade Design Specification  
**Virality Target:** 🔥 Open-source sensation — top of Hacker News, r/gaming, r/gamedev, r/artificial, YouTube creators, Twitch streamers

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [The Idea — Distilled](#2-the-idea--distilled)
3. [Why This Will Go Viral](#3-why-this-will-go-viral)
4. [Competitive Landscape & Prior Art](#4-competitive-landscape--prior-art)
5. [Game Selection — The Decision Matrix](#5-game-selection--the-decision-matrix)
6. [The Chosen Game: Veloren](#6-the-chosen-game-veloren)
7. [Architecture — The Memory Layer](#7-architecture--the-memory-layer)
8. [Memory Types & Taxonomy](#8-memory-types--taxonomy)
   - 8.1 Episodic — "What happened"
   - 8.2 Semantic — "What I know"
   - 8.3 Emotional — "How I feel"
   - 8.4 Social — "What I've heard"
   - 8.5 Reflective — "What I think"
   - 8.6 Injected (Player Only) — "My backstory"
   - 8.7 Procedural — "What I know how to do" *(NEW)*
   - 8.8 Memory Consolidation — "What I've internalized" *(NEW)*
9. [Character Memory Interactions](#9-character-memory-interactions)
10. [Player Memory Injection System](#10-player-memory-injection-system)
11. [The "Living Lore" Engine](#11-the-living-lore-engine)
12. [Technical Design](#12-technical-design)
   - 12.1 Technology Stack
   - 12.2 ECS Integration
   - 12.3 LLM Integration — Tiered Approach
   - 12.3.1 Prompt Engineering *(NEW)*
   - 12.4 Memory Retrieval Algorithm
   - 12.5 Persistence & Save Integration
   - 12.6 Performance Budget
   - 12.7 Hardware Requirements & Scaling Profiles
   - 12.8 Multiplayer Architecture
13. [Differentiation — Why MEMZ ≠ Mantella ≠ Generative Agents](#13-differentiation--why-memz--mantella--generative-agents)
14. [Viral Mechanics — Built for Shareability](#14-viral-mechanics--built-for-shareability)
   - 14.0 The "First 5 Minutes" Experience *(NEW)*
   - 14.1 Memory Replay (with technical design)
   - 14.2 Reputation Board (with data model)
   - 14.3 Memory Conflict Events
   - 14.4 NPC Diary Easter Egg
   - 14.5 Twitch Integration
15. [Phased Roadmap](#15-phased-roadmap)
16. [Community & Open Source Strategy](#16-community--open-source-strategy)
   - Repository Structure
   - Example Configuration (memz.toml) *(NEW)*
   - Community Engagement Plan — Full Launch Strategy *(EXPANDED)*
   - Press Kit *(NEW)*
17. [Risks & Mitigations](#17-risks--mitigations)
18. [Success Metrics](#18-success-metrics)
19. [Open Questions & Brainstorm Notes](#19-open-questions--brainstorm-notes)
   - Resolved Questions ✅ *(NEW)*
   - Open Questions (Active)
   - Bard System — Promoted to Core *(NEW)*
   - Wild Ideas (Remaining)
20. [Benchmark Standards & Quality Gates](#20-benchmark-standards--quality-gates)
21. [Security, Safety & Abuse Prevention](#21-security-safety--abuse-prevention)
22. [Accessibility & Inclusivity](#22-accessibility--inclusivity)
23. [Observability, Telemetry & Debugging](#23-observability-telemetry--debugging)
24. [Appendix — Research References](#24-appendix--research-references)

---

## 1. Executive Summary

**MEMZ** is an open-source project that adds a **persistent, LLM-powered memory layer** to every character (main characters, NPCs, creatures, merchants, quest-givers — *everyone*) in an existing popular open-source game, so that:

- **Every NPC remembers** every conversation, every interaction, every event they've witnessed.
- **NPCs form opinions** about the player and about each other — opinions that evolve over time.
- **NPCs gossip, warn, reminisce, and scheme** based on their accumulated memories.
- **The player can inject memories** into their own character — creating backstories, false memories, traumas, motivations — and the game world *reacts* to them.
- **NPC-to-NPC interactions are memory-enabled** — they remember their relationships, feuds, alliances, and shared experiences.
- **Sub-2ms frame budget** — the memory system runs within a strict performance envelope, never degrading gameplay below 60 FPS on mid-range hardware.
- **Offline-first by design** — the entire system runs locally with no cloud dependency using quantized local models, with optional cloud LLM escalation for richer interactions.
- **Multiplayer-native** — server-authoritative memory state scales to hundreds of concurrent players with per-player NPC memory streams.

The result: **the first truly alive open-source game world**, where no two playthroughs are the same because the world *remembers*.

### Design Principles

| Principle | Description |
|-----------|-------------|
| **🎮 Gameplay-First** | Memory must enhance gameplay, never interrupt it. If the LLM is slow, fall back gracefully. Zero loading screens, zero hitches. |
| **⚡ Performance-Obsessed** | <2ms per frame budget. Async-only LLM calls. Lock-free data structures. Zero-allocation hot paths. |
| **🔒 Deterministic Fallbacks** | Every LLM-powered feature has a rule-based fallback. The game is fully playable with zero LLM access. |
| **🌐 Offline-First** | Local models (Ollama/llama.cpp) are the primary path. Cloud APIs are optional enhancements. |
| **🧪 Scientifically Grounded** | Memory architecture based on peer-reviewed cognitive science: Tulving's memory taxonomy, Ebbinghaus decay curves, Dunbar's social network limits. |
| **🛡️ Safe & Inclusive** | Content filtering, abuse prevention, accessibility-first UI. No player-generated content can produce harmful NPC behavior. |

---

## 2. The Idea — Distilled

```
┌─────────────────────────────────────────────────────────┐
│                    THE CORE THESIS                       │
│                                                         │
│  What if every character in a game had a brain?          │
│  Not scripted dialogue trees.                            │
│  Not pre-written responses.                              │
│  A real, persistent memory that grows, fades,            │
│  connects, and shapes behavior.                          │
│                                                         │
│  And what if YOU — the player — could plant              │
│  memories in your own character's mind?                   │
│                                                         │
│  "I remember a village burning. I was five years old.    │
│   I couldn't save them."                                 │
│                                                         │
│  And the game REACTS to that.                            │
│  NPCs sense your trauma. They respond to it.             │
│  The world bends around your story.                      │
└─────────────────────────────────────────────────────────┘
```

### The Three Pillars

| Pillar | Description |
|--------|-------------|
| **🧠 NPC Memory** | Every NPC has a persistent memory stream — observations, conversations, emotions, reflections. They remember YOU. They remember each other. |
| **🗣️ Inter-Character Memory Sharing** | Characters who interact share context. A merchant who sees you steal tells the guard. The guard remembers next time. The blacksmith heard from the guard. Word spreads. |
| **💉 Player Memory Injection** | The player can write memories for their character. These memories become part of the character's identity. NPCs respond to them. The world adapts. |

---

## 3. Why This Will Go Viral

### The Moment-Factory

This project is designed to generate **shareable moments** — the kind of thing people screenshot, clip, and post:

> *"I told the innkeeper I was an orphan. Three in-game days later, she left me food outside her door with a note: 'I know what it's like. Stay strong.'"*

> *"I stole from a merchant in the first town. I came back 20 hours later. He recognized me. He'd told every merchant in town. No one would trade with me."*

> *"Two NPCs I'd never interacted with were arguing in the town square — about ME. One was defending me based on something I'd done 10 hours ago."*

> *"I injected a memory that my character lost a brother in war. When I met a war veteran NPC, he paused, looked at me, and said 'You've seen loss too, haven't you?' I nearly cried."*

### Virality Vectors

| Vector | Why It Works |
|--------|-------------|
| **YouTube / Twitch** | Streamers will have *unique* NPC interactions every session. Chat will go wild. "WHAT DID HE JUST SAY?" moments every 10 minutes. |
| **Reddit / X / TikTok** | Screenshots of NPC conversations that feel *impossibly human*. Short clips of NPCs gossiping about the player. |
| **Hacker News / Dev Twitter** | The technical architecture is genuinely novel — memory-augmented game characters with episodic/semantic/emotional layers. This is publishable research. |
| **Gaming Press** | "This open-source mod makes every NPC remember everything" — Kotaku, PC Gamer, Rock Paper Shotgun will cover this. |
| **AI Community** | This is a practical application of memory-augmented agents in an interactive environment. AI researchers will cite this. |

### The "Tell Your Friends" Hook

The core viral mechanic: **no two players will EVER have the same experience.** Your NPC relationships, your world's gossip network, your character's injected memories — they create a world that's *yours*. People will want to share their unique stories.

---

## 4. Competitive Landscape & Prior Art

### 4.1 Stanford Generative Agents (Park et al., 2023)

**Paper:** [Generative Agents: Interactive Simulacra of Human Behavior](https://arxiv.org/abs/2304.03442)  
**GitHub:** [joonspk-research/generative_agents](https://github.com/joonspk-research/generative_agents) — ⭐ 20.7k stars

**What it does:**
- 25 agents in a tiny 2D town ("Smallville")
- Memory stream → Reflection → Planning → Action loop
- Agents form relationships, spread information, plan events (Valentine's Day party)
- Uses LLMs for all reasoning

**Why MEMZ is different:**
| Generative Agents | MEMZ |
|---|---|
| Research demo — 25 agents in a toy world | Real game with hundreds of NPCs, combat, quests, economy |
| Not interactive — you observe | You PLAY. You're a character. You inject memories. |
| No game mechanics (no combat, inventory, quests) | Full RPG with memory as a core mechanic |
| Runs in a web browser with 2D sprites | 3D voxel RPG with real gameplay |
| No NPC-to-player relationship depth | NPCs form deep, evolving opinions about YOU |

### 4.2 Mantella — Skyrim AI NPC Mod

**Nexus:** [Mantella - Bring NPCs to Life with AI](https://www.nexusmods.com/skyrimspecialedition/mods/98631) — 138k unique downloads  
**GitHub:** [art-from-the-machine/Mantella](https://github.com/art-from-the-machine/Mantella) — ⭐ 338 stars

**What it does:**
- Voice-to-voice conversations with Skyrim NPCs via LLMs
- NPCs remember previous conversations (basic memory)
- NPCs aware of in-game context (location, time, items)
- Uses Whisper (STT) + LLM + Piper/xVASynth (TTS)

**Why MEMZ is different:**
| Mantella | MEMZ |
|---|---|
| Bolted onto a proprietary game (Skyrim) | Built INTO an open-source game (Veloren) |
| Memory is conversation history — flat | Memory is structured: episodic, semantic, emotional, procedural |
| No NPC-to-NPC memory propagation | Full social memory network — gossip, reputation, rumors |
| No player memory injection | Player can create/inject memories that reshape the world |
| Skyrim's NPCs have fixed personalities | NPC personalities EVOLVE based on accumulated experiences |
| Can't modify the game's core systems | We modify the game's NPC AI, dialogue, quests, economy |
| Voice-focused (STT/TTS) | Memory-focused — text-first, voice optional |

### 4.3 Other Prior Art

| Project | What It Does | Gap MEMZ Fills |
|---------|-------------|----------------|
| **AI Town** (a16z) | Generative agents in a web app | Not a real game; no player agency |
| **Voyager** (NVIDIA) | LLM agent plays Minecraft | Single agent, no NPC memory network |
| **CALM** (DeepMind) | Language agents in text games | No 3D world, no social dynamics |
| **SIMA** (Google DeepMind, 2024) | Instructable agent across 3D games via keyboard/mouse | Agent plays games, doesn't give NPCs memory; no social fabric |
| **Lyfe Agents** (Kaiya et al., 2023) | Low-cost real-time generative agents with Summarize-and-Forget memory | Sandbox demo only; not integrated into a real playable game |
| **Letta / MemGPT** (Packer et al.) | Persistent memory for LLM agents (21.2k ⭐) | General-purpose agent framework, not game-specific; no spatial/social graph |
| **Veloren's rtsim** | Basic real-time NPC simulation | No memory, no LLM, no persistence of interaction history |
| **Dwarf Fortress** | Rich NPC histories (procedural) | Scripted histories, not learned/evolved; not open-source |
| **Inworld AI** (Commercial) | Commercial AI NPC platform for AAA studios | Proprietary, closed-source, cloud-dependent, expensive licensing |
| **Convai** (Commercial) | Voice-enabled AI NPCs for games | Cloud-only, latency-heavy, no open-source game integration |

### The Gap Nobody Has Filled

```
Nobody has built a system where:
  ✅ Every character in a real, playable game has persistent LLM-backed memory
  ✅ Characters share memories with each other through social interaction
  ✅ The player can inject memories that alter the game world's response
  ✅ All of this runs in an open-source game that anyone can play and modify
  ✅ The memory layer itself is a reusable open-source library (game-agnostic)
  ✅ The system runs fully offline with local models — no cloud dependency
  ✅ Memory architecture is grounded in cognitive science (episodic/semantic/emotional taxonomy)
  ✅ It operates within a strict real-time performance budget (<2ms/frame)
  ✅ Multiplayer-native: NPCs maintain per-player memory streams in shared worlds
  ✅ Commercial AI NPC solutions (Inworld, Convai) are proprietary/cloud-only — MEMZ is open

THAT is what MEMZ does. That is why it's unparalleled.
```

---

## 5. Game Selection — The Decision Matrix

We evaluated every major open-source game with NPC characters. Here's the analysis:

### Evaluation Criteria

| Criterion | Weight | Description |
|-----------|--------|-------------|
| **NPC Richness** | 25% | How many NPCs? How complex are their existing behaviors? Do they have roles (merchants, guards, quest-givers)? |
| **Modifiability** | 20% | Can we deeply modify the NPC AI system? Is the codebase clean? |
| **Active Community** | 15% | Is the game actively developed? Will people actually play our mod? |
| **Visual Appeal** | 15% | Does it look good on YouTube/Twitch? Will screenshots go viral? |
| **License** | 10% | Can we freely modify and redistribute? |
| **Existing NPC System** | 10% | Is there already an NPC simulation to build on top of? |
| **Language/Tech Stack** | 5% | Is it written in something we can extend cleanly? |

### The Contenders

| Game | Stars | NPCs | Lang | License | Active | Score |
|------|-------|------|------|---------|--------|-------|
| **Veloren** | 7.1k | ✅ Rich (merchants, guards, villagers, pirates, airship captains) | Rust | GPL-3.0 | ✅ Very active (commits daily) | **92/100** |
| **Cataclysm: DDA** | 12.1k | ⚠️ Mostly hostile, limited NPC interaction | C++ | CC BY-SA 3.0 | ✅ Very active | 65/100 |
| **Battle for Wesnoth** | 6.4k | ⚠️ Turn-based, limited real-time NPC interaction | C++/Lua | GPL-2.0 | ✅ Active | 55/100 |
| **Luanti (Minetest)** | 12.3k | ⚠️ No built-in NPCs (engine only) | C++/Lua | LGPL-2.1 | ✅ Active | 50/100 |
| **Space Station 14** | 3.5k | ✅ Rich character interactions | C# | MIT | ✅ Active | 72/100 |
| **Shattered Pixel Dungeon** | — | ⚠️ Roguelike, minimal NPC depth | Java | GPL-3.0 | ✅ Active | 40/100 |
| **Ryzom** | — | ✅ MMORPG NPCs | C++ | AGPL-3.0 | ⚠️ Declining | 58/100 |

### Why Veloren Wins — Decisively

| Factor | Veloren Advantage |
|--------|-------------------|
| **NPC Ecosystem** | Veloren has a fully simulated NPC world via `rtsim` (Real-Time Simulation). NPCs have jobs, travel routes, belong to factions, trade, and interact. This is the PERFECT foundation to add memory on top of. |
| **Active Development** | 365+ contributors, daily commits, active Discord/Zulip community, weekly dev blogs. They'll likely LOVE this contribution. |
| **Visual Appeal** | Gorgeous voxel art style. Looks fantastic on stream. Beautiful landscapes, character designs, and animations. Perfect for viral screenshots/clips. |
| **Rust Codebase** | Clean, modern Rust. Memory-safe. Excellent for building complex systems. The ECS architecture is perfect for adding a memory component. |
| **Existing rtsim** | Veloren already has `rtsim` — a real-time simulation system for NPCs. NPCs already have: factions, professions, travel behavior, personality traits. We're adding memory ON TOP of this — not building from scratch. |
| **Multiplayer** | Veloren is multiplayer. Imagine multiple players in a shared world where NPCs remember ALL of them and gossip between players' interactions. |
| **GPL-3.0** | Fully open. We can fork, modify, and redistribute. No restrictions. |
| **Community Size** | ~20 active devs, 150+ contributors, active player base. Big enough to matter, small enough that a major contribution gets noticed. |

---

## 6. The Chosen Game: Veloren

### What is Veloren?

> **Veloren** is an open-world, open-source multiplayer voxel RPG written in Rust. Inspired by Cube World, The Legend of Zelda: Breath of the Wild, Dwarf Fortress, and Minecraft. GPL-3.0 licensed.

**Website:** https://veloren.net  
**GitLab:** https://gitlab.com/veloren/veloren  
**GitHub Mirror:** https://github.com/veloren/veloren  
**Discord:** https://veloren.net/discord  
**Wiki:** https://wiki.veloren.net  

### Current NPC System (rtsim)

Veloren's `rtsim` (real-time simulation) module already simulates NPC behavior:

```
Current rtsim capabilities:
├── NPC Scheduling — NPCs have daily routines
├── Faction Membership — NPCs belong to groups (villagers, pirates, merchants)
├── Professions — Blacksmiths, merchants, guards, farmers
├── Travel — NPCs move between settlements, ride airships
├── Trading — NPCs participate in economy
├── Combat — NPCs defend territory, attack hostiles
├── Relationships — Basic NPC-NPC relationships exist
└── Persistence — NPC state persists across server restarts
```

**What's MISSING (and what MEMZ adds):**

```
MEMZ additions:
├── 🧠 Episodic Memory — "I remember when the player helped me fight wolves"
├── 🧠 Semantic Memory — "I know the player is a warrior who prefers axes"
├── 🧠 Emotional Memory — "The player made me feel safe / threatened / betrayed" (PAD model)
├── 🧠 Social Memory — "The guard told me the player is a thief" (trust-weighted)
├── 🧠 Reflective Memory — "After thinking about it, I believe the player is good"
├── 🧠 Procedural Memory — "I know how to craft iron swords" (skill-based, profession-tied)
├── 🗣️ Memory-Driven Dialogue — NPCs reference memories in conversation
├── 📡 Memory Propagation — Memories spread through social networks (Dunbar-limited)
├── 💉 Player Memory Injection — Player writes their own backstory
├── 📊 Reputation System — Built from aggregated NPC memories (per-town, per-faction)
├── 🌊 Memory Decay — Ebbinghaus forgetting curve with emotional flashbulb protection
├── 🔁 Memory Consolidation — Periodic episodic → semantic distillation (like sleep)
└── 🎭 Personality Evolution — NPC personality traits shift based on accumulated experience
```

---

## 7. Architecture — The Memory Layer

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        VELOREN GAME                             │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌──────────┐  │
│  │  Combat    │  │  Trading   │  │  Quests    │  │  rtsim   │  │
│  │  System    │  │  System    │  │  System    │  │  (NPCs)  │  │
│  └─────┬──────┘  └─────┬──────┘  └─────┬──────┘  └────┬─────┘  │
│        │               │               │              │         │
│        └───────────────┼───────────────┼──────────────┘         │
│                        │               │                        │
│                   ┌────▼───────────────▼────┐                   │
│                   │   EVENT BUS / HOOKS     │                   │
│                   │   (Game Event Stream)   │                   │
│                   └────────────┬────────────┘                   │
└────────────────────────────────┼────────────────────────────────┘
                                 │
                    ┌────────────▼────────────┐
                    │                         │
                    │     MEMZ CORE ENGINE    │
                    │                         │
                    │  ┌───────────────────┐  │
                    │  │  Event Ingestion  │  │
                    │  │  & Observation    │  │
                    │  └────────┬──────────┘  │
                    │           │              │
                    │  ┌────────▼──────────┐  │
                    │  │  Memory Formation │  │
                    │  │  (Encoding)       │  │
                    │  └────────┬──────────┘  │
                    │           │              │
                    │  ┌────────▼──────────┐  │
                    │  │  Memory Store     │  │
                    │  │  (Per-Character)  │  │
                    │  └────────┬──────────┘  │
                    │           │              │
                    │  ┌────────▼──────────┐  │
                    │  │  Retrieval &      │  │
                    │  │  Reflection       │  │
                    │  └────────┬──────────┘  │
                    │           │              │
                    │  ┌────────▼──────────┐  │
                    │  │  Memory-Driven    │  │
                    │  │  Behavior Engine  │  │
                    │  └──────────────────┘   │
                    │                         │
                    └─────────────────────────┘
                                 │
                    ┌────────────▼────────────┐
                    │   LLM INFERENCE LAYER   │
                    │                         │
                    │  Local: Ollama/llama.cpp │
                    │  Cloud: OpenAI/Claude    │
                    │  Hybrid: Local + Cloud   │
                    └─────────────────────────┘
```

### Memory Architecture — Per Character

Every character (NPC or player) gets a `MemoryBank`:

```
MemoryBank (per character)
│
├── EpisodicMemory[]
│   ├── event: "Player gave me 10 gold coins"
│   ├── timestamp: game_day_42, hour_14
│   ├── location: "Veloren Town Square"
│   ├── participants: [player_id, self]
│   ├── emotional_valence: +0.7 (positive)
│   ├── importance: 0.6
│   ├── decay_rate: 0.02/day
│   └── embedding: [0.12, -0.34, 0.56, ...]  // for retrieval
│
├── SemanticMemory[]
│   ├── fact: "The player prefers axes over swords"
│   ├── confidence: 0.85
│   ├── source: [episodic_memory_refs]
│   ├── last_reinforced: game_day_45
│   └── embedding: [...]
│
├── EmotionalMemory[]
│   ├── target: player_id
│   ├── emotion: "trust"
│   ├── intensity: 0.7
│   ├── basis: [episodic_memory_refs]
│   └── last_updated: game_day_44
│
├── SocialMemory[]
│   ├── about: player_id
│   ├── source: "guard_npc_42"
│   ├── claim: "The player was seen stealing from the market"
│   ├── believed: true
│   ├── trust_in_source: 0.8
│   └── timestamp: game_day_41
│
├── ReflectiveMemory[]
│   ├── reflection: "I think the player is trying to earn my trust after stealing"
│   ├── basis: [episodic_refs, social_refs]
│   ├── generated_at: game_day_43
│   └── confidence: 0.6
│
└── InjectedMemory[] (player character only)
    ├── memory: "I lost my family in a fire when I was young"
    ├── injected_by: player
    ├── emotional_weight: 0.9
    ├── affects_behavior: true
    └── known_to_npcs: false (until shared/detected)
```

---

## 8. Memory Types & Taxonomy

### 8.1 Episodic Memory — "What happened"

The raw stream of experiences. Every significant event a character witnesses or participates in becomes an episodic memory.

**Triggers:**
- Player speaks to NPC
- Player performs an action near NPC (fighting, stealing, crafting, trading)
- NPC witnesses a world event (attack on village, weather event, another NPC's action)
- NPC-to-NPC interaction

**Properties:**
- `event_description`: Natural language description
- `participants`: Who was involved
- `location`: Where it happened
- `timestamp`: When it happened
- `emotional_valence`: How it made the NPC feel (-1.0 to +1.0)
- `importance`: How significant (0.0 to 1.0) — computed from novelty, emotional intensity, and relevance to NPC's goals
- `decay_rate`: How fast the memory fades
- `embedding`: Vector embedding for semantic retrieval

**Example:**
```json
{
  "event": "A stranger wearing iron armor approached me at my blacksmith stall and asked about sword prices. They seemed interested in my work and complimented my craftsmanship.",
  "participants": ["player_7", "self"],
  "location": "Riverstead Marketplace",
  "timestamp": "Day 12, Hour 14",
  "emotional_valence": 0.4,
  "importance": 0.3,
  "decay_rate": 0.03
}
```

### 8.2 Semantic Memory — "What I know"

Distilled facts and knowledge derived from episodic memories. These are the NPC's beliefs about the world.

**Formation:** Generated periodically by the reflection engine, which examines recent episodic memories and extracts patterns.

**Example:**
```json
{
  "fact": "The player is a skilled warrior who frequents Riverstead",
  "confidence": 0.8,
  "derived_from": ["ep_memory_12", "ep_memory_34", "ep_memory_56"],
  "category": "person_knowledge"
}
```

### 8.3 Emotional Memory — "How I feel"

Persistent emotional associations with entities, places, or concepts.

**Properties:**
- `target`: What/who the emotion is about
- `emotion`: Primary emotion (trust, fear, admiration, resentment, affection, suspicion, etc.)
- `intensity`: 0.0 to 1.0
- `trajectory`: increasing / stable / decreasing

**Example:**
```json
{
  "target": "player_7",
  "emotion": "trust",
  "intensity": 0.65,
  "trajectory": "increasing",
  "basis": "Player helped defend the village, has been fair in trades"
}
```

### 8.4 Social Memory — "What I've heard"

Information received from other NPCs. This is the gossip/rumor system.

**Key mechanic:** NPCs don't automatically believe what they hear. They weigh it against:
- Their trust in the source
- Their own experiences
- Consistency with other information

**Example:**
```json
{
  "about": "player_7",
  "source": "guard_elena",
  "claim": "I saw the stranger lurking near the armory at night",
  "believed": false,
  "reason": "I trust the stranger — they helped me. Elena is known to exaggerate.",
  "trust_in_source": 0.4
}
```

### 8.5 Reflective Memory — "What I think"

Higher-order thoughts generated by periodically reflecting on accumulated memories. This is where NPC "wisdom" and "insight" emerge.

**Generated by:** LLM reflection prompts that ask: "Given everything you remember, what do you think about X?"

**Example:**
```json
{
  "reflection": "The stranger has been consistently kind to everyone in town, but I've noticed they always avoid the eastern gate. I wonder what happened there.",
  "basis": ["ep_42", "ep_67", "ep_89", "semantic_12"],
  "generated_at": "Day 30"
}
```

### 8.6 Injected Memory (Player Only) — "My backstory"

Memories the player writes for their own character. These become part of the character's identity and influence how NPCs respond.

**Example:**
```
Player injects: "I was once a soldier in the Northern Wars. I lost my best 
friend in battle. I carry guilt because I couldn't save him."

Result:
- War veteran NPCs recognize "that look in your eyes"
- NPCs in military areas treat you with respect/camaraderie
- If an NPC friend is in danger, your character shows heightened urgency
- Tavern conversations may trigger PTSD-like responses (character refuses 
  to talk about certain topics)
```

### 8.7 Procedural Memory — "What I know how to do"

Skills, routines, and behavioral patterns learned through repetition. This is how NPCs develop **expertise** and **habits** over time — the "muscle memory" of the game world.

**Cognitive Science Basis:** Tulving (1985) distinguishes procedural memory as implicit knowledge about *how* to do things, separate from *what* happened (episodic) or *what* is true (semantic). Anderson's ACT-R theory (1993) models skill acquisition as production rules that strengthen with practice.

**Key Mechanics:**
- NPCs develop **skill proficiency** through repeated actions (a blacksmith who forges 100 swords gets faster and produces higher quality)
- **Habit formation**: Routines crystallize from repeated episodic patterns (e.g., if an NPC walks to the well every morning for 30 days, it becomes a procedural habit — no longer episodic, just "what I do")
- **Skill transfer**: Related skills accelerate learning (a skilled swordsmith learns dagger-making faster)
- **Rust-out**: Skills unused for long periods degrade (Ebbinghaus decay applied to proficiency, but slower than episodic decay — procedural memories are stickier)

**Data Structure:**
```json
{
  "type": "procedural",
  "skill": "blacksmithing.sword_forging",
  "proficiency": 0.82,
  "repetitions": 147,
  "last_practiced": "Day 45",
  "learning_rate": 0.03,
  "related_skills": ["blacksmithing.dagger", "blacksmithing.repair"],
  "behavioral_routine": {
    "trigger": "morning",
    "action": "go_to_forge → stoke_fire → begin_work",
    "strength": 0.95
  }
}
```

**Gameplay Impact:**
- NPC artisans produce visibly better goods over time — the blacksmith you visit on Day 1 vs Day 100 gives different quality items
- NPCs recommend other NPCs based on procedural expertise: *"For swords, see Kael. But if you need a bow, old Mira is the finest crafter in three towns."*
- Disrupting an NPC's routine (e.g., destroying their forge) creates emotional distress and behavioral change — they don't just respawn their routine
- Expert NPCs can **teach** the player, with teaching quality proportional to proficiency

### 8.8 Memory Consolidation — "What I've internalized"

The process by which memories transform between types over time, modeling how human memory actually works.

**Cognitive Science Basis:** Memory consolidation theory (Stickgold & Walker, 2013) — during "sleep" or downtime, the brain replays and reorganizes memories. Important episodic memories become semantic knowledge; repeated behaviors become procedural; emotional coloring fades or intensifies.

**Consolidation Rules:**
```
During NPC "rest" periods (offline/sleep cycle):

  Episodic → Semantic:
    IF episodic_memory.access_count > 3 AND similar_episodes > 5
    THEN extract_generalization → create_semantic_memory
    Example: 5 separate "player bought bread" episodes → 
             semantic: "this player is a regular customer"

  Episodic → Procedural:
    IF action_pattern.repetitions > 20 AND consistency > 0.8
    THEN crystallize_routine → create_procedural_memory
    Example: 20 mornings walking to the well → 
             procedural habit: "morning_routine: go_to_well"

  Emotional → Semantic:
    IF emotional_memory.age > 30_days AND valence.stable
    THEN distill_disposition → update_semantic_relationship
    Example: months of positive interactions → 
             semantic: "I consider this player a trusted friend"

  Consolidation also triggers FORGETTING:
    - Source memories (episodic) decay faster after consolidation
    - Only the "gist" survives as semantic/procedural
    - This is a FEATURE: NPCs don't remember every detail, just like humans
```

**Performance:** Consolidation runs as a background ECS system during NPC idle ticks. Budget: < 0.1ms per NPC per consolidation cycle (runs once per in-game day, not every frame).

---

## 9. Character Memory Interactions

### 9.1 NPC → Player Interactions

```
Scenario: Player returns to a town after 10 in-game days

Blacksmith (remembers you): "Back again! Last time you were here, you bought 
that iron axe. Did it serve you well against those wolves?"

Guard (remembers your crime): "You... I remember you. You were the one 
skulking around the eastern wall last week. I'm watching you."

Innkeeper (heard gossip): "Ah, you must be the adventurer everyone's been 
talking about. The fisherman said you saved his boat during the storm."

New NPC (no memory): "Haven't seen you around here before, stranger. 
What brings you to Riverstead?"
```

### 9.2 NPC → NPC Memory Propagation

```
Memory Propagation Network:

Player steals from Merchant_A
    │
    ▼
Merchant_A forms memory: "Player is a thief" (high confidence, witnessed)
    │
    ├──► Guard_B (Merchant_A reports to guard)
    │    Guard_B forms social memory: "Merchant says player stole"
    │    Guard_B's trust in Merchant_A: 0.9 → believes it
    │    Guard_B's behavior: increased suspicion of player
    │    │
    │    └──► Guard_C (shift change, Guard_B tells Guard_C)
    │         Guard_C forms social memory (second-hand)
    │         Lower confidence (hearsay)
    │
    ├──► Merchant_D (gossip at market)
    │    Merchant_D forms social memory
    │    Merchant_D's behavior: higher prices for player, watches inventory
    │
    └──► Innkeeper_E (Merchant_A vents over drinks)
         Innkeeper_E forms social memory
         But Innkeeper_E has own positive memories of player
         CONFLICT → Innkeeper_E reflects: "Maybe there's more to the story"
         Innkeeper_E's behavior: treats player normally but asks about it
```

### 9.3 Memory Conflict Resolution

When an NPC receives conflicting information, MEMZ uses a **Bayesian-inspired belief update model**:

```python
def resolve_conflict(npc, new_info, existing_memories):
    """
    Bayesian-inspired belief update for NPCs receiving conflicting info.
    
    Based on: Tenenbaum et al. (2011) "How to Grow a Mind" — simplified 
    for real-time game performance (<0.1ms per resolution).
    """
    # --- Constants (tunable per server/config) ---
    BELIEF_THRESHOLD = 0.5      # Above this → accept info
    HYSTERESIS = 0.05           # Prevents belief flip-flopping on borderline info
    
    # --- 1. Prior: Direct experience vs hearsay ---
    direct_evidence = 0.0
    if npc.has_direct_experience(new_info.subject):
        # NPC personally witnessed something about this subject
        prior_weight = 0.8   # Strong prior from personal experience
        hearsay_weight = 0.2  # Discounted hearsay
        direct_evidence = npc.get_direct_sentiment(new_info.subject)  # -1.0 to 1.0
    else:
        prior_weight = 0.0
        hearsay_weight = 1.0
    
    # --- 2. Source credibility (Dunbar-informed trust network) ---
    source_trust = npc.get_trust(new_info.source)        # 0.0 to 1.0 (relationship quality)
    source_reliability = npc.get_track_record(new_info.source)  # past accuracy of claims
    credibility = source_trust * 0.6 + source_reliability * 0.4
    
    # --- 3. Consistency with existing belief network ---
    consistency = compute_consistency(new_info, existing_memories)
    # High (→1.0): new info aligns with what NPC already believes
    # Low  (→0.0): contradicts existing beliefs → skepticism
    
    # --- 4. Personality modulation (Big Five-inspired traits from Veloren rtsim) ---
    personality_bias = npc.personality.credulity  # 0.0 (skeptic) to 1.0 (gullible)
    openness = npc.personality.openness           # 0.0 (rigid) to 1.0 (flexible)
    
    # --- 5. Emotional state modifier (confirmation bias) ---
    emotional_bias = 0.0
    if npc.emotional_state_toward(new_info.subject) > 0.5:
        emotional_bias = -0.1  # Positive feelings → resistant to negative info
    elif npc.emotional_state_toward(new_info.subject) < -0.5:
        emotional_bias = 0.1   # Negative feelings → receptive to negative info
    
    # --- 6. Information freshness (temporal discounting) ---
    freshness = exp(-0.1 * days_since(new_info.timestamp))  # Recent info weighs more
    
    # --- 7. Information chain depth (first-hand > second-hand > rumor) ---
    chain_discount = 1.0 / (1.0 + new_info.propagation_depth)  # depth 0=direct, 1=second-hand...
    
    # --- 8. Final belief score (Bayesian posterior approximation) ---
    belief = (prior_weight * clamp(direct_evidence, 0.0, 1.0) + 
              hearsay_weight * credibility * consistency * freshness * chain_discount +
              personality_bias * openness * 0.15 +
              emotional_bias)
    
    belief = clamp(belief, 0.0, 1.0)
    
    # --- 9. Threshold with hysteresis (prevent belief flip-flopping) ---
    if belief > BELIEF_THRESHOLD + HYSTERESIS:
        return AcceptInfo(confidence=belief)
    elif belief < BELIEF_THRESHOLD - HYSTERESIS:
        return RejectInfo(confidence=1.0 - belief)
    else:
        return Uncertain(will_investigate=openness > 0.5)
        # Uncertain NPCs may seek more info or wait for corroborating evidence
```

---

## 10. Player Memory Injection System

### The "Memory Journal" UI

The player opens their **Memory Journal** — a special in-game item/UI — and can write entries:

```
┌─────────────────────────────────────────────┐
│           📖 MEMORY JOURNAL                 │
│                                             │
│  Write a memory from your past:             │
│  ┌─────────────────────────────────────┐    │
│  │ I grew up in a fishing village on   │    │
│  │ the northern coast. My mother was   │    │
│  │ a healer. She taught me to respect  │    │
│  │ all living things.                  │    │
│  └─────────────────────────────────────┘    │
│                                             │
│  Emotional tone: [Nostalgic ▼]              │
│  Time period:    [Childhood ▼]              │
│  Category:       [Family ▼]                 │
│                                             │
│  [✍️ Inscribe Memory]  [❌ Discard]         │
│                                             │
│  ─── Past Entries ───                       │
│  📝 "I was a soldier..." (Day 1)           │
│  📝 "My mother was a healer..." (Day 5)    │
│  📝 "I once saw a dragon..." (Day 12)      │
└─────────────────────────────────────────────┘
```

### How Injected Memories Affect the World

The MEMZ engine processes injected memories and creates **behavioral hooks**:

| Injected Memory | World Effect |
|----------------|--------------|
| "I was raised by wolves" | Wolves are less aggressive toward you. Druids sense your connection. Town NPCs find you "strange." |
| "I betrayed my king" | Noble NPCs sense your guilt. You're uncomfortable in castles. A bounty hunter NPC eventually appears. |
| "I can hear the dead speak" | Graveyard NPCs react to you. Priests are suspicious. You receive cryptic "whisper" events. |
| "I promised my dying father I'd find a cure" | Healer NPCs prioritize helping you. Quest-givers offer relevant quests. NPCs sense your urgency. |
| "I am secretly royalty in hiding" | Perceptive NPCs notice your posture/speech. Some NPCs treat you with unexplained deference. |

### Memory Injection Rules

1. **No game-breaking memories** — You can't inject "I am a god" and become invincible. Memories affect social dynamics, not stats.
2. **Memories must be personal** — About your character's past, relationships, traumas, dreams. Not "I know where the treasure is."
3. **NPCs detect inconsistency** — If you inject "I'm from the desert" but act like you know the forest, wise NPCs will notice.
4. **Memories unlock gradually** — Injected memories don't immediately affect everything. They create seeds that grow over time.
5. **Memory injection has a "cost"** — Limited slots, or requires rare in-game item, preventing spam.

---

## 11. The "Living Lore" Engine

### Emergent Storytelling

The most powerful feature of MEMZ is that it creates **emergent narratives** — stories that no developer wrote, arising from the interaction of memory-enabled characters.

**Example Emergent Narrative:**

```
Day 1:  Player arrives in town. Introduces self to NPCs.
Day 3:  Player helps defend town from bandit raid. Multiple NPCs witness heroism.
Day 5:  Town NPCs have positive memories. Blacksmith offers discount. 
        Guard invites player to patrol.
Day 7:  Player accidentally destroys a market stall during a fight.
Day 8:  Merchant NPC forms negative memory. Tells other merchants.
Day 10: Town is split. Some NPCs defend player (remember Day 3). 
        Others are angry (heard about Day 7).
Day 12: An NPC who wasn't present for either event arrives. 
        They hear BOTH stories from different NPCs.
        They form their own opinion based on the weight of evidence.
Day 15: Player injects memory: "I destroyed that stall because I saw a 
        thief hiding behind it."
Day 16: Player tells the story to the angry merchant. 
        Merchant's memory is updated: "Maybe I misjudged them."
Day 20: A bard NPC composes a song about "the hero who destroyed a stall 
        to catch a thief" — a slightly embellished version of events.
Day 25: New players arriving in town hear the bard's song. 
        The legend has begun.
```

### The Memory Web Visualization

Players (and streamers) can access a **Memory Web** — a visual graph showing how memories and opinions have spread through the NPC network:

```
        [Guard_A: trusts player]
              │
              ▼
    [Merchant_B: heard from Guard_A]
         │              │
         ▼              ▼
[Farmer_C: positive] [Baker_D: skeptical]
         │
         ▼
[Traveler_E: mixed - heard both sides]
```

This visualization is perfect for streaming — viewers can see the web of influence in real-time.

---

## 12. Technical Design

### 12.1 Technology Stack

| Component | Technology | Rationale |
|-----------|-----------|-----------|
| **Game Engine** | Veloren (Rust, ECS via `specs`) | Target game |
| **MEMZ Core Library** | Rust | Integrates directly into Veloren's ECS; memory-safe, high-performance, zero-cost abstractions |
| **Memory Store** | SQLite (embedded) via `rusqlite` + LRU in-memory cache | Per-world persistence, zero external deps, WAL mode for concurrent reads |
| **Vector Embeddings** | `fastembed-rs` (ONNX Runtime backend) | Local embedding generation; all-MiniLM-L6-v2 (80MB) for semantic retrieval; quantized INT8 option for low-end hardware |
| **Vector Index** | Custom HNSW via `instant-distance` or `usearch` (Rust bindings) | Sub-millisecond approximate nearest-neighbor search; memory-mapped for large NPC populations |
| **LLM Interface** | `ollama` (local) / OpenAI-compatible API / `llama.cpp` via `llama-cpp-rs` | Reflection, dialogue generation, memory synthesis; structured output via JSON mode / grammar constraints |
| **Serialization** | `serde` + `bincode` (binary) / `rmp-serde` (MessagePack) | Efficient memory state serialization; bincode for hot path, MessagePack for cross-platform save files |
| **Async Runtime** | `tokio` (multi-threaded) | Non-blocking LLM calls, I/O-bound memory persistence, background reflection scheduling |
| **Config** | TOML via `toml` crate | Standard Rust configuration; hot-reloadable for development |
| **Profiling & Metrics** | `tracing` + `tracing-subscriber` + custom game metrics | Real-time frame-budget monitoring; per-system timing; exportable to Tracy profiler |
| **Testing** | `proptest` (property-based) + `criterion` (benchmarks) + integration harness | Correctness guarantees on memory operations; regression benchmarks in CI |

### 12.2 ECS Integration

Veloren uses an Entity Component System. MEMZ adds new components:

```rust
/// Core memory component attached to any entity with memory capability
#[derive(Component)]
pub struct MemoryBank {
    pub episodic: Vec<EpisodicMemory>,
    pub semantic: Vec<SemanticMemory>,
    pub emotional: HashMap<EntityId, EmotionalState>,  // PAD model (Pleasure-Arousal-Dominance)
    pub social: Vec<SocialMemory>,
    pub reflective: Vec<ReflectiveMemory>,
    pub procedural: Vec<ProceduralMemory>,   // skills, habits, routines (§8.7)
    pub injected: Vec<InjectedMemory>,       // player only
    pub config: MemoryConfig,
    pub stats: MemoryStats,                  // runtime perf tracking
    pub consolidation_queue: VecDeque<ConsolidationTask>,  // pending memory transformations (§8.8)
}

/// Configuration per-character for memory behavior
pub struct MemoryConfig {
    pub max_episodic: usize,       // Memory capacity (default: 200)
    pub max_semantic: usize,       // Distilled knowledge cap (default: 50)
    pub max_social: usize,         // Gossip/rumor cap (default: 100)
    pub decay_rate: f32,           // How fast memories fade (0.01–0.10/day)
    pub reflection_interval: f32,  // How often to reflect (game-minutes)
    pub gossip_tendency: f32,      // How likely to share info (0.0–1.0)
    pub credulity: f32,            // How easily they believe rumors (0.0–1.0)
    pub emotional_volatility: f32, // How strongly events affect them (0.0–1.0)
    pub observation_radius: f32,   // How far NPC can perceive events (world units)
}

/// Runtime performance statistics per memory bank
pub struct MemoryStats {
    pub last_retrieval_us: u32,    // Microseconds for last retrieval
    pub total_memories: u32,       // Count across all types
    pub pending_reflections: u16,  // Queued reflection jobs
    pub last_decay_tick: u64,      // Game tick of last decay pass
}

/// System that runs every game tick to process memory
pub struct MemorySystem;

impl<'a> System<'a> for MemorySystem {
    type SystemData = (
        ReadStorage<'a, MemoryBank>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, NpcState>,
        Read<'a, GameEvents>,
        Write<'a, MemoryStore>,
        Read<'a, DeltaTime>,
    );
    
    fn run(&mut self, data: Self::SystemData) {
        // 1. Observe nearby events → create episodic memories (budget: 0.1ms)
        // 2. Decay old memories — Ebbinghaus curve (budget: 0.05ms)
        // 3. Periodic reflection → create semantic/reflective memories (async, non-blocking)
        // 4. Social propagation → share memories during NPC interactions (budget: 0.3ms)
        // 5. Behavior modification → adjust NPC actions based on memories (budget: 0.2ms)
        // 6. Compact/evict if memory count exceeds limits (budget: 0.1ms)
        // 
        // TOTAL BUDGET: <1.0ms per tick for active NPCs in loaded chunks
        // NPCs outside loaded chunks: frozen, zero cost
    }
}
```

### 12.2.1 Memory Lifecycle & Garbage Collection

Unbounded memory growth is the #1 risk for a persistent memory system. MEMZ uses a **multi-tier eviction strategy** inspired by CPU cache hierarchies:

```
Memory Lifecycle:
┌──────────────┐     ┌──────────────┐     ┌──────────────┐     ┌──────────┐
│   Hot Ring    │────▶│  Warm Ring   │────▶│  Cold Ring   │────▶│  Archive │
│  (in-memory)  │     │  (in-memory)  │     │  (SQLite)    │     │  (evict) │
│  Last 24h     │     │  Last 7 days  │     │  Last 90 days│     │  Forget  │
│  game-time    │     │  game-time    │     │  game-time   │     │          │
└──────────────┘     └──────────────┘     └──────────────┘     └──────────┘
     Fast access          Medium access        Slow access         Gone

Eviction Priority (lowest priority evicted first):
  Score = Importance × EmotionalWeight × (1 / TimeSinceLastAccess)
  
  Protected memories (never evicted):
  - Injected player memories
  - Memories with emotional_valence > 0.8 or < -0.8
  - First-meeting memories (initial impression)
  - Memories referenced by active quests
```

### 12.3 LLM Integration — Tiered Approach

Not every memory operation needs an LLM. We use a tiered approach to keep costs/latency manageable:

```
Tier 0: Rule-Based (No LLM) — 90% of operations          [Latency: <0.1ms]
├── Memory creation from game events (template-based)
├── Memory decay calculations (Ebbinghaus exponential curve)
├── Emotional valence updates (formula-based, PAD model)
├── Basic retrieval (embedding similarity via HNSW index)
├── Simple behavior modifications (threshold-based)
└── Reputation score aggregation (weighted average)

Tier 1: Small Local Model (1B–3B params) — 9% of operations  [Latency: 50–200ms, async]
├── Memory summarization (episodic → semantic distillation)
├── Gossip generation (NPC tells another NPC about event)
├── Simple dialogue variations (template + fill)
├── Memory conflict resolution
├── Recommended models: Phi-3-mini (3.8B), Qwen2.5-1.5B, TinyLlama-1.1B
└── Quantization: Q4_K_M via llama.cpp for <2GB VRAM

Tier 2: Large Model (7B+ local or cloud API) — 1% of operations  [Latency: 200ms–2s, async]
├── Deep reflection ("What do I really think about this person?")
├── Complex dialogue generation (multi-turn, context-rich)
├── Emergent story synthesis (bard composing songs, etc.)
├── Player memory injection processing & validation
├── Novel behavioral responses to unprecedented situations
├── Recommended models: Mistral-7B, Llama-3.1-8B, or cloud (GPT-4o-mini, Claude Haiku)
└── Quantization: Q5_K_M for local, full precision for cloud

Tier 3: Structured Output (Grammar-Constrained) — All LLM calls
├── All LLM outputs use JSON mode or GBNF grammars
├── Guaranteed parseable output — no regex hacks, no "hope it works"
├── Schema-validated before ingestion into memory store
└── Failed validations → automatic retry with simplified prompt → rule-based fallback
```

**Key insight:** 99% of memory operations DON'T need an LLM. Embeddings, vector search, and rule-based systems handle the bulk. The LLM is reserved for the moments that matter — making them feel truly magical.

**Graceful Degradation Chain:**
```
LLM Available → Rich, contextual, surprising NPC responses
LLM Unavailable → Template-based responses referencing stored memories  
LLM + Embeddings Unavailable → Keyword-match retrieval + templates
Everything Offline → Static personality-based responses (game still fully playable)
```

### 12.3.1 Prompt Engineering — Reproducible, Versioned, Tested

Every LLM prompt in MEMZ is a **versioned artifact**, stored as template files, tested in CI, and evaluated against a quality benchmark. No ad-hoc string concatenation.

**Prompt Architecture:**
```
memz-llm/prompts/
├── v1/
│   ├── reflection.toml          # Deep reflection prompt
│   ├── dialogue_simple.toml     # Single-turn NPC dialogue
│   ├── dialogue_complex.toml    # Multi-turn with memory context
│   ├── gossip_generation.toml   # NPC-to-NPC information sharing
│   ├── memory_summary.toml      # Episodic → Semantic distillation
│   ├── bard_composition.toml    # Song/poem generation
│   ├── memory_replay.toml       # First-person memory narration
│   └── injection_validation.toml # Player memory safety check
├── grammars/
│   ├── dialogue_response.gbnf   # Structured dialogue output
│   ├── reflection_output.gbnf   # Structured reflection JSON
│   ├── gossip_output.gbnf       # Structured gossip JSON
│   └── bard_poem.gbnf           # Verse structure (AABB/ABAB)
└── eval/
    ├── golden_set.json          # 100 prompt→expected_output pairs
    └── eval_rubric.toml         # Scoring criteria for LLM-as-judge
```

**Example Prompt — NPC Dialogue (dialogue_simple.toml):**
```toml
[prompt]
version = "1.0"
tier = 1  # Small model sufficient
max_tokens = 150
temperature = 0.7
grammar = "dialogue_response.gbnf"

system = """
You are {npc_name}, a {npc_profession} in {settlement_name}.
Your personality: {personality_description}.
Your current emotional state: {pad_state} (Pleasure: {pleasure}, Arousal: {arousal}, Dominance: {dominance}).

RULES:
- Stay in character. Never break the fourth wall.
- Reference memories naturally — don't list them.
- Keep responses under 3 sentences.
- If you don't remember the player, say so honestly.
- Your response must be valid JSON matching the grammar.
"""

user = """
Context: {context_description}
Player action: {player_action}

Your relevant memories (ranked by importance):
{%- for mem in top_memories %}
- [{mem.type}] {mem.summary} (strength: {mem.strength}, age: {mem.age_days} days)
{%- endfor %}

Your current opinion of this player: {overall_sentiment} (confidence: {confidence})

Respond as {npc_name} would. Return JSON:
{{"dialogue": "your response", "emotion_shift": float, "new_memory": "what you'll remember about this"}}
"""
```

**Example GBNF Grammar — Dialogue Response (dialogue_response.gbnf):**
```
root   ::= "{" ws "\"dialogue\"" ws ":" ws string "," ws "\"emotion_shift\"" ws ":" ws number "," ws "\"new_memory\"" ws ":" ws string "}" ws
string ::= "\"" ([^"\\] | "\\" .)* "\""
number ::= "-"? [0-1] ("." [0-9]{1,2})?
ws     ::= [ \t\n]*
```

**Example Prompt — Deep Reflection (reflection.toml):**
```toml
[prompt]
version = "1.0"
tier = 2  # Needs larger model for nuanced reasoning
max_tokens = 300
temperature = 0.8
grammar = "reflection_output.gbnf"

system = """
You are the inner mind of {npc_name}, a {npc_profession}.
You are reflecting on your recent experiences during a quiet moment.
Think deeply. Consider patterns. Form opinions. Wonder about things.
You are NOT speaking to anyone — this is your private thought.
"""

user = """
Your recent episodic memories (last {time_window}):
{%- for mem in recent_episodic %}
- {mem.timestamp}: {mem.description} [emotional impact: {mem.valence}]
{%- endfor %}

Your existing beliefs and knowledge:
{%- for sem in semantic_memories %}
- {sem.summary}
{%- endfor %}

Your personality traits: {personality_summary}

Based on these experiences, what do you think? What patterns do you notice?
What has changed in your view of the world or the people around you?

Return JSON:
{{"reflection": "your inner thought", "new_beliefs": ["belief1", ...], "questions": ["thing you wonder about", ...], "mood_shift": {{"pleasure": float, "arousal": float, "dominance": float}}}}
"""
```

**Prompt Quality Assurance:**
| Check | Method | CI Gate? |
|-------|--------|----------|
| Prompt renders without errors | Template engine unit test | ✅ Yes |
| Output parses against GBNF grammar | Schema validation test | ✅ Yes |
| Golden set: output quality ≥ 4.0/5.0 | LLM-as-judge (GPT-4o evaluates Tier 1 outputs) | ⚠️ Nightly |
| No prompt injection leaks | Adversarial input test suite (50 attack patterns) | ✅ Yes |
| Token count within budget | Token counter assertion | ✅ Yes |
| Personality consistency | Same NPC, different contexts → coherent personality | ⚠️ Weekly eval |

### 12.4 Memory Retrieval Algorithm

When an NPC needs to respond to a situation, the retrieval system finds relevant memories:

```
Input: Current context (who's present, location, topic, action)

Retrieval Score = w₁ · Recency(m) + w₂ · Relevance(m) + w₃ · Importance(m) + w₄ · Emotional(m) + w₅ · Social(m)

Where:
  Recency(m)    = exp(-λ · ΔT) where ΔT = game-time since memory, λ = decay constant
                  (Ebbinghaus forgetting curve: R = e^(-t/S) where S = memory strength)
  Relevance(m)  = cosine_similarity(context_embedding, memory_embedding)
                  computed via HNSW index in <0.5ms for 1000+ memories
  Importance(m) = pre-computed importance score (0-1), boosted by:
                  novelty (dissimilarity to existing memories), emotional intensity,
                  relevance to NPC's goals/profession
  Emotional(m)  = |emotional_valence| × emotional_volatility
                  (strong emotions are more memorable — flashbulb memory effect)
  Social(m)     = trust_in_source × recency_of_social_transmission
                  (first-hand > second-hand > rumor, with Dunbar-informed decay)

Weights (tunable per-personality archetype):
  w₁ = 0.20 (recency)
  w₂ = 0.30 (relevance)  ← most important: is this memory ABOUT the current situation?
  w₃ = 0.20 (importance)
  w₄ = 0.20 (emotional)
  w₅ = 0.10 (social)     ← new: weights heard information by source trust

Top-K memories (K=5, configurable) are retrieved and fed to dialogue/behavior system.

Performance guarantee: 
  - HNSW search: O(log n) for n memories, <0.3ms for 500 memories
  - Scoring + ranking: O(K log K), <0.1ms
  - Total retrieval: <0.5ms per NPC interaction
```

This is directly inspired by the Stanford Generative Agents retrieval function but enhanced with:
- **Ebbinghaus-curve decay** (scientifically grounded, not arbitrary linear decay)
- **Emotional flashbulb effect** (traumatic/joyful memories resist decay)
- **Social propagation scoring** (trust-weighted hearsay)
- **HNSW indexing** (orders of magnitude faster than brute-force cosine similarity)
- **Personality-modulated weights** (a sentimental NPC weighs emotion higher; a logical NPC weighs relevance higher)

### 12.5 Persistence & Serialization

```
World Save Structure:
world_save/
├── world_data.bin          (Veloren's existing save)
├── memz/
│   ├── memory_index.db     (SQLite: memory metadata, relationships)
│   ├── embeddings.bin      (Vector embeddings for all memories)
│   ├── character_states/
│   │   ├── npc_001.memz    (Individual NPC memory banks)
│   │   ├── npc_002.memz
│   │   ├── ...
│   │   └── player_001.memz (Player character memories)
│   ├── social_graph.bin    (NPC relationship/gossip network)
│   └── config.toml         (Memory system configuration)
```

### 12.6 Performance Budget

**Hard Constraint:** MEMZ must NEVER cause a frame drop below 60 FPS. The memory system is budgeted at **< 2ms per frame** on a mid-range PC (Ryzen 5 5600X / RTX 3060 / 16GB RAM).

| Operation | Budget | Frequency | P99 Target |
|-----------|--------|-----------|------------|
| Event observation & memory creation | 0.1ms | Every frame (for NPCs in player's area) | 0.15ms |
| Memory decay (Ebbinghaus curve) | 0.05ms | Every 60 frames (1 sec) | 0.08ms |
| Memory retrieval (for dialogue) | 0.5ms | On interaction trigger | 0.8ms |
| HNSW index update | 0.02ms | Per new memory | 0.05ms |
| Reflection (LLM call) | Async, non-blocking | Every 5 min game time | N/A (off main thread) |
| Memory propagation (gossip) | 0.3ms | When NPCs interact | 0.5ms |
| Serialization (save) | 1-5ms | On save (every 5 min) | 8ms |
| Embedding generation | Async, non-blocking | Per new memory (batched) | N/A (off main thread) |

**Key optimizations:**
- **Chunk-based activation:** Only NPCs within the player's loaded chunks have active memory processing. Distant NPCs' memories are "frozen" and loaded on demand.
- **Batched embedding generation:** New memories are queued and embedded in batches every 500ms off the main thread.
- **LRU cache for embeddings:** The 1000 most recently accessed memory embeddings are cached in-memory, avoiding disk I/O.
- **Incremental HNSW updates:** New memories are inserted into the index without full rebuild. Periodic background compaction.
- **Memory pooling:** Pre-allocated memory pools for EpisodicMemory structs to avoid allocation jitter.
- **Lazy loading:** NPC memory banks are loaded from SQLite only when the NPC enters the player's perception radius.

**Benchmarking Requirements (CI-enforced):**
```
cargo bench --bench memory_system -- --output-format=bencher

Benchmark targets (must pass in CI):
  memory_creation_single ........... < 10μs
  memory_retrieval_top5_from_200 ... < 500μs  
  memory_decay_pass_50_npcs ....... < 50μs
  hnsw_search_500_memories ........ < 300μs
  memory_serialization_100 ........ < 2ms
  full_frame_budget_20_active_npcs . < 2ms
```

### 12.7 Hardware Requirements & Scaling Profiles

| Profile | Hardware | LLM Capability | Memory Experience |
|---------|----------|---------------|-------------------|
| **Ultra-Low** | 4-core CPU, 8GB RAM, no GPU | No LLM, no embeddings | Rule-based only. NPCs still remember via keyword matching. Fully playable. |
| **Low** | 6-core CPU, 16GB RAM, iGPU | Embeddings only (fastembed, 80MB) | Vector-based retrieval. Rich memory recall. No LLM-generated dialogue. |
| **Medium (Target)** | Ryzen 5 / i5, 16GB RAM, RTX 3060 | Tier 0+1 (local 1-3B model via Ollama) | Full memory system with local LLM for summarization and simple dialogue. |
| **High** | Ryzen 7 / i7, 32GB RAM, RTX 4070+ | Tier 0+1+2 (local 7B model) | Full memory system with deep reflection and complex dialogue, all local. |
| **Cloud-Assisted** | Any hardware + API key | Tier 0+1 local, Tier 2 cloud | Best experience: local for speed, cloud for depth. <$0.50/hour API cost. |

**Auto-Detection:** MEMZ detects available hardware at startup and automatically selects the appropriate profile. Users can override in `config.toml`.

### 12.8 Multiplayer Architecture

```
Multiplayer Memory Architecture (Server-Authoritative):

┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│   Player A   │     │   Player B   │     │   Player C   │
│   (Client)   │     │   (Client)   │     │   (Client)   │
└──────┬───────┘     └──────┬───────┘     └──────┬───────┘
       │ Actions/Events     │                     │
       ▼                    ▼                     ▼
┌─────────────────────────────────────────────────────────┐
│                    GAME SERVER                           │
│  ┌───────────────────────────────────────────────────┐  │
│  │              MEMZ SERVER ENGINE                    │  │
│  │                                                   │  │
│  │  NPC_1.memories = {                               │  │
│  │    player_A_stream: [...],   // A's interactions   │  │
│  │    player_B_stream: [...],   // B's interactions   │  │
│  │    world_stream: [...],      // Non-player events  │  │
│  │    social_stream: [...],     // NPC-NPC gossip     │  │
│  │  }                                                │  │
│  │                                                   │  │
│  │  On interaction(NPC_1, Player_A):                 │  │
│  │    context = merge(player_A_stream, world_stream,  │  │
│  │                    social_stream)                   │  │
│  │    retrieve_top_k(context) → dialogue             │  │
│  │                                                   │  │
│  │  Cross-player awareness:                          │  │
│  │    "Ah, you two again! Last time you were here,   │  │
│  │     one of you was kind and the other stole       │  │
│  │     my apples."                                   │  │
│  └───────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────┐  │
│  │  LLM Queue (Priority-Based):                      │  │
│  │  1. Active dialogue (player waiting) — immediate   │  │
│  │  2. Memory injection processing — within 5 sec     │  │
│  │  3. NPC reflection — background, minutes           │  │
│  │  4. Gossip generation — background, lazy           │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘

Consistency Model: Eventual consistency for gossip/reputation.
Strong consistency for direct player interactions.
```

---

## 13. Differentiation — Why MEMZ ≠ Mantella ≠ Generative Agents

```
┌──────────────────────┬──────────────┬──────────────┬──────────────┬──────────────┬──────────────┐
│ Feature              │ Generative   │ Mantella     │ Lyfe Agents  │ Inworld AI   │ MEMZ         │
│                      │ Agents       │              │              │ (Commercial) │              │
├──────────────────────┼──────────────┼──────────────┼──────────────┼──────────────┼──────────────┤
│ Real game?           │ ❌ Demo only │ ✅ Skyrim    │ ❌ Demo only │ ✅ AAA games │ ✅ Veloren   │
│ Open source game?    │ N/A          │ ❌ Prop.     │ ⚠️ Partial   │ ❌ Prop.     │ ✅ GPL-3.0   │
│ Open source engine?  │ ✅ Apache    │ ✅ GPL-3.0   │ ⚠️ Partial   │ ❌ Closed    │ ✅ GPL-3.0   │
│ Memory architecture  │ ✅ Novel     │ ⚠️ Basic     │ ✅ Novel     │ ⚠️ Unknown   │ ✅ Advanced  │
│ Episodic memory      │ ✅           │ ⚠️ Conv.hist │ ✅           │ ⚠️ Partial   │ ✅           │
│ Semantic memory      │ ⚠️ Partial   │ ❌           │ ⚠️ Summ.only │ ⚠️ Unknown   │ ✅           │
│ Emotional memory     │ ❌           │ ❌           │ ❌           │ ⚠️ Basic     │ ✅ PAD model │
│ Social propagation   │ ✅           │ ❌           │ ✅ Basic     │ ❌           │ ✅ Deep+Trust│
│ Player mem injection │ ❌           │ ❌           │ ❌           │ ❌           │ ✅ Core feat │
│ Memory visualization │ ❌           │ ❌           │ ❌           │ ❌           │ ✅ Memory Web│
│ NPC-NPC gossip       │ ✅ Basic     │ ✅ Basic     │ ✅           │ ❌           │ ✅ Deep+Decay│
│ Memory decay model   │ ⚠️ Implicit  │ ❌           │ ✅ Summ.&Fgt │ ⚠️ Unknown   │ ✅ Ebbinghaus│
│ Voice support        │ ❌           │ ✅ Core      │ ❌           │ ✅ Core      │ ⚠️ Optional  │
│ Runs fully offline   │ ❌           │ ⚠️ Possible  │ ❌           │ ❌ Cloud-only│ ✅ Primary   │
│ Multiplayer          │ ❌           │ ❌           │ ❌           │ ⚠️ Partial   │ ✅ Native    │
│ Reusable library     │ ❌           │ ❌           │ ❌           │ ❌ Prop.     │ ✅ Crate     │
│ Combat/quest impact  │ ❌           │ ⚠️ Limited   │ ❌           │ ⚠️ Limited   │ ✅           │
│ Mod-friendly         │ N/A          │ ⚠️           │ N/A          │ ❌           │ ✅           │
│ Performance budget   │ ❌ Heavy     │ ⚠️           │ ✅ 10-100x   │ ⚠️ Cloud lat │ ✅ <2ms/frame│
│ Structured LLM out   │ ❌           │ ❌           │ ❌           │ ⚠️ Unknown   │ ✅ GBNF/JSON │
│ Graceful degradation │ ❌           │ ⚠️ Basic     │ ⚠️           │ ❌           │ ✅ 4-tier    │
│ CI benchmarks        │ ❌           │ ❌           │ ❌           │ N/A          │ ✅ Criterion │
│ Free to use          │ ✅           │ ✅           │ ⚠️           │ ❌ $$$       │ ✅ Forever   │
└──────────────────────┴──────────────┴──────────────┴──────────────┴──────────────┴──────────────┘

MEMZ's unique value: First system that combines deep cognitive-science-grounded 
memory architecture with a playable open-source game, player memory injection, 
trust-weighted social memory networks, offline-first design, multiplayer support,
grammar-constrained LLM outputs, and CI-enforced performance benchmarks.
No commercial or open-source project has this combination. Period.
```

---

## 14. Viral Mechanics — Built for Shareability

### 14.0 The "First 5 Minutes" Experience — Hook or Lose Them

The single most important factor for virality is what happens in the **first five minutes** of a new player's experience. If this doesn't blow their mind, nothing else matters. MEMZ is designed to deliver a "wow" moment within the first interaction.

**Minute 0-1 — Arrival:**
- Player spawns in a starter village. Normal game experience. Nothing unusual yet.
- But: the NPC nearest the spawn point has been **pre-seeded** with a low-strength procedural memory: *"I think I've seen someone like you before... or was that a dream?"*
- This seeds curiosity without breaking immersion. The NPC isn't sure — it's a fuzzy memory.

**Minute 1-3 — First Interaction:**
- Player talks to a shopkeeper. Normal dialogue options, but...
- If the player buys something, the shopkeeper says a standard line. **The magic happens on the second visit.**
- MEMZ ensures the very next NPC interaction references the first one. Even if it's just:
  > *"You again! I remember you — you bought that torch earlier. Heading somewhere dark?"*
- This is the **"wait, did the NPC just remember me?"** moment. This is where screenshots happen. This is where streamers react. This is where Twitter posts are born.

**Minute 3-5 — The Hook Deepens:**
- Player interacts with a second NPC who **references information from the first NPC** via the gossip system:
  > *"The shopkeeper mentioned a newcomer bought supplies for a journey. Are you the one heading to the ruins?"*
- Player realizes: **NPCs are talking about them**. The world is watching. The world remembers.
- Optional: A subtle UI notification appears: *"📝 The town is forming opinions about you."*

**Design Requirements for the First 5 Minutes:**
| Requirement | Implementation | Why |
|------------|----------------|-----|
| Guaranteed memorable NPC within 30 seconds of spawn | Pre-seeded "fuzzy memory" NPC near spawn | First impression must surprise |
| Second interaction must reference first | Priority memory retrieval for new players (boost recency weight to 1.0 for first hour) | "Wait, it remembered!" moment |
| Cross-NPC awareness within 5 minutes | Accelerated gossip propagation in starter area (5x speed) | "They're talking about me!" realization |
| No loading screens or lag | All first-5-min NPCs use Tier 0 (rule-based) responses only | Performance cannot break immersion |
| Streamer-ready from frame one | First NPC reactions are quotable, screenshot-worthy | Content creation starts immediately |

**Anti-Patterns to Avoid:**
- ❌ Generic "Welcome, adventurer!" greetings (every game does this — instant boredom)
- ❌ Memory features that only show up after hours of play (too late — player has already churned)
- ❌ Obvious "AI demo" feel (NPCs shouldn't monologue about their memory system)
- ❌ Any latency spike during the first interaction (use only cached/rule-based for minute 0-5)

### 14.1 The "Memory Replay" Feature

Players can rewatch key memory moments from NPCs' perspectives — seeing how the NPC perceived an event:

> **Your perspective:** "I walked into the tavern and ordered a drink."  
> **Innkeeper's memory:** "A travel-worn stranger entered. They looked exhausted. I noticed a sword wound on their arm they were trying to hide. I served them extra bread."

This is content gold for YouTube/TikTok.

**Technical Design:**
```rust
/// Memory Replay generates a narrative "re-telling" of a stored memory
/// from the NPC's emotional and personality perspective.
struct MemoryReplay {
    /// The original memory being replayed
    source_memory: MemoryId,
    /// NPC's emotional state at the time of the event
    emotional_context: PADState,
    /// NPC's personality filters (affects narrative tone)
    personality_lens: PersonalityTraits,
    /// Generated narrative text (cached after first generation)
    narrative: Option<String>,
    /// Accuracy drift: how much the memory has "changed" over time
    /// (older memories become less accurate — just like human recall)
    drift_factor: f32,  // 0.0 = perfect recall, 1.0 = heavily embellished
}

impl MemoryReplay {
    /// Generate narrative, with drift applied. Older/more emotional
    /// memories are more likely to be exaggerated or reinterpreted.
    fn generate_narrative(&self, llm: &LlmClient) -> String {
        let accuracy = 1.0 - self.drift_factor;
        // Prompt includes: original memory, emotional context, personality,
        // and instruction to embellish proportional to drift_factor
        llm.generate_with_grammar(
            &REPLAY_PROMPT_TEMPLATE,
            &ReplayContext { accuracy, ..self.into() },
            &REPLAY_GRAMMAR,  // GBNF: ensures output is a narrative paragraph
        )
    }
}
```

### 14.2 The "Reputation Board"

Every town has a bulletin board where you can see what NPCs are saying about you (anonymized):

```
┌─────────────────────────────────────────┐
│        🏛️ RIVERSTEAD NOTICE BOARD       │
│                                         │
│ "A hooded stranger helped the farmers   │
│  during the wolf attack. We are in      │
│  their debt." — Anonymous               │
│                                         │
│ "Someone has been stealing from the     │
│  eastern market stalls. Be vigilant."   │
│  — Market Association                   │
│                                         │
│ "The adventurer's tales grow taller     │
│  with each telling. I wonder what's     │
│  true." — A Skeptic                     │
└─────────────────────────────────────────┘
```

**Reputation Board Data Model:**
```rust
/// Server-side reputation aggregation for a settlement
struct ReputationBoard {
    settlement_id: SettlementId,
    entries: Vec<ReputationEntry>,
    last_updated: GameTimestamp,
    max_entries: usize,  // configurable, default 10
}

struct ReputationEntry {
    /// What the entry says (generated via Tier 1 LLM or rule-based template)
    text: String,
    /// Anonymous attribution (NPC role, not name — adds mystery)
    attribution: String,  // e.g., "A Merchant", "The Night Watch"
    /// Aggregated from how many NPC memories?
    source_count: u32,
    /// Sentiment of the entry (-1.0 to 1.0)
    sentiment: f32,
    /// When this entry was posted (game time)
    posted_at: GameTimestamp,
    /// How many NPCs in this settlement agree with this sentiment?
    agreement_ratio: f32,  // 0.0 to 1.0
}

impl ReputationBoard {
    /// Refreshes the board by aggregating recent NPC social memories about
    /// the player. Runs once per in-game day per settlement.
    /// Performance: < 1ms for settlements with < 50 NPCs.
    fn refresh(&mut self, npcs: &[NpcMemoryBank], player_id: PlayerId) {
        // 1. Collect all social memories about this player from settlement NPCs
        // 2. Cluster by topic (trade, combat, crime, kindness, etc.)
        // 3. For each cluster: pick the most representative memory
        // 4. Generate anonymized entry text (rule-based for perf)
        // 5. Sort by recency and source_count (most-discussed first)
        // 6. Truncate to max_entries
    }
}
```

### 14.3 The "Memory Conflict" Events

When NPCs with conflicting memories about the player meet, they argue — in front of the player:

> **Guard:** "This one is a hero! They saved three villagers from the bandits!"  
> **Merchant:** "Hero?! They stole from my stall not two days ago!"  
> **Guard:** "I find that hard to believe. I've seen nothing but honor from them."  
> **Merchant:** "Then you weren't looking closely enough."  
> *[Player can intervene or let it play out]*

### 14.4 The "NPC Diary" Easter Egg

Rare item drop: an NPC's personal diary. Reading it reveals THEIR memories of events — from their perspective. Players will share these online.

### 14.5 Twitch Integration (Stretch Goal)

- Viewers can vote to inject memories into the streamer's character
- "Chat chose to inject: 'You were once a pirate captain' — watch the world react!"
- NPC reactions appear in real-time

---

## 15. Phased Roadmap

### Phase 0: Foundation (Weeks 1-4)

```
Goals:
├── Fork Veloren, set up development environment
├── Study rtsim codebase in depth (document all NPC hooks)
├── Design MEMZ ECS components (MemoryBank, MemoryConfig, MemoryStats)
├── Implement basic MemoryBank component
├── Implement event observation system (game events → episodic memories)
├── Basic memory persistence (SQLite WAL mode, save/load)
├── Simple retrieval (recency-based, no embeddings yet)
├── Set up CI pipeline: cargo test, cargo clippy, cargo bench (criterion)
├── Set up Tracy profiler integration for frame budget monitoring
└── Property-based tests (proptest) for memory creation/retrieval invariants

Quality Gate — Phase 0 Exit Criteria:
  ☐ NPCs form and retain episodic memories across save/load cycles
  ☐ All CI checks pass (tests, clippy, benchmarks)
  ☐ Memory creation < 10μs per event (benchmark verified)
  ☐ Save/load round-trip preserves 100% of memory data (fuzz tested)
  ☐ Zero game crashes attributable to MEMZ after 1-hour soak test

Deliverable: NPCs form and retain episodic memories. "The NPC remembers 
you were here yesterday."
```

### Phase 1: Memory Intelligence (Weeks 5-10)

```
Goals:
├── Implement embedding-based retrieval (fastembed-rs + HNSW index)
├── Add semantic memory formation (LLM reflection via Tier 1 model)
├── Add emotional memory layer (PAD emotional model)
├── Implement memory decay (Ebbinghaus forgetting curve)
├── Basic dialogue system that references memories
├── Memory-aware NPC behavior (greeting changes, price adjustments)
├── Structured LLM output (GBNF grammars / JSON mode)
├── Graceful degradation chain (LLM → template → keyword → static)
├── Unit tests, property-based tests, criterion benchmarks
└── First playtest session (internal, 5 testers, 2-hour sessions)

Quality Gate — Phase 1 Exit Criteria:
  ☐ Memory retrieval < 0.5ms P50 for 200 memories (benchmark verified)
  ☐ LLM output parse success rate > 99% over 1000 test prompts
  ☐ Graceful degradation works: game fully playable with LLM disabled
  ☐ Emotional valence correctly tracks positive/negative interactions
  ☐ Memory decay follows Ebbinghaus curve (validated by unit tests)
  ☐ Playtest: >70% of testers report NPC felt like it "remembered" them

Deliverable: NPCs remember, feel, and respond differently based on history.
"The blacksmith gives you a discount because you helped her last week."
```

### Phase 2: Social Memory Network (Weeks 11-16)

```
Goals:
├── Implement NPC-to-NPC memory sharing
├── Gossip mechanics (NPCs share info when they interact)
├── Trust/credibility system for received information (Dunbar-informed)
├── Memory conflict detection and resolution
├── Reputation aggregation from network memories
├── Town-level "collective memory" (aggregated sentiment)
├── Memory web visualization (debug tool → player feature)
├── Multiplayer: per-player NPC memory streams
├── Load testing: 100+ NPCs with active memories, verify <2ms budget
└── Second playtest session (external, 20 testers, varied playstyles)

Quality Gate — Phase 2 Exit Criteria:
  ☐ Gossip propagates correctly: merchant→guard→other merchants in <5 game hours
  ☐ Trust weighting works: first-hand > second-hand > rumor (validated)
  ☐ Memory conflict resolution produces correct outcomes in 95%+ of test cases
  ☐ 100 NPCs with active memories: <2ms per frame (benchmark verified)
  ☐ Multiplayer: 4 concurrent players, NPCs maintain separate memory streams
  ☐ Playtest: >80% report "the world feels interconnected"

Deliverable: Full social memory network. "The whole town knows you're a 
hero / thief based on word-of-mouth."
```

### Phase 3: Player Memory Injection (Weeks 17-22)

```
Goals:
├── Design and implement Memory Journal UI
├── Memory injection processing pipeline (LLM-validated)
├── Behavioral hooks from injected memories
├── NPC response to player's implied backstory
├── Balance and gating (memory slot limits, in-game costs)
├── Content safety: local classifier for injection validation
├── Memory injection → world effect testing (automated + manual)
├── Edge case handling (contradictory memories, abuse prevention)
├── Procedural Memory system (§8.7): NPC skill learning, habits, routines
├── Memory Consolidation engine (§8.8): episodic→semantic, habit formation
├── Bard System (core feature): song composition, performance, spreading
├── Accessibility: screen reader support, keyboard navigation for Journal UI
└── Third playtest session (30 testers, focus on injection creativity + bard songs)

Quality Gate — Phase 3 Exit Criteria:
  ☐ Injected memories produce detectable world changes within 3 game-hours
  ☐ Content filter catches 99%+ of harmful injection attempts (test suite)
  ☐ Contradictory injections handled gracefully (NPC notices inconsistency)
  ☐ Memory Journal UI is fully keyboard-navigable and screen-reader compatible
  ☐ Procedural memory: NPC skill proficiency visibly improves over 50+ repetitions
  ☐ Bard system: generates coherent 4-8 line songs from memory input
  ☐ Memory consolidation: episodic→semantic transformation verified over 7-day sim
  ☐ Playtest: >75% report "my backstory made the world feel personal"

Deliverable: Players can write backstories. "You wrote that you fear fire. 
NPCs near campfires notice your discomfort."
```

### Phase 4: Polish & Viral Features (Weeks 23-28)

```
Goals:
├── Memory Replay system (view events from NPC perspective, with drift)
├── Reputation Board in-game feature (with data model, refresh system)
├── NPC Diary collectible items
├── Memory Conflict event scripting
├── "First 5 Minutes" experience tuning (§14.0): fuzzy seed NPC, starter gossip boost
├── Performance optimization sprint (hit ALL P99 targets)
├── Comprehensive configuration system (memz.toml, hot-reloadable)
├── "Streamer Mode" configuration preset
├── Documentation: README, architecture guide, API docs, tutorial
├── Demo videos (3-min trailer, 15-min deep dive, TikTok-length clips)
├── Promotional screenshots (curated "moment factory" gallery)
├── Upstream PR preparation (or standalone mod distribution)
├── Security audit: memory injection, save file integrity, multiplayer auth
└── Full regression test suite, 8-hour soak test, memory leak detection

Quality Gate — Phase 4 Exit Criteria:
  ☐ ALL P99 performance targets met (CI benchmark suite, zero regressions)
  ☐ 8-hour soak test: zero crashes, zero memory leaks, <30% RAM growth
  ☐ Save file corruption recovery works (inject corruption → verify recovery)
  ☐ Demo video produced and reviewed by 5 external viewers for "wow factor"
  ☐ Documentation reviewed by someone who hasn't seen the codebase
  ☐ Playtest: NPS > 50 (n=50 testers)

Deliverable: Release-ready. Beautiful, shareable, streamable.
```

### Phase 5: Community & Growth (Ongoing)

```
Goals:
├── Release on Veloren forums, Reddit, HN, X/Twitter, Mastodon
├── Create demo videos for YouTube/TikTok (moment compilations)
├── Reach out to gaming YouTubers / Twitch streamers (with press kit)
├── Twitch integration (stretch: chat-driven memory injection)
├── Community feedback incorporation (fast iteration cycles)
├── Extract MEMZ as standalone Rust crate (game-agnostic, crates.io)
├── Port feasibility study for other games (Luanti, CDDA, Space Station 14)
├── Research paper / blog series on the architecture
├── Conference talk submissions (GDC, SIGGRAPH, AI conferences, RustConf)
├── Establish "Memory Modders" community program for contributors
└── Monthly release cadence with changelog and community highlights
```

---

## 16. Community & Open Source Strategy

### Naming & Branding

| Element | Choice | Rationale |
|---------|--------|-----------|
| **Project Name** | MEMZ | Short, memorable, sounds like "memes" (viral), relates to memory |
| **Tagline** | "Every character remembers" | Clear, evocative, fits in a tweet |
| **Logo** | Brain + pixel art style | Matches Veloren's voxel aesthetic |
| **Repo Name** | `memz` or `memz-veloren` | Clean, searchable |

### Contribution Model

```
Repository Structure:
memz/
├── memz-core/          # Game-agnostic memory library (Rust crate, publishable to crates.io)
│   ├── memory/         # Memory types, storage, retrieval, decay (Ebbinghaus)
│   ├── retrieval/      # HNSW index, embedding search, scoring algorithm
│   ├── reflection/     # LLM-based reflection engine (structured output)
│   ├── social/         # Social memory propagation, trust network, gossip
│   ├── embedding/      # Vector embedding utilities (fastembed-rs, ONNX)
│   └── safety/         # Content filtering, injection validation, abuse prevention
│
├── memz-veloren/       # Veloren integration
│   ├── components/     # ECS components (MemoryBank, MemoryConfig, MemoryStats)
│   ├── systems/        # ECS systems (MemorySystem, ReflectionSystem, PropagationSystem)
│   ├── dialogue/       # Memory-driven dialogue generation
│   ├── ui/             # Memory Journal, Reputation Board, Memory Web overlay
│   ├── hooks/          # Game event hooks (combat, trade, quest, movement)
│   └── streamer/       # Streamer mode config, Twitch integration (stretch)
│
├── memz-llm/           # LLM abstraction layer
│   ├── ollama/         # Ollama integration
│   ├── openai/         # OpenAI-compatible API (also Anthropic, Together, etc.)
│   ├── local/          # llama.cpp direct integration via llama-cpp-rs
│   └── grammar/        # GBNF grammars and JSON schemas for structured output
│
├── memz-bench/         # Benchmark suite (criterion)
│   ├── benches/        # Per-component benchmarks
│   └── profiles/       # Hardware-specific baseline profiles
│
├── docs/               # Documentation, architecture, guides
│   ├── architecture.md # Deep dive into memory architecture
│   ├── getting-started.md
│   ├── configuration.md
│   ├── porting-guide.md # How to port MEMZ to another game
│   └── research.md     # Academic references, cognitive science basis
│
├── examples/           # Standalone examples and demos
├── tests/              # Integration tests, soak tests, fuzz tests
├── .github/            # CI workflows, issue templates, PR templates
└── CONTRIBUTING.md     # Contributor guide with coding standards
```

### Example Configuration (memz.toml)

A real, complete configuration file that ships with MEMZ. Every value is documented. Players/server operators can tune this without touching code.

```toml
# MEMZ Configuration — v2.0
# This file controls all aspects of the NPC memory system.
# Hot-reloadable: changes take effect without restart (except [hardware]).

[general]
enabled = true
log_level = "info"                    # trace, debug, info, warn, error
profile = "auto"                      # auto-detect hardware tier, or: "minimal", "standard", "high", "server", "dev"

[memory]
max_episodic_per_npc = 200            # Hard cap on episodic memories per NPC
max_semantic_per_npc = 50             # Distilled knowledge cap
max_social_per_npc = 100              # Gossip / hearsay cap
max_procedural_per_npc = 30           # Skills and routines cap
max_reflective_per_npc = 20           # Deep thoughts cap
decay_rate = 0.05                     # Base Ebbinghaus decay constant (per game-day)
consolidation_interval_days = 1       # How often memory consolidation runs (game-days)
consolidation_budget_ms = 0.1         # Max milliseconds per NPC per consolidation cycle

[memory.eviction]
hot_ring_hours = 24                   # In-memory: last 24 game-hours
warm_ring_days = 7                    # In-memory: last 7 game-days
cold_ring_days = 90                   # SQLite: last 90 game-days
protect_emotional_threshold = 0.8     # Memories with |valence| > this are never evicted
protect_first_meeting = true          # First-encounter memories are permanent

[retrieval]
algorithm = "hnsw"                    # "hnsw" (default), "brute_force" (debug), "tfidf" (fallback)
top_k = 5                             # Number of memories retrieved per interaction
embedding_model = "all-MiniLM-L6-v2"  # ONNX model for semantic embeddings
embedding_dimensions = 384
hnsw_ef_construction = 128            # HNSW build quality (higher = better, slower build)
hnsw_ef_search = 64                   # HNSW search quality (higher = better recall, slower query)
hnsw_m = 16                           # HNSW connections per node

[retrieval.weights]
recency = 0.20
relevance = 0.30
importance = 0.20
emotional = 0.20
social = 0.10

[llm]
provider = "ollama"                   # "ollama", "openai", "llama_cpp", "none"
base_url = "http://localhost:11434"   # Ollama default
tier1_model = "qwen2.5:1.5b"         # Small, fast, local
tier2_model = "mistral:7b-instruct"  # Large, deep reasoning
max_tier2_calls_per_hour = 20        # Cost/performance cap
request_timeout_ms = 5000            # Hard timeout for any LLM call
structured_output = true             # Enforce GBNF/JSON mode on all calls
retry_on_parse_failure = true        # Auto-retry with simplified prompt
max_retries = 2

[llm.fallback]
# What to do when LLM is unavailable
tier2_fallback = "tier1"             # Downgrade to smaller model
tier1_fallback = "templates"         # Use rule-based templates
templates_fallback = "silent"        # NPC uses body language only

[social]
gossip_tendency_default = 0.5         # 0.0 = secretive, 1.0 = town crier
gossip_propagation_speed = 1.0        # 1.0 = normal, 5.0 = starter area boost
trust_decay_rate = 0.01               # Trust erodes slowly without reinforcement
max_gossip_chain_depth = 4            # Info degrades after 4 hops (telephone game)

[first_five_minutes]
# Special tuning for the new player experience (§14.0)
enabled = true
starter_area_gossip_speed_multiplier = 5.0  # Gossip spreads 5x faster near spawn
recency_weight_boost_duration_hours = 1.0   # Boost recency weight for first hour
fuzzy_seed_npc_enabled = true               # Pre-seeded "I think I've seen you" NPC
guaranteed_recognition_on_second_visit = true

[performance]
frame_budget_ms = 2.0                 # Hard limit: MEMZ total per frame
memory_creation_budget_us = 10        # Max microseconds per memory creation
retrieval_budget_us = 500             # Max microseconds per retrieval query
active_npc_radius_chunks = 3          # Only process NPCs within this radius
max_concurrent_llm_requests = 2       # Prevent LLM queue flooding

[persistence]
backend = "sqlite"                    # "sqlite" (default), "json" (debug)
wal_mode = true                       # SQLite WAL for concurrent reads
auto_save_interval_seconds = 300      # Save every 5 minutes
backup_count = 3                      # Keep last 3 save backups
checksum_enabled = true               # Detect save corruption

[safety]
content_filter_enabled = true         # Filter player memory injections
injection_rate_limit_per_minute = 5   # Max injection attempts per minute
max_injection_length_chars = 500      # Prevent prompt stuffing
profanity_filter = "moderate"         # "off", "moderate", "strict"
log_moderation_events = true          # Audit trail for moderation

[accessibility]
screen_reader_support = true
high_contrast_ui = false
reduce_motion = false
text_size_multiplier = 1.0            # 1.0 = default, 1.5 = large, 2.0 = extra large
memory_journal_keyboard_only = true   # Full keyboard navigation

[telemetry]
enabled = false                       # Opt-in only
prometheus_endpoint = "127.0.0.1:9090"
export_tracy = true                   # Tracy profiler integration
log_slow_operations_ms = 5.0          # Log any operation exceeding this
```

### Community Engagement Plan — Full Launch Strategy

**Pre-Launch (Phase 0-1: Foundation & Core)**

| Week | Action | Channel | Success Metric |
|------|--------|---------|----------------|
| Week 1 | Join Veloren Discord + Zulip. Introduce project concept. Gather feedback. | Veloren community | 10+ positive reactions |
| Week 2 | Share early architecture doc. Ask for rtsim hook suggestions. | Veloren dev channel | 3+ developer responses |
| Week 3 | Post "Building an AI memory system for a real game" (dev log #1) | r/gamedev, r/rust | 100+ upvotes |
| Week 4 | 30-second teaser clip: NPC remembering a player action | Twitter/X, Mastodon | 50+ shares |

**Launch (Phase 2: Memory-Driven Dialogue)**

| Day | Action | Channel | Success Metric |
|-----|--------|---------|----------------|
| L-7 | Seed 5 gaming content creators with early access build | Direct outreach | 3/5 agree to try |
| L-3 | "Announcement" post with GIF demos | r/gaming, r/veloren, r/artificial | Building hype |
| L-Day | Full launch post: "NPCs that actually remember you" | HN, r/gamedev, r/rust, r/gaming, Veloren forums | HN top 10, 5k GitHub stars |
| L+1 | YouTube demo video (5-min, high production quality) | YouTube, Twitter/X | 10k views in 48h |
| L+1 | Technical deep-dive blog post | Personal blog, cross-posted to HN, dev.to | HN front page |
| L+3 | Respond to every comment, issue, and question | All channels | 100% response rate |
| L+7 | "Week 1 retrospective" post with stats + community highlights | r/gamedev, blog | Sustain interest wave |

**Post-Launch (Phase 3+: Growth)**

| Month | Action | Channel | Success Metric |
|-------|--------|---------|----------------|
| Month 1 | "How I built MEMZ" technical talk (recorded) | YouTube, conf proposal | 500+ views |
| Month 1 | Community contribution guide + "good first issue" labels | GitHub | 5+ community PRs |
| Month 2 | Academic paper submission (short paper) | AIIDE, FDG, or CHI Play | Accepted |
| Month 2 | Partnership outreach: other open-source game projects | Direct outreach | 1+ port begins |
| Month 3 | Conference talk (GDC AI Summit, RustConf, or equivalent) | Conference | Accepted |
| Month 3+ | Monthly dev log with memory stats, funny NPC moments | Blog, Reddit | Sustained engagement |

**Press Kit (Available from L-Day):**
```
memz/press-kit/
├── README.md               # One-page project summary for journalists
├── fact-sheet.md            # Key stats, tech specs, quotes
├── screenshots/             # 10 curated screenshots of NPC memory moments
│   ├── first_recognition.png      # NPC remembering player for the first time
│   ├── gossip_spread.png          # Two NPCs discussing the player
│   ├── reputation_board.png       # Town bulletin board
│   ├── memory_web.png             # Memory Web visualization
│   ├── bard_song.png              # Bard composing a song about the player
│   └── memory_conflict.png        # Two NPCs arguing about the player
├── gifs/                    # 5 animated GIFs for social media
│   ├── npc_remembers.gif          # "Wait, it remembered me!" moment
│   └── gossip_realtime.gif        # Gossip spreading between NPCs
├── videos/                  # Downloadable demo clips (MP4, 720p+)
│   ├── 30sec_teaser.mp4
│   └── 5min_demo.mp4
├── logos/                   # MEMZ logo in SVG, PNG (light/dark)
└── quotes.md                # Approved quotes from developers, playtesters
```

---

## 17. Risks & Mitigations

| Risk | Severity | Likelihood | Mitigation |
|------|----------|------------|------------|
| **LLM costs too high** | High | Medium | Tiered architecture: 99% of operations are local/rule-based. LLM only for reflection & complex dialogue. Offline-first with Ollama. Cost cap: configurable max LLM calls/hour. |
| **Performance impact on game** | High | Medium | Strict 2ms budget enforced by CI benchmarks (criterion). Async LLM calls via tokio. Only process NPCs in loaded chunks. Aggressive LRU caching. Memory pooling. Frame budget breaker → automatic throttling. |
| **Memory grows unbounded** | Medium | High | Multi-tier eviction (Hot→Warm→Cold→Archive). Ebbinghaus decay. Memory summarization (merge old episodic→semantic). Configurable per-NPC limits. Hard cap with LRU eviction. |
| **NPC dialogue feels generic** | Medium | Medium | Personality-specific prompt templates with GBNF grammar constraints. Veloren's existing character data as context. Community-contributed NPC backstories. A/B testing framework for dialogue quality. |
| **Veloren community rejects PR** | Medium | Medium | Build as a standalone mod first. Prove value with demos. Engage community early on Discord/Zulip. Worst case: maintained fork with clean merge path. |
| **Player memory injection abuse** | Medium | Medium | Multi-layer defense: input validation, rate limiting, content filtering (via local classifier), no stat-affecting memories. Configurable server-side moderation. See Section 21 for full abuse prevention design. |
| **Multiplayer synchronization** | Medium | Low | Memory state is server-authoritative. Clients receive memory-influenced dialogue, not raw memory data. Eventual consistency for gossip propagation. Conflict resolution via server-side vector clocks. |
| **Embedding model size** | Low | Low | Use compact models (all-MiniLM-L6-v2 is 80MB; INT8 quantized is 40MB). Lazy-load on first use. Fallback to TF-IDF for ultra-low-spec machines. |
| **LLM output parsing failures** | Medium | Medium | All LLM calls use structured output (JSON mode / GBNF grammars). Schema validation on every response. Auto-retry with simplified prompt. 3rd fallback: rule-based template. Zero tolerance for unparseable output reaching the game. |
| **Save file corruption** | High | Low | SQLite WAL mode for atomic writes. Checksummed save files. Automatic backup rotation (keep last 3 saves). Recovery mode: rebuild memory index from episodic log. |
| **Legal/licensing conflicts** | Medium | Low | MEMZ core is GPL-3.0 (matches Veloren). All dependencies are OSS-compatible. No proprietary model weights shipped. Users bring their own LLM. |
| **Streamer/content creator negative experience** | Medium | Low | "Streamer mode" config: increased memorable event frequency, reduced boring interactions, optional Memory Web overlay. First-time experience tuning. |

---

## 18. Success Metrics

### Viral Metrics (3 months post-launch)

| Metric | Target | Stretch | Measurement Method |
|--------|--------|---------|-------------------|
| GitHub Stars | 5,000 | 15,000 | GitHub API |
| HN Front Page | Top 10 | #1 | HN API / manual |
| Reddit posts about MEMZ | 50+ | 200+ | Reddit search / alerts |
| YouTube videos featuring MEMZ | 20+ | 100+ | YouTube search |
| Twitch streamers trying MEMZ | 10+ | 50+ | Twitch category tracking |
| Gaming press articles | 3+ | 10+ | Google News alerts |
| Veloren player adoption | 30% of active players | 70% | Server analytics |
| Discord community size | 500 | 2,000 | Discord member count |
| Academic citations | 1 | 5 | Google Scholar alerts |
| Forks / derivative projects | 10+ | 50+ | GitHub API |
| Community PRs merged | 20+ | 100+ | GitLab/GitHub PR metrics |

### Technical Metrics (Hard Requirements — CI-Enforced)

| Metric | Target | Measurement | CI Gate? |
|--------|--------|-------------|----------|
| Memory operations per frame | < 2ms (P99) | `criterion` benchmarks | ✅ Yes — PR blocked if exceeded |
| Memory retrieval latency | < 0.5ms (P50), < 1ms (P99) | In-game profiler + tracing | ✅ Yes |
| Memory storage per NPC (200 memories) | < 80KB | Unit test assertion | ✅ Yes |
| Embedding generation latency | < 5ms per memory (async) | Benchmark suite | ⚠️ Advisory |
| LLM calls per hour (gameplay) | < 20 (Tier 2 calls) | Runtime counter | ⚠️ Configurable |
| LLM response parse success rate | > 99% | Structured output validation | ✅ Yes |
| World save size increase | < 30% | Integration test | ✅ Yes |
| Memory system startup time | < 500ms for 100 NPCs | Benchmark suite | ✅ Yes |
| Zero frame drops attributable to MEMZ | 0 drops below 60 FPS | Tracy profiler integration | ✅ Yes |
| HNSW index build time (1000 memories) | < 50ms | Benchmark suite | ✅ Yes |

### Quality Metrics (Playtesting — Human Evaluation)

| Metric | Target | Method |
|--------|--------|--------|
| "Did the NPC feel like they remembered you?" | > 80% yes | Playtester survey (n=50) |
| "Was any NPC response noticeably delayed?" | < 5% yes | Playtester survey |
| "Did you screenshot/share an NPC moment?" | > 60% yes | Playtester survey + analytics |
| "Would you tell a friend about this?" | > 70% yes | Net Promoter Score (NPS) |
| NPC dialogue coherence (blind evaluation) | > 4.0/5.0 | Expert panel rating (n=10) |
| Memory injection → world response satisfaction | > 75% positive | Playtester survey |

---

## 19. Open Questions & Brainstorm Notes

### Resolved Questions ✅

1. ~~**Should NPC memories be visible to the player?**~~ → **Resolved in §10 (Memory Journal UI):** Not by default. Use "Insight" skill or special items to reveal what NPCs think of you.

2. ~~**How to handle multiplayer memory conflicts?**~~ → **Resolved in §12.8 (Multiplayer Architecture):** Server-authoritative memory state. Per-player memory streams with cross-player awareness during co-located interactions.

3. ~~**Should we upstream into Veloren or maintain as a mod?**~~ → **Resolved in §16 (Roadmap):** Start as standalone fork. If popular, propose upstream. This avoids gatekeeping risk.

### Open Questions (Active)

1. **How deep should injected memories go?**  
   *Proposal:* Three tiers — Simple (single fact), Rich (short narrative), Deep (full backstory paragraph). Each tier has different world-impact and in-game cost.
   *Status:* Needs playtesting data. Design exists, tuning TBD.

2. **Should there be a "memory wipe" mechanic?**  
   *Proposal:* Yes — rare potion or quest that resets one NPC's memory of you. Useful for fixing mistakes. Creates interesting gameplay.
   *Risk:* Could be abused (reset after crime, repeat scam). Mitigation: permanent "wiped memory" flag that some perceptive NPCs detect.

3. **Voice support?**  
   *Proposal:* Text-first. Voice (TTS) as optional addon in Phase 5+. Text is more shareable (screenshots), lower latency, and runs offline.
   *Consideration:* Modern TTS (Bark, XTTS) can run locally but add 200ms+ latency. May conflict with <2ms budget if not fully async.

4. **How to handle NPC death and memory inheritance?**  
   *Proposal:* When an NPC dies, their memories don't disappear — they persist as "ghost memories" accessible to the Library of Memories end-game location. Other NPCs who knew them retain social memories about them that evolve (grief, nostalgia).
   *Status:* Needs design spec. Affects save file size.

5. **What is the optimal embedding model size/quality tradeoff?**  
   *Options:*  
   - all-MiniLM-L6-v2 (80MB, 384-dim) — current choice, good quality, fast  
   - BGE-small-en (130MB, 384-dim) — better quality, slightly slower  
   - Nomic-embed-text (270MB, 768-dim) — best quality, needs quantization for low-end  
   *Decision:* Benchmark all three in Phase 1. Let the numbers decide.

6. **Should bard songs use a fixed verse structure or free-form?**  
   *Proposal:* GBNF grammar enforces structure (AABB or ABAB rhyme, 4-8 lines). Free-form risks incoherent output. Structured poems are more shareable.
   *Status:* Needs prompt tuning + grammar authoring in Phase 3.

7. **How to handle NPC personality drift over time?**  
   *Observation:* If memories shape behavior and behavior creates new memories, personality can drift dramatically. A kind NPC who witnesses enough violence may become fearful or hardened.
   *Proposal:* Allow personality evolution but with "core identity anchors" — traits that resist change (e.g., a fundamentally brave NPC's bravery may be tested but never drops below a floor).
   *Status:* Fascinating design problem. Needs simulation testing.

### Wild Ideas (Brainstorm) — Promoted & Unpromoted

**🎵 Bard System — PROMOTED TO CORE (Phase 3)**

Bard NPCs compose songs and tales about memorable events. This is not a stretch goal — it's a **core viral mechanic**. Songs are the most shareable content an NPC can produce.

```rust
/// A song or tale composed by a Bard NPC based on collected memories
struct BardComposition {
    bard_id: NpcId,
    /// The style of this bard (affects generated text structure)
    style: BardStyle,  // Epic, Comedic, Dark, Romantic, Satirical
    /// Source memories that inspired the composition
    source_memories: Vec<MemoryId>,
    /// The composed text (generated via Tier 2 LLM with poetry GBNF grammar)
    text: String,
    /// How many times this has been performed
    performances: u32,
    /// Settlements where this song has spread to (via traveling bards)
    known_in: HashSet<SettlementId>,
    /// Player reactions (like/dislike) — affects bard's future style
    reception: f32,
}

enum BardStyle {
    Epic,       // "Hear ye! The tale of the stranger who slew the wolf king..."
    Comedic,    // "A fool walked into Riverstead and tried to sell the baker his own bread..."
    Dark,       // "In shadows they came. In shadows they left. Only the blood remained..."
    Romantic,   // "Two travelers met beneath the great oak. One carried steel, the other, hope..."
    Satirical,  // "Our noble guard, brave and true, hid behind a barrel when the bandits came through..."
}

/// Bard composition prompt (used with GBNF grammar for verse structure)
/// Input: 3-5 most dramatic memories from the settlement
/// Output: 4-8 line poem/song with consistent meter and rhyme scheme
/// Grammar enforces: line breaks, max line length, rhyme pattern (AABB or ABAB)
/// Performance: Tier 2 LLM call, cached after generation, < 2 sec generation time
```

**How Bard Songs Spread:**
1. Bard composes song based on local memories (settlement-scoped)
2. Bard performs at taverns → NPCs in earshot gain a social memory of the song
3. Traveling bards carry songs between settlements (gossip propagation model)
4. Songs mutate slightly as they spread (drift_factor applied to retelling)
5. Players can hear the same event told differently in different towns
6. **Shareable artifact**: Players can "transcribe" songs into an in-game book item

**Why This Goes Viral:** Players will screenshot/share bard songs about their own actions. "An NPC wrote a SONG about me stealing that bread." This is guaranteed social media gold.

**Remaining Wild Ideas (Future Exploration):**
- **📸 NPC Photography:** NPCs "take mental snapshots" of dramatic moments. These become collectible in-game art.
- **🔮 Oracle NPC:** A special NPC that can read your injected memories and give cryptic prophecies based on them.
- **📚 Library of Memories:** End-game location where all NPC memories are archived. Players can browse the world's collective memory.
- **🌀 Memory Corruption:** Dark magic that corrupts NPC memories, turning allies into enemies. Quest line to fix it.
- **⏳ Generational Memory:** If the game supports it — memories passed down from NPC parents to children. Legends that grow over generations.
- **🤝 Memory Trading:** NPCs can "trade" memories (as favors). "I'll tell you what I know about the dungeon if you tell me about the outsider who arrived yesterday."
- **🎭 False Memory Injection (NPCs):** An antagonist NPC spreading false memories to turn the town against the player. Player must find and disprove them.

---

## 20. Benchmark Standards & Quality Gates

### 20.1 Cognitive Science Grounding

MEMZ's memory architecture is not invented from thin air. Every component maps to established cognitive science:

| MEMZ Component | Cognitive Science Basis | Key Reference |
|----------------|------------------------|---------------|
| **Episodic Memory** | Tulving's episodic memory (1972) — personal, event-specific, time-stamped | Tulving, E. "Episodic and Semantic Memory" |
| **Semantic Memory** | Tulving's semantic memory — general knowledge distilled from episodes | Tulving, E. (1985) "Memory and Consciousness" |
| **Procedural Memory** | Tulving's procedural memory — implicit skill knowledge + Anderson's ACT-R skill acquisition | Anderson, J.R. (1993) "Rules of the Mind"; Tulving (1985) |
| **Emotional Memory** | Plutchik's wheel of emotions + PAD (Pleasure-Arousal-Dominance) model | Russell & Mehrabian (1977); Plutchik (1980) |
| **Memory Decay** | Ebbinghaus forgetting curve: R = e^(-t/S) | Ebbinghaus, H. (1885) "Memory: A Contribution to Experimental Psychology" |
| **Flashbulb Memory** | Brown & Kulik (1977) — emotionally charged events resist decay | Brown, R. & Kulik, J. (1977) |
| **Social Memory / Gossip** | Dunbar's social brain hypothesis — gossip as social bonding | Dunbar, R. (1996) "Grooming, Gossip, and the Evolution of Language" |
| **Trust-Weighted Belief** | Bayesian belief updating — prior beliefs + evidence strength | Tenenbaum et al. (2011) "How to Grow a Mind" |
| **Reflection** | Metacognition — thinking about thinking | Flavell (1979) "Metacognition and Cognitive Monitoring" |
| **Memory Consolidation** | Sleep-mediated memory consolidation (episodic→semantic→procedural) | Stickgold, R. (2005) "Sleep-Dependent Memory Consolidation" |
| **Personality Evolution** | Five-Factor Model (Big Five) with trait plasticity over time | Roberts & Mroczek (2008) "Personality Trait Change in Adulthood" |
| **Confirmation Bias** | Nickerson (1998) — existing beliefs bias processing of new information | Modeled in belief conflict resolution (§9.3) |
| **Social Network Limits** | Dunbar's number (~150 meaningful relationships) | Used to cap NPC social memory graph size |

### 20.2 Engineering Quality Standards

| Standard | Implementation | Verification |
|----------|---------------|--------------|
| **Rust Safety** | No `unsafe` blocks in MEMZ core. All memory operations are bounds-checked. | `cargo clippy -- -D warnings` in CI |
| **Zero Panics in Production** | All fallible operations return `Result<T, E>`. No `.unwrap()` in non-test code. | Custom clippy lint + `#[deny(clippy::unwrap_used)]` |
| **Deterministic Reproducibility** | Given the same game event sequence and RNG seed, memory state is identical. | Property-based test: `proptest! { ... }` |
| **Backward Compatibility** | Save file format is versioned. Migrations auto-run on load. | Integration test: load saves from all previous versions |
| **Documentation Coverage** | All public APIs have doc comments with examples. | `#[deny(missing_docs)]` in CI |
| **Test Coverage** | >80% line coverage for memz-core. | `cargo llvm-cov` in CI |
| **Benchmark Regression** | No performance regression >10% from baseline. | `criterion` benchmarks in CI with `--save-baseline` |
| **Dependency Audit** | No known vulnerabilities in dependencies. | `cargo audit` in CI |

### 20.3 Playability Standards

MEMZ follows the **"invisible technology" principle**: the memory system should feel like magic to the player, never like a technical limitation.

| Standard | Description | Test |
|----------|-------------|------|
| **Zero Loading Screens** | Memory operations never block the game. All LLM calls are async. | Soak test: 8 hours, zero hitches >16ms |
| **First Interaction Quality** | The very first NPC interaction in a new game must feel responsive and coherent. | Manual QA: cold-start test suite |
| **Memory Feels Natural** | NPCs should never say "I remember event #47 from Day 12." Memory references must be natural language. | Dialogue quality review (human evaluation panel) |
| **Failure is Invisible** | If the LLM fails, the player sees a slightly less interesting response — NOT an error. | Fault injection test: kill LLM process mid-game |
| **Consistent Personality** | An NPC's personality must remain consistent across memory-driven responses. | A/B test: personality consistency score >0.85 |

---

## 21. Security, Safety & Abuse Prevention

### 21.1 Player Memory Injection Safety

The player memory injection system is the most abuse-prone feature. Defense-in-depth:

```
Layer 1: Client-Side Validation
├── Max length: 500 characters per injection
├── Rate limit: 3 injections per real-world hour
├── Basic regex filter: no URLs, no code, no special characters beyond punctuation
└── Emotional tone must be selected from predefined list

Layer 2: Server-Side Content Filter (Local Classifier)
├── Small ONNX classification model (~5MB) for toxicity detection
├── Categories: hate speech, sexual content, self-harm, violence (gratuitous)
├── Threshold: content score < 0.3 passes; 0.3-0.7 flagged for review; >0.7 rejected
├── Runs locally — no cloud dependency for content filtering
└── False positive rate tuned for gaming context (violence in fantasy is OK)

Layer 3: Semantic Validation (LLM)
├── LLM checks: "Is this a plausible personal memory for a fantasy RPG character?"
├── Rejects: game-breaking claims, meta-gaming, out-of-world references
├── Accepts: personal backstories, traumas, relationships, dreams, fears
└── Uses Tier 1 model (local, fast)

Layer 4: World-Impact Throttling
├── Injected memories affect NPC behavior GRADUALLY (over game-hours, not instantly)
├── No single injection can flip an NPC's core disposition more than 0.2 on any axis
├── Contradictory injections trigger "NPC suspicion" rather than blind acceptance
└── Server admin can configure injection rules per-server
```

### 21.2 Multiplayer Security

| Threat | Mitigation |
|--------|------------|
| **Memory data leaking between players** | Memory state is server-authoritative. Clients only receive rendered dialogue text, never raw memory objects. |
| **Malicious client sending fake events** | All game events validated server-side. Clients cannot inject memory events directly. |
| **Save file tampering** | Save files are checksummed (BLAKE3). Tampered files are rejected on load. |
| **Denial-of-service via LLM spam** | Per-player LLM call rate limiting. Server-wide LLM queue with priority (gameplay > reflection > gossip). |

### 21.3 Data Privacy

- **No telemetry by default.** Optional, opt-in analytics for game developers.
- **Player memories are stored locally** (server-side in multiplayer, controlled by server admin).
- **No memory data is sent to cloud LLM APIs** unless the user explicitly configures cloud LLM. Even then, only the prompt (with memory context) is sent, not the full memory store.
- **GDPR-friendly:** Players can export and delete all their memory data at any time.

---

## 22. Accessibility & Inclusivity

MEMZ is designed to be playable and enjoyable by everyone.

| Feature | Implementation |
|---------|---------------|
| **Screen Reader Support** | Memory Journal UI, Reputation Board, and Memory Web all expose ARIA-equivalent semantics. All NPC dialogue is text-first. |
| **Keyboard Navigation** | Full keyboard navigation for Memory Journal (no mouse required). All UI elements are tab-navigable. |
| **Color-Blind Modes** | Memory Web visualization uses patterns + shapes in addition to colors. Emotional valence uses icons, not just red/green. |
| **Dyslexia-Friendly Font** | Configurable font in Memory Journal UI. OpenDyslexic font bundled as option. |
| **Text Size Scaling** | All MEMZ UI elements respect Veloren's text scaling settings. |
| **Reduced Motion** | Memory Web animations can be disabled. Reputation Board uses static layout option. |
| **Language** | All MEMZ UI strings are internationalized via Veloren's existing Fluent i18n system. Community-translatable. |
| **Cognitive Accessibility** | Optional "Memory Summary" mode: instead of full NPC dialogue, shows bullet-point summaries of what NPCs remember. |

---

## 23. Observability, Telemetry & Debugging

### 23.1 Developer Tools

| Tool | Description |
|------|-------------|
| **Memory Inspector** | In-game debug overlay showing any NPC's full MemoryBank. Filterable by memory type, time range, emotional valence. |
| **Memory Web Debugger** | Real-time visualization of gossip propagation. Shows which NPC told which NPC what, with timestamps. |
| **LLM Call Log** | Rolling log of all LLM calls: prompt, response, latency, parse success/failure, tier. Exportable for analysis. |
| **Frame Budget Monitor** | Real-time overlay showing per-system timing within the 2ms budget. Alerts when budget is >80% consumed. |
| **Memory Diff Tool** | Compare NPC memory state between two save files. Useful for debugging "why did the NPC's opinion change?" |
| **Tracy Integration** | All MEMZ systems emit Tracy spans for profiling with the Tracy profiler. |

### 23.2 Runtime Metrics (Optional Server Dashboard)

For multiplayer server admins:

```
MEMZ Server Metrics (Prometheus-compatible):
├── memz_active_npc_count          — NPCs with active memory processing
├── memz_total_memories            — Total memories across all NPCs
├── memz_frame_budget_ms           — P50, P95, P99 of per-frame memory cost
├── memz_llm_calls_total           — Counter by tier (0/1/2)
├── memz_llm_latency_ms            — Histogram of LLM response times
├── memz_llm_parse_failures        — Counter of structured output parse failures
├── memz_memory_evictions          — Counter of memories evicted by tier
├── memz_gossip_propagations       — Counter of NPC-to-NPC memory shares
├── memz_injection_attempts        — Counter (accepted/rejected/filtered)
├── memz_save_duration_ms          — Time to persist memory state
└── memz_embedding_queue_depth     — Pending embedding generation jobs
```

---

## 24. Appendix — Research References

### Academic Papers

1. **Park, J.S., et al. (2023).** "Generative Agents: Interactive Simulacra of Human Behavior." *UIST '23.* [arXiv:2304.03442](https://arxiv.org/abs/2304.03442)
   - Foundation for memory stream → reflection → planning architecture
   - 20.7k GitHub stars, highly influential

2. **Shinn, N., et al. (2023).** "Reflexion: Language Agents with Verbal Reinforcement Learning." *NeurIPS 2023.*
   - Self-reflection mechanisms for LLM agents

3. **Zhuge, M., et al. (2024).** "Agent Hospital: A Simulacrum of Hospital with Evolvable Medical Agents."
   - Memory-based agent simulation in a domain setting

4. **Wang, G., et al. (2023).** "Voyager: An Open-Ended Embodied Agent with Large Language Models." *NeurIPS 2023.*
   - LLM agents in Minecraft with skill/memory libraries

5. **Kaiya, Z., et al. (2023).** "Lyfe Agents: Generative agents for low-cost real-time social interactions." [arXiv:2310.02172](https://arxiv.org/abs/2310.02172)
   - Option-action framework for low-cost agent decisions; Summarize-and-Forget memory
   - Key inspiration for MEMZ's tiered LLM approach (10-100x cost reduction)

6. **SIMA Team, Google DeepMind (2024).** "Scaling Instructable Agents Across Many Simulated Worlds." [arXiv:2404.10179](https://arxiv.org/abs/2404.10179)
   - Language-driven agents operating across diverse 3D game environments
   - Validates the approach of using language as a universal interface for game agents

7. **Coda-Forno, J., et al. (2024).** "CogBench: A Large Language Model Walks into a Psychology Lab." [arXiv:2402.18225](https://arxiv.org/abs/2402.18225)
   - Behavioral benchmarks for LLM agents from cognitive psychology experiments
   - Informs MEMZ's approach to psychologically grounded NPC behavior

8. **Packer, C., et al. (2023).** "MemGPT: Towards LLMs as Operating Systems." [arXiv:2310.08560](https://arxiv.org/abs/2310.08560)
   - Virtual memory paging for LLM context management (now Letta, 21.2k ⭐)
   - Inspiration for MEMZ's multi-tier memory management (hot/warm/cold rings)

9. **Tulving, E. (1985).** "Memory and Consciousness." *Canadian Psychology.*
   - Foundational taxonomy: episodic vs. semantic memory — directly implemented in MEMZ

10. **Ebbinghaus, H. (1885).** "Memory: A Contribution to Experimental Psychology."
    - The forgetting curve: R = e^(-t/S) — directly implemented in MEMZ's decay model

11. **Dunbar, R. (1996).** "Grooming, Gossip, and the Evolution of Language." Harvard University Press.
    - Social brain hypothesis; gossip as social bonding — directly informs MEMZ's gossip mechanics

12. **Russell, J.A. & Mehrabian, A. (1977).** "Evidence for a Three-Factor Theory of Emotions." *J. of Research in Personality.*
    - PAD (Pleasure-Arousal-Dominance) emotional model — used for MEMZ's emotional memory

13. **Anderson, J.R. (1993).** "Rules of the Mind." Lawrence Erlbaum Associates.
    - ACT-R theory of skill acquisition — production rules that strengthen with practice
    - Directly informs MEMZ's procedural memory learning rate model (§8.7)

14. **Stickgold, R. & Walker, M.P. (2013).** "Sleep-Dependent Memory Consolidation and Reconsolidation." *Sleep Medicine.*
    - Memory replay during sleep transforms episodic→semantic and strengthens procedural
    - Directly informs MEMZ's memory consolidation system (§8.8)

15. **Roberts, B.W. & Mroczek, D. (2008).** "Personality Trait Change in Adulthood." *Current Directions in Psychological Science.*
    - Personality traits can change over time but have stable "core identity" anchors
    - Informs MEMZ's NPC personality evolution with floor/ceiling constraints

16. **Nickerson, R.S. (1998).** "Confirmation Bias: A Ubiquitous Phenomenon in Many Guises." *Review of General Psychology.*
    - Existing beliefs bias processing of new information
    - Directly implemented in MEMZ's conflict resolution emotional_bias modifier (§9.3)

### Open-Source Projects

| Project | Stars | Relevance | URL |
|---------|-------|-----------|-----|
| **Generative Agents** | 20.7k ⭐ | Memory architecture reference | https://github.com/joonspk-research/generative_agents |
| **Letta (MemGPT)** | 21.2k ⭐ | Persistent memory for LLM agents; virtual memory paging | https://github.com/letta-ai/letta |
| **Mantella** | 338 ⭐ | Game AI NPC mod reference | https://github.com/art-from-the-machine/Mantella |
| **Veloren** | 7.1k ⭐ | Target game (365+ contributors, daily commits) | https://gitlab.com/veloren/veloren |
| **Veloren rtsim** | — | NPC simulation to build on | https://gitlab.com/veloren/veloren/-/tree/master/rtsim |
| **AI Town** (a16z) | — | Generative agents in web app | https://github.com/a16z-infra/ai-town |
| **llama.cpp** | 75k+ ⭐ | Local LLM inference (GGUF format, GBNF grammars) | https://github.com/ggerganov/llama.cpp |
| **Ollama** | 120k+ ⭐ | Local LLM serving (simple API) | https://github.com/ollama/ollama |
| **fastembed-rs** | — | Rust embedding generation | https://github.com/Anush008/fastembed-rs |
| **usearch** | — | High-performance HNSW vector search | https://github.com/unum-cloud/usearch |

### Games Evaluated (Full List)

| Game | Stars | NPCs | Language | License | Why Not Chosen |
|------|-------|------|----------|---------|---------------|
| **Veloren** | 7.1k | ✅ Rich | Rust | GPL-3.0 | ✅ **CHOSEN** |
| Cataclysm: DDA | 12.1k | ⚠️ Hostile focus | C++ | CC BY-SA 3.0 | NPCs are mostly enemies; limited social interaction |
| Battle for Wesnoth | 6.4k | ⚠️ Turn-based | C++/Lua | GPL-2.0 | Turn-based strategy; NPCs are units, not social characters |
| Luanti (Minetest) | 12.3k | ❌ No built-in NPCs | C++/Lua | LGPL-2.1 | Engine only; would need to build NPCs from scratch |
| Space Station 14 | 3.5k | ✅ Rich | C# | MIT | Good candidate, but sessions are transient (rounds reset) |
| Ryzom | — | ✅ MMO NPCs | C++ | AGPL-3.0 | Declining community; complex legacy codebase |
| Dwarf Fortress | — | ✅ Deep simulation | C++ | Proprietary | Not open source (source-available with restrictions) |
| Crossfire | — | ⚠️ MMORPG | C | GPL-2.0 | Dated; small community |
| NetHack | — | ⚠️ Text-based | C | NGPL | Text-only; limited NPC interaction |
| Tales of Maj'Eyal | — | ⚠️ Roguelike | Lua | GPL-3.0 | Assets proprietary; limited NPC social dynamics |
| 0 A.D. | — | ⚠️ RTS units | C++/JS | GPL-2.0 | RTS; units aren't characters with social lives |
| OpenMW (Morrowind) | — | ✅ Rich RPG NPCs | C++ | GPL-3.0 | Requires proprietary Morrowind data; can't distribute freely |
| Shattered Pixel Dungeon | — | ⚠️ Roguelike | Java | GPL-3.0 | Mobile roguelike; minimal NPC depth |
| Friday Night Funkin' | — | ❌ Rhythm game | Haxe | Apache 2.0 | No NPC interaction mechanic |
| SuperTuxKart | — | ❌ Racing | C++ | GPL-3.0 | Racing game; no NPCs |

---

## Final Note

> **MEMZ is not just a mod. It's a thesis statement:**  
> *Games don't need bigger worlds. They need worlds that remember.*  
>  
> Every game studio is building bigger maps, more quests, more content.  
> None of them are building **memory.**  
>  
> MEMZ proves that a single layer — persistent, structured, social memory —  
> can make a game world feel more alive than any AAA open world with 100x the budget.  
>  
> And we're not cutting corners:  
> - **Cognitive science** grounds every memory type (Tulving, Ebbinghaus, Dunbar, PAD model)  
> - **Performance engineering** ensures zero frame drops (CI-enforced <2ms budget)  
> - **Structured LLM output** guarantees reliable parsing (GBNF grammars, not hope)  
> - **Graceful degradation** means the game works perfectly even without an LLM  
> - **Security & safety** protect against every abuse vector  
> - **Accessibility** ensures everyone can experience the magic  
> - **Observability** means we can debug anything in production  
>  
> No commercial platform (Inworld, Convai) offers this combination.  
> No research project (Generative Agents, Lyfe Agents) is playable.  
> No existing mod (Mantella) has this depth of memory architecture.  
>  
> MEMZ is the intersection of cognitive science, systems engineering,  
> and game design that nobody else has built.  
>  
> And we're giving it away. Open source. For everyone.  
>  
> **Let's make game characters remember.** 🧠

---

*Document version: 3.0 — Production-Ready Edition*  
*Last updated: 20 February 2026*  
*Project Vyuh — MEMZ*  

*Changelog:*  
*v3.0 — Added Procedural Memory type (§8.7), Memory Consolidation system (§8.8),*  
*"First 5 Minutes" experience design (§14.0), Prompt Engineering section with versioned*  
*templates and GBNF grammars (§12.3.1), Memory Replay technical design, Reputation Board*  
*data model, Bard System promoted to core feature with full spec, example memz.toml*  
*configuration, full launch strategy with press kit, expanded conflict resolution algorithm*  
*(information chain depth, freshness scoring, confirmation bias), cleaned up Open Questions*  
*(resolved vs active), expanded Table of Contents with sub-sections.*  

*v2.0 — Added benchmark standards, quality gates, cognitive science grounding,*  
*security/safety design, accessibility, observability, expanded competitive landscape,*  
*CI-enforced performance targets, graceful degradation, structured LLM output,*  
*multi-tier memory eviction, and human evaluation metrics.*
