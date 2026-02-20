//! Ebbinghaus Forgetting Curve — Memory Decay Model (§12.4, §12.6)
//!
//! The Ebbinghaus curve models memory retention as:
//!   R = e^(-t/S)
//!
//! Where:
//!   R = retention (0.0 = forgotten, 1.0 = perfect recall)
//!   t = time since memory formation (game-days)
//!   S = memory strength (higher S = slower decay)
//!
//! Memory strength S is determined by:
//!   - Importance (0.0–1.0)
//!   - Emotional intensity (flashbulb memory effect)
//!   - Number of times the memory has been accessed (rehearsal effect)
//!   - Whether the memory is a "first meeting" (initial impression bias)
//!
//! Reference: Ebbinghaus, H. (1885). "Memory: A Contribution to Experimental Psychology."

use crate::config::MemoryConfig;
use crate::memory::episodic::EpisodicMemory;
use crate::memory::social::SocialMemory;
use crate::types::GameTimestamp;

/// Calculate the retention of an episodic memory using the Ebbinghaus forgetting curve.
///
/// Returns a value in (0.0, 1.0] where 1.0 is perfect retention.
#[must_use]
pub fn episodic_retention(memory: &EpisodicMemory, current_time: &GameTimestamp) -> f64 {
    let delta_ticks = current_time.tick.saturating_sub(memory.timestamp.tick);
    let delta_days = delta_ticks as f64 / 72_000.0;

    let strength = memory_strength(
        memory.importance,
        memory.emotional_valence.abs(),
        memory.access_count,
        memory.is_first_meeting,
    );

    ebbinghaus(delta_days, strength)
}

/// Calculate the retention of a social memory (gossip decays faster).
#[must_use]
pub fn social_retention(memory: &SocialMemory, current_time: &GameTimestamp) -> f64 {
    let delta_ticks = current_time.tick.saturating_sub(memory.received_at.tick);
    let delta_days = delta_ticks as f64 / 72_000.0;

    // Social memories decay faster (gossip is less durable than personal experience).
    let trust_factor = f64::from(memory.trust_in_source);
    let chain_penalty = 1.0 / (1.0 + memory.propagation_depth as f64);
    let strength = trust_factor * chain_penalty * 10.0; // base strength ~10 for trusted sources

    ebbinghaus(delta_days, strength)
}

/// Core Ebbinghaus forgetting curve: R = e^(-t/S).
///
/// # Arguments
/// * `delta_days` — Time elapsed in game-days.
/// * `strength` — Memory strength S (higher = slower decay).
#[must_use]
pub fn ebbinghaus(delta_days: f64, strength: f64) -> f64 {
    if strength <= 0.0 {
        return 0.0;
    }
    (-delta_days / strength).exp()
}

/// Compute memory strength S from contributing factors.
///
/// S = base_strength × (1 + importance) × (1 + emotional_intensity) × log2(1 + access_count) × first_meeting_bonus
#[must_use]
pub fn memory_strength(
    importance: f32,
    emotional_intensity: f32,
    access_count: u32,
    is_first_meeting: bool,
) -> f64 {
    let base: f64 = 10.0; // base strength: ~10 game-days to reach ~37% retention

    let importance_factor = 1.0 + f64::from(importance);
    let emotional_factor = 1.0 + f64::from(emotional_intensity); // flashbulb memory effect
    let rehearsal_factor = (1.0 + access_count as f64).log2().max(1.0); // spaced repetition
    let first_meeting_bonus = if is_first_meeting { 1.5 } else { 1.0 };

    base * importance_factor * emotional_factor * rehearsal_factor * first_meeting_bonus
}

/// Run a decay pass over a collection of episodic memories, removing those
/// below the retention threshold.
///
/// Performance target: < 50μs for 50 NPCs (§12.6).
pub fn decay_episodic_memories(
    memories: &mut Vec<EpisodicMemory>,
    current_time: &GameTimestamp,
    config: &MemoryConfig,
) {
    let threshold = f64::from(config.decay_rate);

    memories.retain(|memory| {
        // Protected memories never decay.
        if memory.is_first_meeting {
            return true;
        }
        if memory.emotional_valence.abs() > 0.8 {
            return true; // flashbulb memories
        }
        // Check retention against threshold.
        episodic_retention(memory, current_time) > threshold
    });
}

/// Run a decay pass over social memories.
pub fn decay_social_memories(
    memories: &mut Vec<SocialMemory>,
    current_time: &GameTimestamp,
    threshold: f64,
) {
    memories.retain(|memory| social_retention(memory, current_time) > threshold);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ebbinghaus_immediate_is_one() {
        let retention = ebbinghaus(0.0, 10.0);
        assert!((retention - 1.0).abs() < 0.001);
    }

    #[test]
    fn ebbinghaus_decays_over_time() {
        let r1 = ebbinghaus(1.0, 10.0);
        let r10 = ebbinghaus(10.0, 10.0);
        let r100 = ebbinghaus(100.0, 10.0);

        assert!(r1 > r10);
        assert!(r10 > r100);
        assert!(r1 > 0.9); // 1 day, strength 10 → ~90% retained
        assert!((r10 - (-1.0_f64).exp()).abs() < 0.001); // exactly e^(-1) ≈ 0.368
    }

    #[test]
    fn stronger_memories_decay_slower() {
        let r_weak = ebbinghaus(10.0, 5.0);
        let r_strong = ebbinghaus(10.0, 20.0);

        assert!(r_strong > r_weak, "Stronger memories should decay slower");
    }

    #[test]
    fn first_meeting_is_stronger() {
        let s_normal = memory_strength(0.5, 0.5, 1, false);
        let s_first = memory_strength(0.5, 0.5, 1, true);

        assert!(s_first > s_normal, "First-meeting memories should be stronger");
    }

    #[test]
    fn emotional_memories_are_stronger() {
        let s_calm = memory_strength(0.5, 0.1, 1, false);
        let s_intense = memory_strength(0.5, 0.9, 1, false);

        assert!(
            s_intense > s_calm,
            "Emotionally intense memories should be stronger"
        );
    }

    #[test]
    fn rehearsal_strengthens_memory() {
        let s_once = memory_strength(0.5, 0.5, 1, false);
        let s_many = memory_strength(0.5, 0.5, 100, false);

        assert!(
            s_many > s_once,
            "Frequently accessed memories should be stronger"
        );
    }
}
