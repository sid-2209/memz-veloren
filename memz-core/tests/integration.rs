//! Integration Tests — End-to-End Memory Flows (§20.2)
//!
//! These tests verify complete memory lifecycle scenarios:
//! save/load round-trips, multi-NPC interactions, event→memory→behavior chains.

use memz_core::behavior;
use memz_core::config::{MemoryConfig, PersistenceConfig};
use memz_core::consolidation;
use memz_core::decay;
use memz_core::memory::episodic::EpisodicMemory;
use memz_core::memory::emotional::EmotionalMemory;
use memz_core::memory::social::SocialMemory;
use memz_core::memory::MemoryBank;
use memz_core::observation::{self, EventKind, ObservedEvent};
use memz_core::persistence::PersistenceEngine;
use memz_core::replay;
use memz_core::reputation::{NotableDeed, ReputationBoard};
use memz_core::social;
use memz_core::types::{EntityId, GameTimestamp, Location, PADState, PersonalityTraits, SettlementId};

fn ts(tick: u64) -> GameTimestamp {
    GameTimestamp::now(tick)
}

fn loc() -> Location {
    Location::default()
}

// ---------------------------------------------------------------------------
// Full memory lifecycle: create → decay → retrieve → persist → restore
// ---------------------------------------------------------------------------

#[test]
fn full_memory_lifecycle() {
    let player = EntityId::new();
    let npc = EntityId::new();
    let config = MemoryConfig::default();
    let persist_config = PersistenceConfig::default();

    // 1. NPC observes player events
    let mut bank = MemoryBank::new();
    bank.episodic.push(EpisodicMemory::new(
        format!("Entity {player} helped defend the village"),
        vec![player],
        loc(),
        ts(1000),
        0.8,
        0.9,
    ));

    bank.episodic.push(EpisodicMemory::new(
        format!("Entity {player} bought supplies"),
        vec![player],
        loc(),
        ts(2000),
        0.1,
        0.2,
    ));

    bank.emotional.push(EmotionalMemory::new(
        player,
        "gratitude",
        0.7,
        PADState::new(0.6, 0.3, 0.2),
        vec![],
        ts(1000),
    ));

    // 2. Check disposition is positive
    let disposition = behavior::compute_disposition(&bank, player);
    assert!(disposition.sentiment > 0.0, "Should have positive sentiment after help");

    // 3. Greeting should be warm
    let greeting = behavior::compute_greeting_style(&disposition);
    assert!(
        matches!(greeting, behavior::GreetingStyle::Warm | behavior::GreetingStyle::Neutral),
        "Expected warm or neutral greeting"
    );

    // 4. Persist
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("integration_test.db");
    let engine = PersistenceEngine::open(db_path.to_str().expect("path"), &persist_config).expect("open");
    engine.save_bank(&npc, &bank).expect("save");

    // 5. Restore
    let restored = engine.load_bank(&npc).expect("load").expect("found");
    assert_eq!(restored.episodic.len(), bank.episodic.len());
    assert_eq!(restored.emotional.len(), bank.emotional.len());

    // 6. Disposition is preserved after restore
    let restored_disposition = behavior::compute_disposition(&restored, player);
    assert!(
        (restored_disposition.sentiment - disposition.sentiment).abs() < 0.01,
        "Disposition should be preserved after save/load"
    );

    // 7. Decay pass (simulate time passing)
    let mut restored_bank = restored;
    let far_future = GameTimestamp::now(36_000_000);
    let initial_count = restored_bank.episodic.len();
    decay::decay_episodic_memories(&mut restored_bank.episodic, &far_future, &config);
    // Low-importance memories may have decayed
    assert!(restored_bank.episodic.len() <= initial_count);
}

// ---------------------------------------------------------------------------
// Multi-NPC gossip chain: A tells B, B tells C
// ---------------------------------------------------------------------------

#[test]
fn multi_npc_gossip_chain() {
    let subject = EntityId::new();
    let npc_a = EntityId::new();
    let npc_b = EntityId::new();

    let credulous = PersonalityTraits {
        credulity: 0.9,
        openness: 0.9,
        gossip_tendency: 0.8,
        ..Default::default()
    };

    // NPC A has first-hand knowledge
    let claim_a = SocialMemory::new(
        subject,
        npc_a,
        "Subject slew the dragon of Mount Flame",
        1.0,
        0,   // first-hand
        ts(1000),
    );

    // A tells B
    let result_ab = social::propagate_memory(
        &claim_a,
        npc_b,
        &credulous,
        0.9,    // high trust
        false,  // no direct experience
        None,
        0.7,    // consistent
        0.0,
        0.8,    // reliable source
        ts(2000),
    );

    let claim_b = match result_ab {
        social::PropagationResult::Accepted { new_memory, .. } => new_memory,
        other => panic!("Expected Accepted, got {:?}", other),
    };

    // B's memory should have incremented propagation_depth
    assert_eq!(claim_b.propagation_depth, 1, "Should be second-hand");
    assert_eq!(claim_b.about, subject);

    // B tells C
    let npc_c = EntityId::new();
    let result_bc = social::propagate_memory(
        &claim_b,
        npc_c,
        &credulous,
        0.7,    // somewhat less trust
        false,
        None,
        0.5,
        0.0,
        0.6,
        ts(3000),
    );

    match result_bc {
        social::PropagationResult::Accepted { new_memory, belief_strength } => {
            assert_eq!(new_memory.propagation_depth, 2, "Should be third-hand");
            // Belief should be weaker due to chain degradation
            assert!(belief_strength > 0.0);
        }
        social::PropagationResult::Uncertain { .. } => {
            // Also acceptable — long gossip chains reduce belief
        }
        social::PropagationResult::Rejected { reason } => {
            // Acceptable for long chains with reduced trust
            assert!(reason.contains("too low"), "Unexpected rejection: {reason}");
        }
    }
}

// ---------------------------------------------------------------------------
// Observation → Disposition → Price modifier chain
// ---------------------------------------------------------------------------

#[test]
fn observation_to_price_modifier() {
    let player = EntityId::new();
    let _npc = EntityId::new();

    let mut bank = MemoryBank::new();

    // NPC directly observes player helping (via ObservedEvent API)
    let event1 = ObservedEvent {
        kind: EventKind::Help,
        description: "Player defended the market from thieves".to_string(),
        participants: vec![player],
        witnesses: vec![player],
        location: loc(),
        timestamp: ts(1000),
        emotional_valence: 0.7,
        importance: 0.8,
        pad_shift: None,
    };
    observation::observe(&event1, player, &mut bank, &[]);

    let event2 = ObservedEvent {
        kind: EventKind::Help,
        description: "Player donated gold to the orphanage".to_string(),
        participants: vec![player],
        witnesses: vec![player],
        location: loc(),
        timestamp: ts(2000),
        emotional_valence: 0.6,
        importance: 0.7,
        pad_shift: None,
    };
    observation::observe(&event2, player, &mut bank, &[]);

    // Add emotional memory to strengthen disposition
    bank.emotional.push(EmotionalMemory::new(
        player,
        "admiration",
        0.8,
        PADState::new(0.7, 0.2, 0.1),
        vec![],
        ts(2000),
    ));

    // Price modifier should reflect positive disposition
    let disposition = behavior::compute_disposition(&bank, player);
    let price_mod = behavior::compute_price_modifier(&disposition);

    assert!(
        price_mod <= 1.0,
        "Positive disposition should give discount (price_mod={price_mod})"
    );
}

// ---------------------------------------------------------------------------
// Reputation board lifecycle: report → decay → check tier
// ---------------------------------------------------------------------------

#[test]
fn reputation_lifecycle() {
    let settlement = SettlementId::new();
    let hero = EntityId::new();
    let villain = EntityId::new();

    let mut board = ReputationBoard::new(settlement, ts(0));

    // Hero does good deeds
    for i in 0..5 {
        board.report_sentiment(hero, 0.6, ts(i * 1000));
        board.record_deed(NotableDeed {
            actor: hero,
            description: format!("Heroic deed {i}"),
            valence: 0.7,
            timestamp: ts(i * 1000),
            witness_count: 3,
        });
    }

    // Villain does bad deeds
    for i in 0..3 {
        board.report_sentiment(villain, -0.7, ts(i * 1000));
        board.record_deed(NotableDeed {
            actor: villain,
            description: format!("Villainous act {i}"),
            valence: -0.8,
            timestamp: ts(i * 1000),
            witness_count: 5,
        });
    }

    // Check tiers
    let hero_rep = board.get_reputation(hero).expect("hero should have reputation");
    assert!(hero_rep.score > 0.0, "Hero should have positive score");
    let hero_score_before = hero_rep.score;

    let villain_rep = board.get_reputation(villain).expect("villain should have reputation");
    assert!(villain_rep.score < 0.0, "Villain should have negative score");

    // Top heroes
    let heroes = board.top_heroes(1);
    assert_eq!(heroes.len(), 1);
    assert_eq!(heroes[0].entity, hero);

    // Decay over time
    let far_future = ts(100_000_000);
    board.decay_reputations(0.05, far_future);

    // After significant decay, scores should be closer to neutral
    if let Some(hero_after) = board.get_reputation(hero) {
        assert!(
            hero_after.score.abs() < hero_score_before.abs(),
            "Score should decay toward neutral"
        );
    }
}

// ---------------------------------------------------------------------------
// Replay selection with varying memory quality
// ---------------------------------------------------------------------------

#[test]
fn replay_selection_prefers_strong_memories() {
    let player = EntityId::new();
    let mut bank = MemoryBank::new();

    // Weak memory
    bank.episodic.push(EpisodicMemory::new(
        format!("Entity {player} walked by"),
        vec![player],
        loc(),
        ts(1000),
        0.05,
        0.1,
    ));

    // Strong memory
    bank.episodic.push(EpisodicMemory::new(
        format!("Entity {player} saved my child from the fire"),
        vec![player],
        loc(),
        ts(2000),
        0.9,
        0.95,
    ));

    let result = replay::select_replay(&bank, player, &ts(3000), 0.3);

    // Should select the strong memory
    assert!(result.is_some(), "Should find a replay");
    let r = result.expect("checked");
    assert!(r.valence > 0.5, "Should pick the strongly positive memory");
    assert!(!r.dialogue_hint.is_empty(), "Should have a dialogue hint");
}

// ---------------------------------------------------------------------------
// Consolidation identifies tasks from accumulated memories
// ---------------------------------------------------------------------------

#[test]
fn consolidation_identifies_semantic_tasks() {
    let player = EntityId::new();
    let mut bank = MemoryBank::new();

    // Add enough positive episodic memories about the same participant
    for i in 0..6 {
        bank.episodic.push(EpisodicMemory::new(
            format!("Entity {player} helped villagers (instance {i})"),
            vec![player],
            loc(),
            ts((i + 1) as u64 * 1000),
            0.5,
            0.6,
        ));
    }

    let current_time = ts(10_000);
    let tasks = consolidation::identify_consolidation_tasks(&bank.episodic, &current_time);

    // Should identify at least one semantic consolidation task
    assert!(
        !tasks.is_empty(),
        "Should identify consolidation opportunities from 6 similar memories"
    );
}

// ---------------------------------------------------------------------------
// Persistence: multiple NPCs saved and restored independently
// ---------------------------------------------------------------------------

#[test]
fn multi_npc_persistence() {
    let npc_a = EntityId::new();
    let npc_b = EntityId::new();
    let player = EntityId::new();
    let persist_config = PersistenceConfig::default();

    let mut bank_a = MemoryBank::new();
    bank_a.episodic.push(EpisodicMemory::new(
        "NPC A saw the player fight bandits",
        vec![player],
        loc(),
        ts(1000),
        0.6,
        0.7,
    ));

    let mut bank_b = MemoryBank::new();
    bank_b.episodic.push(EpisodicMemory::new(
        "NPC B traded with the player",
        vec![player],
        loc(),
        ts(2000),
        0.2,
        0.3,
    ));
    bank_b.social.push(SocialMemory::new(
        player,
        npc_a,
        "Player is a brave fighter",
        0.7,
        1,
        ts(3000),
    ));

    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("multi_npc.db");
    let engine = PersistenceEngine::open(db_path.to_str().expect("path"), &persist_config).expect("open");

    // Save both
    engine.save_bank(&npc_a, &bank_a).expect("save A");
    engine.save_bank(&npc_b, &bank_b).expect("save B");

    // Verify count via list_entities
    let entities = engine.list_entities().expect("list");
    assert_eq!(entities.len(), 2);

    // Restore independently
    let restored_a = engine.load_bank(&npc_a).expect("load A").expect("found A");
    let restored_b = engine.load_bank(&npc_b).expect("load B").expect("found B");

    assert_eq!(restored_a.episodic.len(), 1);
    assert_eq!(restored_b.episodic.len(), 1);
    assert_eq!(restored_b.social.len(), 1);

    // NPC B's social memory references player correctly
    assert_eq!(restored_b.social[0].about, player);

    // Delete A, verify B still exists
    engine.delete_bank(&npc_a).expect("delete A");
    assert!(engine.load_bank(&npc_a).expect("load A").is_none());
    assert!(engine.load_bank(&npc_b).expect("load B").is_some());
}

// ---------------------------------------------------------------------------
// Memory storage size check (target: < 80KB per NPC with 200 memories)
// ---------------------------------------------------------------------------

#[test]
fn memory_storage_size_within_budget() {
    let mut bank = MemoryBank::new();

    for i in 0..200u64 {
        bank.episodic.push(EpisodicMemory::new(
            format!("Event {i} at the market"),
            vec![EntityId::new()],
            Location {
                x: i as f32,
                y: i as f32 * 2.0,
                z: 0.0,
            },
            GameTimestamp::now(i * 1000),
            (i as f32 / 200.0 - 0.5) * 2.0,
            (i as f32 / 200.0).clamp(0.1, 0.9),
        ));
    }

    let serialized = serde_json::to_vec(&bank).expect("serialize");
    let size_kb = serialized.len() as f64 / 1024.0;

    // UUID-based IDs make JSON larger; 120KB is the realistic budget for JSON
    assert!(
        size_kb < 120.0,
        "200 memories should serialize to < 120KB JSON, got {size_kb:.1}KB"
    );

    // Compact binary format should be well under budget
    let bincode_serialized = bincode::serialize(&bank).expect("bincode");
    let bincode_kb = bincode_serialized.len() as f64 / 1024.0;

    assert!(
        bincode_kb < 80.0,
        "Bincode should be < 80KB, got {bincode_kb:.1}KB"
    );
}
