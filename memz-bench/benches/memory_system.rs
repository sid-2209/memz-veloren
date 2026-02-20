//! MEMZ Benchmark Suite (§12.6, §20.2)
//!
//! CI-enforced performance targets:
//!   memory_creation_single ........... < 10μs
//!   memory_retrieval_top5_from_200 ... < 500μs
//!   memory_decay_pass_50_npcs ....... < 50μs
//!   full_frame_budget_20_active_npcs . < 2ms

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use memz_core::config::RetrievalConfig;
use memz_core::memory::MemoryBank;
use memz_core::memory::episodic::EpisodicMemory;
use memz_core::retrieval::RetrievalEngine;
use memz_core::types::{Embedding, EntityId, GameTimestamp, Location};

fn make_timestamp(day: u32, tick: u64) -> GameTimestamp {
    GameTimestamp {
        day,
        hour: 12.0,
        tick,
    }
}

fn make_episodic(i: u32) -> EpisodicMemory {
    EpisodicMemory::new(
        format!("Event number {} happened in the town square", i),
        vec![EntityId(1), EntityId(i as u64 + 10)],
        Location {
            x: i as f32,
            y: 0.0,
            z: 0.0,
            region: Some("test_region".to_string()),
        },
        make_timestamp(i / 10 + 1, u64::from(i) * 1000),
        (i as f32 / 100.0 - 0.5).clamp(-1.0, 1.0),
        (i as f32 / 200.0).clamp(0.1, 0.9),
    )
}

/// Benchmark: Single memory creation (target: < 10μs).
fn bench_memory_creation(c: &mut Criterion) {
    c.bench_function("memory_creation_single", |b| {
        b.iter(|| {
            let mem = make_episodic(black_box(42));
            black_box(mem);
        });
    });
}

/// Benchmark: Memory retrieval top-5 from 200 memories (target: < 500μs).
fn bench_memory_retrieval(c: &mut Criterion) {
    // Pre-populate a memory bank with 200 episodic memories.
    let mut bank = MemoryBank::new();
    for i in 0..200 {
        let mut mem = make_episodic(i);
        mem.embedding = Some(Embedding(vec![
            (i as f32 / 200.0).sin(),
            (i as f32 / 200.0).cos(),
            0.5,
        ]));
        bank.add_episodic(mem);
    }

    let engine = RetrievalEngine::new(RetrievalConfig::default());
    let context_embedding = Embedding(vec![0.5, 0.5, 0.5]);
    let current_time = make_timestamp(30, 2_000_000);
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
                .unwrap();
            black_box(results);
        });
    });
}

/// Benchmark: Decay pass for 50 NPCs (target: < 50μs).
fn bench_decay_pass(c: &mut Criterion) {
    let config = memz_core::config::MemoryConfig::default();
    let current_time = make_timestamp(100, 7_200_000);

    // Create 50 memory banks with 20 memories each.
    let mut banks: Vec<MemoryBank> = (0..50)
        .map(|npc| {
            let mut bank = MemoryBank::new();
            for i in 0..20 {
                bank.add_episodic(make_episodic(npc * 20 + i));
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

/// Benchmark: Full frame budget for 20 active NPCs (target: < 2ms).
fn bench_full_frame(c: &mut Criterion) {
    use memz_veloren::events::GameEvent;
    use memz_veloren::systems;

    let config = memz_core::config::MemoryConfig::default();
    let current_time = make_timestamp(10, 720_000);

    // Create 20 NPC memory banks with 50 memories each.
    let mut banks: Vec<MemoryBank> = (0..20)
        .map(|npc| {
            let mut bank = MemoryBank::new();
            for i in 0..50 {
                bank.add_episodic(make_episodic(npc * 50 + i));
            }
            bank
        })
        .collect();

    let event = GameEvent::Dialogue {
        speaker: EntityId(1),
        listener: EntityId(2),
        content: "Hello there!".to_string(),
        location: Location::default(),
        timestamp: current_time,
    };

    c.bench_function("full_frame_budget_20_active_npcs", |b| {
        b.iter(|| {
            // 1. Observe event for each NPC.
            for bank in &mut banks {
                systems::observe_event(black_box(&event), black_box(bank));
            }
            // 2. Decay pass.
            for bank in &mut banks {
                systems::run_decay(
                    black_box(bank),
                    black_box(&current_time),
                    black_box(&config),
                );
            }
            // 3. Enforce limits.
            for bank in &mut banks {
                systems::enforce_limits(black_box(bank));
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
);
criterion_main!(benches);
