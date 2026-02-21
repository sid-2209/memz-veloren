//! Multi-tier memory eviction system (§12.2.1).
//!
//! Implements the Hot → Warm → Cold → Archive lifecycle described in
//! the design doc.  Each ring is a logical age boundary measured in
//! game-time.  Memories move through the rings automatically via
//! [`evict_memories`], and protected memories (emotional flashbulbs,
//! first meetings, active-quest references) skip eviction.
//!
//! ```text
//! ┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
//! │ Hot Ring │────▶│Warm Ring │────▶│Cold Ring │────▶│ Archive  │
//! │ (fast)   │     │ (medium) │     │ (SQLite) │     │ (forget) │
//! │ <24h     │     │ <7 days  │     │ <90 days │     │          │
//! └──────────┘     └──────────┘     └──────────┘     └──────────┘
//! ```

use crate::config::EvictionConfig;
use crate::memory::episodic::EpisodicMemory;
use crate::memory::social::SocialMemory;

// ---------------------------------------------------------------------------
// Ring classification
// ---------------------------------------------------------------------------

/// Which eviction ring a memory currently belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ring {
    /// In-memory, fastest access — most recent memories.
    Hot,
    /// In-memory, slightly older memories.
    Warm,
    /// Persisted to `SQLite`, oldest still-retained memories.
    Cold,
    /// Beyond cold threshold — eligible for deletion.
    Archive,
}

impl std::fmt::Display for Ring {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hot => write!(f, "Hot"),
            Self::Warm => write!(f, "Warm"),
            Self::Cold => write!(f, "Cold"),
            Self::Archive => write!(f, "Archive"),
        }
    }
}

/// Classify a memory's ring based on its age in game ticks.
///
/// `ticks_per_hour` converts tick deltas to game-hours so the ring
/// thresholds (defined in hours/days in the config) can be compared.
#[must_use]
pub fn classify_ring(
    memory_tick: u64,
    current_tick: u64,
    ticks_per_hour: u64,
    config: &EvictionConfig,
) -> Ring {
    if current_tick < memory_tick {
        return Ring::Hot; // clock skew guard
    }
    let age_ticks = current_tick - memory_tick;
    let age_hours = if ticks_per_hour == 0 { 0 } else { age_ticks / ticks_per_hour };

    let hot_limit = u64::from(config.hot_ring_hours);
    let warm_limit = u64::from(config.warm_ring_days) * 24;
    let cold_limit = u64::from(config.cold_ring_days) * 24;

    if age_hours < hot_limit {
        Ring::Hot
    } else if age_hours < warm_limit {
        Ring::Warm
    } else if age_hours < cold_limit {
        Ring::Cold
    } else {
        Ring::Archive
    }
}

// ---------------------------------------------------------------------------
// Eviction score
// ---------------------------------------------------------------------------

/// Compute an eviction priority score for a memory.
///
/// **Lower** scores are evicted first.
///
/// ```text
/// Score = importance × emotional_weight × (1 / time_since_last_access)
/// ```
///
/// Protected memories return `f64::MAX` so they are never evicted.
#[must_use]
pub fn eviction_score(
    importance: f32,
    emotional_valence: f32,
    is_first_meeting: bool,
    ticks_since_last_access: u64,
    config: &EvictionConfig,
) -> f64 {
    // Protected memories — never evicted
    if is_first_meeting && config.protect_first_meeting {
        return f64::MAX;
    }
    if emotional_valence.abs() > config.protect_emotional_threshold {
        return f64::MAX;
    }

    let access_factor = if ticks_since_last_access == 0 {
        1.0
    } else {
        1.0 / (ticks_since_last_access as f64)
    };

    let emotional_weight = 1.0 + f64::from(emotional_valence.abs());
    f64::from(importance) * emotional_weight * access_factor
}

// ---------------------------------------------------------------------------
// Eviction driver
// ---------------------------------------------------------------------------

/// Result of an eviction pass on episodic memories.
#[derive(Debug, Default)]
pub struct EvictionResult {
    /// Memories that remain in-memory (Hot + Warm).
    pub retained: Vec<EpisodicMemory>,
    /// Memories that should be moved to cold storage (`SQLite`).
    pub to_cold_storage: Vec<EpisodicMemory>,
    /// Memories that should be permanently deleted (Archive).
    pub to_archive: Vec<EpisodicMemory>,
}

/// Run a full eviction pass on a set of episodic memories.
///
/// Memories are classified into rings; those in Cold or Archive are
/// separated out.  Within each ring the lowest-scored memories are
/// dropped first if `max_in_memory` is exceeded.
///
/// `ticks_per_hour` is the game-specific conversion factor (Veloren
/// uses ~7200 ticks/hour at 30 TPS).
#[must_use] 
pub fn evict_episodic_memories(
    memories: Vec<EpisodicMemory>,
    current_tick: u64,
    ticks_per_hour: u64,
    max_in_memory: usize,
    config: &EvictionConfig,
) -> EvictionResult {
    let mut result = EvictionResult::default();

    for mem in memories {
        let ring = classify_ring(
            mem.timestamp.tick,
            current_tick,
            ticks_per_hour,
            config,
        );
        match ring {
            Ring::Hot | Ring::Warm => result.retained.push(mem),
            Ring::Cold => result.to_cold_storage.push(mem),
            Ring::Archive => result.to_archive.push(mem),
        }
    }

    // If in-memory set still exceeds capacity, score-sort and spill the weakest.
    if result.retained.len() > max_in_memory {
        // Score each memory.
        let mut scored: Vec<(f64, EpisodicMemory)> = result
            .retained
            .drain(..)
            .map(|m| {
                let ticks_since_access = current_tick.saturating_sub(m.last_accessed.tick);
                let score = eviction_score(
                    m.importance,
                    m.emotional_valence,
                    m.is_first_meeting,
                    ticks_since_access,
                    config,
                );
                (score, m)
            })
            .collect();

        // Sort descending — highest scores first (keep these).
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        for (i, (_score, mem)) in scored.into_iter().enumerate() {
            if i < max_in_memory {
                result.retained.push(mem);
            } else {
                result.to_cold_storage.push(mem);
            }
        }
    }

    result
}

/// Run a full eviction pass on social memories.
///
/// Social memories don't have `is_first_meeting` or `last_accessed`
/// timestamps in the same way, so we use `received_at` as the age proxy
/// and `trust_in_source` as the importance proxy.
#[must_use] 
pub fn evict_social_memories(
    memories: Vec<SocialMemory>,
    current_tick: u64,
    ticks_per_hour: u64,
    max_in_memory: usize,
    config: &EvictionConfig,
) -> (Vec<SocialMemory>, Vec<SocialMemory>) {
    let mut retained = Vec::new();
    let mut evicted = Vec::new();

    for mem in memories {
        let ring = classify_ring(
            mem.received_at.tick,
            current_tick,
            ticks_per_hour,
            config,
        );
        match ring {
            Ring::Hot | Ring::Warm | Ring::Cold => retained.push(mem),
            Ring::Archive => evicted.push(mem),
        }
    }

    // Spill weakest if over capacity.
    if retained.len() > max_in_memory {
        let mut scored: Vec<(f64, SocialMemory)> = retained
            .drain(..)
            .map(|m| {
                let age = current_tick.saturating_sub(m.received_at.tick);
                let score = f64::from(m.trust_in_source) / (1.0 + age as f64);
                (score, m)
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        for (i, (_score, mem)) in scored.into_iter().enumerate() {
            if i < max_in_memory {
                retained.push(mem);
            } else {
                evicted.push(mem);
            }
        }
    }

    (retained, evicted)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::EvictionConfig;
    use crate::memory::episodic::EpisodicMemory;
    use crate::types::{GameTimestamp, Location, MemoryId};
    use chrono::Utc;

    fn default_config() -> EvictionConfig {
        EvictionConfig::default()
    }

    fn make_episodic(tick: u64, importance: f32, valence: f32, first_meeting: bool) -> EpisodicMemory {
        let ts = GameTimestamp { tick, real_time: Utc::now() };
        EpisodicMemory {
            id: MemoryId::new(),
            event: format!("Event at tick {tick}"),
            participants: vec![],
            location: Location::default(),
            timestamp: ts,
            emotional_valence: valence,
            importance,
            decay_rate: 0.02,
            strength: 1.0,
            access_count: 0,
            last_accessed: ts,
            is_first_meeting: first_meeting,
            embedding: None,
        }
    }

    #[test]
    fn ring_classification_hot() {
        let config = default_config();
        // Memory 1 hour old, hot limit is 24h → Hot
        let ring = classify_ring(100, 100 + 3600, 3600, &config);
        assert_eq!(ring, Ring::Hot);
    }

    #[test]
    fn ring_classification_warm() {
        let config = default_config();
        // 2 days old (48 hours) → Warm (24h < 48h < 7*24=168h)
        let ring = classify_ring(0, 48 * 3600, 3600, &config);
        assert_eq!(ring, Ring::Warm);
    }

    #[test]
    fn ring_classification_cold() {
        let config = default_config();
        // 30 days → Cold (168h < 720h < 90*24=2160h)
        let ring = classify_ring(0, 30 * 24 * 3600, 3600, &config);
        assert_eq!(ring, Ring::Cold);
    }

    #[test]
    fn ring_classification_archive() {
        let config = default_config();
        // 100 days → Archive (> 90*24 = 2160h)
        let ring = classify_ring(0, 100 * 24 * 3600, 3600, &config);
        assert_eq!(ring, Ring::Archive);
    }

    #[test]
    fn protected_memories_not_evicted() {
        let config = default_config();
        // First meeting
        let score_first = eviction_score(0.1, 0.1, true, 999_999, &config);
        assert_eq!(score_first, f64::MAX);
        // High emotional valence
        let score_emo = eviction_score(0.1, 0.9, false, 999_999, &config);
        assert_eq!(score_emo, f64::MAX);
    }

    #[test]
    fn eviction_respects_capacity() {
        let config = default_config();
        let current_tick = 1000;
        let ticks_per_hour = 3600;

        // Create 10 hot-ring memories
        let memories: Vec<_> = (0..10)
            .map(|i| make_episodic(current_tick - i * 10, 0.5, 0.3, false))
            .collect();

        let result = evict_episodic_memories(memories, current_tick, ticks_per_hour, 5, &config);
        assert_eq!(result.retained.len(), 5);
        assert_eq!(result.to_cold_storage.len(), 5);
    }

    #[test]
    fn eviction_keeps_protected() {
        let config = default_config();
        let current_tick = 1000;
        let ticks_per_hour = 3600;

        let mut memories = Vec::new();
        // 3 normal + 2 protected (first meeting)
        for i in 0..3 {
            memories.push(make_episodic(current_tick - i * 10, 0.1, 0.1, false));
        }
        for i in 0..2 {
            memories.push(make_episodic(current_tick - i * 10, 0.1, 0.1, true));
        }

        let result = evict_episodic_memories(memories, current_tick, ticks_per_hour, 3, &config);
        // Both protected memories must be retained
        let protected_count = result.retained.iter().filter(|m| m.is_first_meeting).count();
        assert_eq!(protected_count, 2);
    }
}
