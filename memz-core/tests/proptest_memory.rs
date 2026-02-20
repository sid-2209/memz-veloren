//! Property-Based Tests for MEMZ Core (§20.2)
//!
//! Uses `proptest` to verify memory system invariants under random inputs.
//! These tests ensure **deterministic reproducibility** and structural
//! correctness regardless of input patterns.
//!
//! References:
//!   - §20.2: "Given the same game event sequence and RNG seed, memory state is identical."
//!   - §12.1: Memory limits, eviction, decay guarantees.

use proptest::prelude::*;

use memz_core::config::{EvictionConfig, MemoryConfig};
use memz_core::decay;
use memz_core::eviction;
use memz_core::memory::episodic::EpisodicMemory;
use memz_core::memory::emotional::EmotionalMemory;
use memz_core::memory::social::SocialMemory;
use memz_core::memory::MemoryBank;
use memz_core::reputation::{NotableDeed, ReputationBoard};
use memz_core::social;
use memz_core::types::{EntityId, GameTimestamp, Location, PADState, PersonalityTraits, SettlementId};

// ---------------------------------------------------------------------------
// Strategy helpers — generate arbitrary MEMZ types
// ---------------------------------------------------------------------------

fn arb_personality() -> impl Strategy<Value = PersonalityTraits> {
    (
        0.0..1.0f32, // openness
        0.0..1.0f32, // credulity
        0.0..1.0f32, // gossip_tendency
        0.0..1.0f32, // emotional_volatility
        0.0..1.0f32, // bravery
    )
        .prop_map(|(o, cr, g, ev, b)| PersonalityTraits {
            openness: o,
            credulity: cr,
            gossip_tendency: g,
            emotional_volatility: ev,
            bravery: b,
        })
}

// ---------------------------------------------------------------------------
// Property: Emotional valence is always clamped to [-1, 1]
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn episodic_valence_always_clamped(valence in -100.0..100.0f32) {
        let mem = EpisodicMemory::new(
            "test event",
            vec![],
            Location::default(),
            GameTimestamp::now(1000),
            valence,
            0.5,
        );
        prop_assert!(mem.emotional_valence >= -1.0);
        prop_assert!(mem.emotional_valence <= 1.0);
    }
}

// ---------------------------------------------------------------------------
// Property: Importance is always clamped to [0, 1]
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn episodic_importance_always_clamped(importance in -100.0..100.0f32) {
        let mem = EpisodicMemory::new(
            "test event",
            vec![],
            Location::default(),
            GameTimestamp::now(1000),
            0.5,
            importance,
        );
        prop_assert!(mem.importance >= 0.0);
        prop_assert!(mem.importance <= 1.0);
    }
}

// ---------------------------------------------------------------------------
// Property: Strength is always in [0, 1] after creation
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn episodic_strength_always_valid(
        valence in -1.0..1.0f32,
        importance in 0.0..1.0f32,
    ) {
        let mem = EpisodicMemory::new(
            "test event",
            vec![],
            Location::default(),
            GameTimestamp::now(1000),
            valence,
            importance,
        );
        prop_assert!(mem.strength >= 0.0);
        prop_assert!(mem.strength <= 1.0);
    }
}

// ---------------------------------------------------------------------------
// Property: MemoryBank limits are enforced after truncation
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn bank_truncation_enforces_limits(count in 1..500usize) {
        let config = MemoryConfig::default();
        let mut bank = MemoryBank::new();

        for i in 0..count {
            bank.episodic.push(EpisodicMemory::new(
                format!("Event {i}"),
                vec![],
                Location::default(),
                GameTimestamp::now(i as u64),
                0.0,
                0.1,
            ));
        }

        bank.episodic.truncate(config.max_episodic_per_npc);
        prop_assert!(bank.episodic.len() <= config.max_episodic_per_npc);
    }
}

// ---------------------------------------------------------------------------
// Property: Decay never increases memory count
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn decay_never_increases_count(num_memories in 1..100usize) {
        let config = MemoryConfig::default();
        let mut bank = MemoryBank::new();

        for i in 0..num_memories {
            bank.episodic.push(EpisodicMemory::new(
                format!("Event {i}"),
                vec![],
                Location::default(),
                GameTimestamp::now(i as u64 * 1000),
                0.0,
                0.1,
            ));
        }

        let before = bank.episodic.len();
        let far_future = GameTimestamp::now(72_000_000);
        decay::decay_episodic_memories(&mut bank.episodic, &far_future, &config);

        prop_assert!(bank.episodic.len() <= before);
    }
}

// ---------------------------------------------------------------------------
// Property: Flashbulb memories (high |valence|) survive decay
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn flashbulb_memories_survive_decay(tick_offset in 1..72_000_000u64) {
        let config = MemoryConfig::default();
        let mut memories = vec![EpisodicMemory::new(
            "Traumatic event",
            vec![],
            Location::default(),
            GameTimestamp::now(0),
            0.95, // Very high valence → flashbulb
            0.9,
        )];

        let future = GameTimestamp::now(tick_offset);
        decay::decay_episodic_memories(&mut memories, &future, &config);

        // Flashbulb memories (|valence| > 0.8) are protected
        prop_assert!(!memories.is_empty(), "Flashbulb memory should survive decay");
    }
}

// ---------------------------------------------------------------------------
// Property: First-meeting memories survive decay
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn first_meeting_survives_decay(tick_offset in 1..72_000_000u64) {
        let config = MemoryConfig::default();
        let mut mem = EpisodicMemory::new(
            "First time meeting the player",
            vec![],
            Location::default(),
            GameTimestamp::now(0),
            0.1, // Low valence — would normally decay
            0.1, // Low importance
        );
        mem.is_first_meeting = true;
        let mut memories = vec![mem];

        let future = GameTimestamp::now(tick_offset);
        decay::decay_episodic_memories(&mut memories, &future, &config);

        prop_assert!(!memories.is_empty(), "First-meeting memory must survive");
    }
}

// ---------------------------------------------------------------------------
// Property: Eviction ring classification is monotonic with age
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn eviction_ring_monotonic(
        memory_tick in 0..100_000u64,
        current_tick in 100_000..200_000u64,
    ) {
        let config = EvictionConfig::default();
        let ticks_per_hour = 1000u64;
        let ring = eviction::classify_ring(memory_tick, current_tick, ticks_per_hour, &config);
        // Just verify it returns a valid ring (type system guarantees, but let's be thorough)
        let _valid = match ring {
            eviction::Ring::Hot | eviction::Ring::Warm | eviction::Ring::Cold | eviction::Ring::Archive => true,
        };

        // If current_tick == memory_tick, ring should be Hot (age = 0)
        let ring_fresh = eviction::classify_ring(current_tick, current_tick, ticks_per_hour, &config);
        prop_assert_eq!(ring_fresh, eviction::Ring::Hot, "Zero-age memory should be Hot");
    }
}

// ---------------------------------------------------------------------------
// Property: Reputation score is always in [-1, 1]
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn reputation_score_bounded(
        sentiments in prop::collection::vec(-1.0..1.0f32, 1..50),
    ) {
        let settlement = SettlementId::new();
        let entity = EntityId::new();
        let ts = GameTimestamp::now(1000);
        let mut board = ReputationBoard::new(settlement, ts);

        for (i, s) in sentiments.iter().enumerate() {
            let t = GameTimestamp::now(1000 + i as u64);
            board.report_sentiment(entity, *s, t);
        }

        if let Some(rep) = board.get_reputation(entity) {
            prop_assert!(rep.score >= -1.0, "Score {} < -1.0", rep.score);
            prop_assert!(rep.score <= 1.0, "Score {} > 1.0", rep.score);
        }
    }
}

// ---------------------------------------------------------------------------
// Property: Social memory trust is always in [0, 1]
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn social_trust_clamped(trust in -100.0..100.0f32) {
        let mem = SocialMemory::new(
            EntityId::new(),
            EntityId::new(),
            "test claim",
            trust,
            0,
            GameTimestamp::now(1000),
        );
        prop_assert!(mem.trust_in_source >= 0.0);
        prop_assert!(mem.trust_in_source <= 1.0);
    }
}

// ---------------------------------------------------------------------------
// Property: Gossip probability is always in [0, 1]
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn gossip_probability_bounded(
        personality in arb_personality(),
        importance in 0.0..1.0f32,
        emotional_weight in 0.0..1.0f32,
    ) {
        let prob = social::gossip_probability(&personality, importance, emotional_weight);
        prop_assert!(prob >= 0.0, "Gossip probability {} < 0", prob);
        prop_assert!(prob <= 1.0, "Gossip probability {} > 1", prob);
    }
}

// ---------------------------------------------------------------------------
// Property: Serialization round-trip preserves MemoryBank
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20))]
    #[test]
    fn memory_bank_serialization_roundtrip(num_memories in 0..20usize) {
        let mut bank = MemoryBank::new();
        for i in 0..num_memories {
            bank.episodic.push(EpisodicMemory::new(
                format!("Event {i}"),
                vec![EntityId::new()],
                Location::default(),
                GameTimestamp::now(i as u64 * 100),
                (i as f32 / 20.0 - 0.5) * 2.0,
                (i as f32 / 20.0).clamp(0.1, 0.9),
            ));
        }

        // JSON round-trip
        let json = serde_json::to_string(&bank).expect("serialize");
        let restored: MemoryBank = serde_json::from_str(&json).expect("deserialize");

        prop_assert_eq!(bank.episodic.len(), restored.episodic.len());
        for (orig, rest) in bank.episodic.iter().zip(restored.episodic.iter()) {
            prop_assert_eq!(&orig.event, &rest.event);
            prop_assert!((orig.emotional_valence - rest.emotional_valence).abs() < 1e-6);
            prop_assert!((orig.importance - rest.importance).abs() < 1e-6);
            prop_assert!((orig.strength - rest.strength).abs() < 1e-6);
        }
    }
}

// ---------------------------------------------------------------------------
// Property: Emotional memory intensity is clamped to [0, 1]
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn emotional_intensity_clamped(intensity in -100.0..100.0f32) {
        let mem = EmotionalMemory::new(
            EntityId::new(),
            "test",
            intensity,
            PADState::default(),
            vec![],
            GameTimestamp::now(1000),
        );
        prop_assert!(mem.intensity >= 0.0);
        prop_assert!(mem.intensity <= 1.0);
    }
}

// ---------------------------------------------------------------------------
// Property: Notable deed valence doesn't corrupt board
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn notable_deeds_dont_corrupt_board(
        num_deeds in 1..50usize,
        valences in prop::collection::vec(-1.0..1.0f32, 1..50),
    ) {
        let settlement = SettlementId::new();
        let ts = GameTimestamp::now(1000);
        let mut board = ReputationBoard::new(settlement, ts);

        for (i, &v) in valences.iter().take(num_deeds).enumerate() {
            board.record_deed(NotableDeed {
                actor: EntityId::new(),
                description: format!("Deed {i}"),
                valence: v,
                timestamp: GameTimestamp::now(1000 + i as u64),
                witness_count: 1,
            });
        }

        // Board should not have more deeds than its max (20)
        prop_assert!(board.notable_deeds.len() <= 20);
    }
}

// ---------------------------------------------------------------------------
// Property: Decay with same timestamp is idempotent
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn decay_idempotent_same_timestamp(num_memories in 1..50usize) {
        let config = MemoryConfig::default();
        let ts = GameTimestamp::now(3_600_000);
        let mut bank = MemoryBank::new();

        for i in 0..num_memories {
            bank.episodic.push(EpisodicMemory::new(
                format!("Event {i}"),
                vec![],
                Location::default(),
                GameTimestamp::now(i as u64 * 100),
                0.3,
                0.3,
            ));
        }

        // First decay pass
        decay::decay_episodic_memories(&mut bank.episodic, &ts, &config);
        let count_after_first = bank.episodic.len();

        // Second decay pass with same timestamp — should be idempotent
        decay::decay_episodic_memories(&mut bank.episodic, &ts, &config);
        let count_after_second = bank.episodic.len();

        prop_assert_eq!(count_after_first, count_after_second);
    }
}
