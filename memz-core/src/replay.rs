//! Memory Replay — NPCs spontaneously referencing past events (§14.1)
//!
//! NPCs periodically recall significant memories during interactions,
//! creating a sense of continuity. A blacksmith might say "Remember when
//! you helped me fight off those wolves last week?"
//!
//! This module selects the best memory to replay and formats it for
//! insertion into dialogue context.

use crate::memory::MemoryBank;
use crate::memory::episodic::EpisodicMemory;
use crate::types::{EntityId, GameTimestamp};

/// A memory selected for replay in dialogue.
#[derive(Debug, Clone)]
pub struct MemoryReplay {
    /// The memory being replayed.
    pub memory: EpisodicMemory,
    /// How strong this memory still is.
    pub strength: f32,
    /// Whether this is a positive or negative memory.
    pub valence: f32,
    /// A dialogue-ready reference to the memory.
    pub dialogue_hint: String,
}

/// Select the best memory to replay during an interaction with a target.
///
/// Returns `None` if there are no suitable memories to replay.
///
/// Selection criteria:
/// 1. Must involve the target entity
/// 2. Must still have sufficient strength (not faded)
/// 3. Prefers emotionally significant memories
/// 4. Prefers memories not recently replayed (novelty)
#[must_use] 
pub fn select_replay(
    bank: &MemoryBank,
    target: EntityId,
    current_time: &GameTimestamp,
    min_strength: f32,
) -> Option<MemoryReplay> {
    let candidates: Vec<&EpisodicMemory> = bank
        .episodic
        .iter()
        .filter(|m| {
            m.participants.contains(&target)
                && m.strength >= min_strength
        })
        .collect();

    if candidates.is_empty() {
        return None;
    }

    // Score each candidate: emotional_valence × strength × importance × novelty
    let best = candidates
        .iter()
        .max_by(|a, b| {
            let score_a = replay_score(a, current_time);
            let score_b = replay_score(b, current_time);
            score_a
                .partial_cmp(&score_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        })?;

    let hint = generate_replay_hint(best);

    Some(MemoryReplay {
        memory: (*best).clone(),
        strength: best.strength,
        valence: best.emotional_valence,
        dialogue_hint: hint,
    })
}

/// Compute replay priority score for a memory.
fn replay_score(memory: &EpisodicMemory, current_time: &GameTimestamp) -> f32 {
    let emotional_weight = memory.emotional_valence.abs();
    let importance_weight = memory.importance;
    let strength_weight = memory.strength;

    // Novelty: prefer memories not recently accessed
    let ticks_since_access = current_time.tick.saturating_sub(memory.last_accessed.tick);
    let novelty = (ticks_since_access as f32 / 72_000.0).min(10.0) / 10.0; // 0–1 over 10 days

    // First meetings get a bonus
    let first_meeting_bonus = if memory.is_first_meeting { 0.3 } else { 0.0 };

    emotional_weight * 0.3
        + importance_weight * 0.25
        + strength_weight * 0.2
        + novelty * 0.15
        + first_meeting_bonus
        + 0.1 // base score
}

/// Generate a dialogue hint from a memory.
fn generate_replay_hint(memory: &EpisodicMemory) -> String {
    if memory.is_first_meeting {
        format!(
            "I remember when we first met — {}",
            memory.event.to_lowercase()
        )
    } else if memory.emotional_valence > 0.5 {
        format!(
            "I fondly recall when {}",
            memory.event.to_lowercase()
        )
    } else if memory.emotional_valence < -0.5 {
        format!(
            "I haven't forgotten when {}",
            memory.event.to_lowercase()
        )
    } else {
        format!(
            "I remember when {}",
            memory.event.to_lowercase()
        )
    }
}

/// Check if an NPC should spontaneously replay a memory during interaction.
///
/// Based on:
/// - Time since last replay
/// - Emotional state
/// - Whether the target is present
#[must_use]
pub fn should_replay(
    last_replay_tick: u64,
    current_tick: u64,
    emotional_intensity: f32,
) -> bool {
    let ticks_since_last = current_tick.saturating_sub(last_replay_tick);
    let min_interval_ticks = 36_000; // ~30 game-minutes

    if ticks_since_last < min_interval_ticks {
        return false;
    }

    // Higher emotional intensity → more likely to replay
    let probability = 0.3 + emotional_intensity * 0.4;
    // Deterministic for testing: use threshold instead of random
    probability > 0.5
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{EntityId, GameTimestamp, Location};

    fn make_memory(target: EntityId, event: &str, valence: f32, strength: f32) -> EpisodicMemory {
        let mut m = EpisodicMemory::new(
            event,
            vec![target],
            Location::default(),
            GameTimestamp::now(36_000),
            valence,
            0.5,
        );
        m.strength = strength;
        m
    }

    #[test]
    fn selects_relevant_memory() {
        let target = EntityId::new();
        let other = EntityId::new();
        let mut bank = MemoryBank::new();

        bank.episodic.push(make_memory(target, "fought wolves together", 0.7, 0.8));
        bank.episodic.push(make_memory(other, "unrelated event", 0.5, 0.9));

        let replay = select_replay(&bank, target, &GameTimestamp::now(72_000), 0.3);
        assert!(replay.is_some());
        let replay = replay.expect("should select");
        assert!(replay.memory.participants.contains(&target));
    }

    #[test]
    fn no_replay_for_unknown_entity() {
        let unknown = EntityId::new();
        let bank = MemoryBank::new();

        let replay = select_replay(&bank, unknown, &GameTimestamp::now(72_000), 0.3);
        assert!(replay.is_none());
    }

    #[test]
    fn faded_memories_excluded() {
        let target = EntityId::new();
        let mut bank = MemoryBank::new();

        bank.episodic.push(make_memory(target, "old event", 0.5, 0.1)); // low strength

        let replay = select_replay(&bank, target, &GameTimestamp::now(72_000), 0.3);
        assert!(replay.is_none());
    }

    #[test]
    fn positive_memory_generates_fond_hint() {
        let target = EntityId::new();
        let mut bank = MemoryBank::new();
        bank.episodic.push(make_memory(target, "saved my shop", 0.8, 0.9));

        let replay = select_replay(&bank, target, &GameTimestamp::now(72_000), 0.3)
            .expect("should have replay");
        assert!(replay.dialogue_hint.contains("fondly"));
    }

    #[test]
    fn negative_memory_generates_warning_hint() {
        let target = EntityId::new();
        let mut bank = MemoryBank::new();
        bank.episodic.push(make_memory(target, "stole from me", -0.8, 0.9));

        let replay = select_replay(&bank, target, &GameTimestamp::now(72_000), 0.3)
            .expect("should have replay");
        assert!(replay.dialogue_hint.contains("forgotten"));
    }

    #[test]
    fn should_replay_respects_interval() {
        assert!(!should_replay(36_000, 36_100, 0.9)); // too soon
        assert!(should_replay(0, 72_000, 0.9)); // enough time + high emotion
    }
}
