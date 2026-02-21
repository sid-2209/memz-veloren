//! MEMZ Benchmark Suite (§12.6, §20.2)
//!
//! CI-enforced performance targets:
//!   memory_creation_single ........... < 10μs
//!   memory_retrieval_top5_from_200 ... < 500μs
//!   memory_decay_pass_50_npcs ....... < 50μs
//!   full_frame_budget_20_active_npcs . < 2ms
//!   observation_pipeline ............. < 100μs
//!   gossip_propagation ............... < 50μs
//!   reputation_update ................ < 20μs
//!   eviction_pass .................... < 100μs

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use memz_core::behavior;
use memz_core::config::{MemoryConfig, RetrievalConfig};
use memz_core::memory::MemoryBank;
use memz_core::memory::episodic::EpisodicMemory;
use memz_core::memory::social::SocialMemory;
use memz_core::observation::{self, EventKind, ObservedEvent};
use memz_core::reputation::{NotableDeed, ReputationBoard};
use memz_core::retrieval::RetrievalEngine;
use memz_core::social;
use memz_core::types::{
    Embedding, EntityId, GameTimestamp, Location, PersonalityTraits, SettlementId,
};

fn ts(tick: u64) -> GameTimestamp {
    GameTimestamp::now(tick)
}

fn make_episodic(i: u32) -> EpisodicMemory {
    EpisodicMemory::new(
        format!("Event number {} happened in the town square", i),
        vec![EntityId::new(), EntityId::new()],
        Location {
            x: i as f32,
            y: 0.0,
            z: 0.0,
        },
        ts(u64::from(i) * 1000),
        (i as f32 / 100.0 - 0.5).clamp(-1.0, 1.0),
        (i as f32 / 200.0).clamp(0.1, 0.9),
    )
}

// ---------------------------------------------------------------------------
// Bench: Single memory creation (target: < 10μs)
// ---------------------------------------------------------------------------

fn bench_memory_creation(c: &mut Criterion) {
    c.bench_function("memory_creation_single", |b| {
        b.iter(|| {
            let mem = make_episodic(black_box(42));
            black_box(mem);
        });
    });
}

// ---------------------------------------------------------------------------
// Bench: Memory retrieval top-5 from 200 (target: < 500μs)
// ---------------------------------------------------------------------------

fn bench_memory_retrieval(c: &mut Criterion) {
    let mut bank = MemoryBank::new();
    for i in 0..200 {
        let mut mem = make_episodic(i);
        mem.embedding = Some(Embedding(vec![
            (i as f32 / 200.0).sin(),
            (i as f32 / 200.0).cos(),
            0.5,
        ]));
        bank.episodic.push(mem);
    }

    let engine = RetrievalEngine::new(RetrievalConfig::default());
    let context_embedding = Embedding(vec![0.5, 0.5, 0.5]);
    let current_time = ts(2_000_000);
    let entries: Vec<_> = bank
        .episodic
        .iter()
        .map(|m| memz_core::memory::MemoryEntry::Episodic(m.clone()))
        .collect();

    c.bench_function("memory_retrieval_top5_from_200", |b| {
        b.iter(|| {
            let results = engine
                .retrieve(
                    black_box(&context_embedding),
                    black_box(&entries),
                    black_box(&current_time),
                    None,
                )
                .expect("retrieval");
            black_box(results);
        });
    });
}

// ---------------------------------------------------------------------------
// Bench: Decay pass for 50 NPCs (target: < 50μs)
// ---------------------------------------------------------------------------

fn bench_decay_pass(c: &mut Criterion) {
    let config = MemoryConfig::default();
    let current_time = ts(7_200_000);

    let mut banks: Vec<MemoryBank> = (0..50)
        .map(|npc| {
            let mut bank = MemoryBank::new();
            for i in 0..20 {
                bank.episodic.push(make_episodic(npc * 20 + i));
            }
            bank
        })
        .collect();

    c.bench_function("memory_decay_pass_50_npcs", |b| {
        b.iter(|| {
            for bank in &mut banks {
                memz_core::decay::decay_episodic_memories(
                    black_box(&mut bank.episodic),
                    black_box(&current_time),
                    black_box(&config),
                );
            }
        });
    });
}

// ---------------------------------------------------------------------------
// Bench: Full frame budget for 20 active NPCs (target: < 2ms)
// ---------------------------------------------------------------------------

fn bench_full_frame(c: &mut Criterion) {
    use memz_veloren::events::GameEvent;
    use memz_veloren::systems;

    let config = MemoryConfig::default();
    let current_time = ts(720_000);

    let mut banks: Vec<MemoryBank> = (0..20)
        .map(|npc| {
            let mut bank = MemoryBank::new();
            for i in 0..50 {
                bank.episodic.push(make_episodic(npc * 50 + i));
            }
            bank
        })
        .collect();

    let event = GameEvent::Dialogue {
        speaker: EntityId::new(),
        listener: EntityId::new(),
        content: "Hello there!".to_string(),
        location: Location::default(),
        timestamp: current_time,
    };

    c.bench_function("full_frame_budget_20_active_npcs", |b| {
        b.iter(|| {
            for bank in &mut banks {
                systems::observe_event(black_box(&event), black_box(bank));
            }
            for bank in &mut banks {
                systems::run_decay(
                    black_box(bank),
                    black_box(&current_time),
                    black_box(&config),
                );
            }
            for bank in &mut banks {
                systems::enforce_limits(black_box(bank), black_box(&config));
            }
        });
    });
}

// ---------------------------------------------------------------------------
// Bench: Observation pipeline (target: < 100μs)
// ---------------------------------------------------------------------------

fn bench_observation_pipeline(c: &mut Criterion) {
    let mut bank = MemoryBank::new();
    for i in 0..50 {
        bank.episodic.push(make_episodic(i));
    }

    let observer = EntityId::new();
    let event = ObservedEvent {
        kind: EventKind::Help,
        description: "Defended the village from raiders".to_string(),
        participants: vec![EntityId::new(), EntityId::new()],
        witnesses: vec![observer],
        location: Location::default(),
        timestamp: ts(1_000_000),
        emotional_valence: 0.7,
        importance: 0.8,
        pad_shift: None,
    };
    let known: Vec<EntityId> = (0..10).map(|_| EntityId::new()).collect();

    c.bench_function("observation_pipeline", |b| {
        b.iter(|| {
            let result = observation::observe(
                black_box(&event),
                black_box(observer),
                black_box(&mut bank),
                black_box(&known),
            );
            black_box(result);
        });
    });
}

// ---------------------------------------------------------------------------
// Bench: Gossip propagation (target: < 50μs)
// ---------------------------------------------------------------------------

fn bench_gossip_propagation(c: &mut Criterion) {
    let personality = PersonalityTraits {
        credulity: 0.7,
        openness: 0.7,
        gossip_tendency: 0.6,
        ..Default::default()
    };

    let claim = SocialMemory::new(
        EntityId::new(),
        EntityId::new(),
        "The hero slew the dragon of Mount Flame",
        0.9,
        0,
        ts(1000),
    );

    let receiver = EntityId::new();

    c.bench_function("gossip_propagation", |b| {
        b.iter(|| {
            let result = social::propagate_memory(
                black_box(&claim),
                black_box(receiver),
                black_box(&personality),
                black_box(0.8),
                black_box(false),
                black_box(None),
                black_box(0.6),
                black_box(0.0),
                black_box(0.7),
                black_box(ts(2000)),
            );
            black_box(result);
        });
    });
}

// ---------------------------------------------------------------------------
// Bench: Reputation update (target: < 20μs)
// ---------------------------------------------------------------------------

fn bench_reputation_update(c: &mut Criterion) {
    let settlement = SettlementId::new();
    let mut board = ReputationBoard::new(settlement, ts(0));
    let entity = EntityId::new();

    // Pre-populate with some history
    for i in 0..20 {
        board.report_sentiment(entity, 0.3, ts(i * 100));
    }

    c.bench_function("reputation_update", |b| {
        let mut tick = 10_000u64;
        b.iter(|| {
            tick += 1;
            board.report_sentiment(black_box(entity), black_box(0.5), black_box(ts(tick)));
            board.record_deed(black_box(NotableDeed {
                actor: entity,
                description: "Heroic deed".to_string(),
                valence: 0.7,
                timestamp: ts(tick),
                witness_count: 3,
            }));
        });
    });
}

// ---------------------------------------------------------------------------
// Bench: Disposition + behavior computation (target: < 50μs)
// ---------------------------------------------------------------------------

fn bench_disposition_computation(c: &mut Criterion) {
    let player = EntityId::new();
    let mut bank = MemoryBank::new();

    // Build a realistic memory bank
    for i in 0..100 {
        bank.episodic.push(EpisodicMemory::new(
            format!("Entity {player} did thing {i}"),
            vec![player],
            Location::default(),
            ts(u64::from(i) * 1000),
            (i as f32 / 100.0 - 0.5) * 2.0,
            0.5,
        ));
    }

    c.bench_function("disposition_computation", |b| {
        b.iter(|| {
            let disp = behavior::compute_disposition(black_box(&bank), black_box(player));
            let _greeting = behavior::compute_greeting_style(black_box(&disp));
            let _price = behavior::compute_price_modifier(black_box(&disp));
            black_box(disp);
        });
    });
}

// ---------------------------------------------------------------------------
// Bench: Eviction pass for 50 NPCs (target: < 100μs)
// ---------------------------------------------------------------------------

fn bench_eviction_pass(c: &mut Criterion) {
    let config = MemoryConfig::default();

    let mut banks: Vec<MemoryBank> = (0..50)
        .map(|npc| {
            let mut bank = MemoryBank::new();
            for i in 0..40 {
                bank.episodic.push(make_episodic(npc * 40 + i));
            }
            bank
        })
        .collect();

    c.bench_function("eviction_pass_50_npcs", |b| {
        b.iter(|| {
            for bank in &mut banks {
                bank.episodic.truncate(black_box(config.max_episodic_per_npc));
                bank.social.truncate(black_box(config.max_social_per_npc));
            }
        });
    });
}

criterion_group!(
    benches,
    bench_memory_creation,
    bench_memory_retrieval,
    bench_decay_pass,
    bench_full_frame,
    bench_observation_pipeline,
    bench_gossip_propagation,
    bench_reputation_update,
    bench_disposition_computation,
    bench_eviction_pass,
);
criterion_main!(benches);
