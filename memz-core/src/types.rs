//! Core type definitions for the MEMZ memory system.
//!
//! All types are serializable and designed for zero-copy where possible.

use chrono::{DateTime, Utc};
use ordered_float::OrderedFloat;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Identity Types
// ---------------------------------------------------------------------------

/// Unique identifier for any entity (NPC, player, creature) in the game world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(pub Uuid);

impl EntityId {
    /// Create a new random entity ID.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for EntityId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for a memory entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemoryId(pub Uuid);

impl MemoryId {
    /// Create a new random memory ID.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for MemoryId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for MemoryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a settlement / location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SettlementId(pub Uuid);

// ---------------------------------------------------------------------------
// Spatial
// ---------------------------------------------------------------------------

/// A 3D position in the game world.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Location {
    /// X coordinate.
    pub x: f32,
    /// Y coordinate.
    pub y: f32,
    /// Z coordinate.
    pub z: f32,
}

impl Default for Location {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({:.1}, {:.1}, {:.1})", self.x, self.y, self.z)
    }
}

/// Runtime statistics for a single entity's memory system.
#[derive(Debug, Clone, Copy, Default)]
pub struct MemoryStats {
    /// Microseconds taken for the last retrieval query.
    pub last_retrieval_us: u64,
    /// Total number of memories across all types.
    pub total_memories: u32,
    /// Number of pending reflection operations.
    pub pending_reflections: u32,
    /// Last game tick when decay was run.
    pub last_decay_tick: u64,
}

// ---------------------------------------------------------------------------
// Time
// ---------------------------------------------------------------------------

/// In-game timestamp measured in game-ticks since world creation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GameTimestamp {
    /// Game tick (monotonically increasing).
    pub tick: u64,
    /// Corresponding real-world wall-clock time (for save metadata).
    pub real_time: DateTime<Utc>,
}

impl GameTimestamp {
    /// Create a new game timestamp at the current wall-clock time.
    #[must_use]
    pub fn now(tick: u64) -> Self {
        Self {
            tick,
            real_time: Utc::now(),
        }
    }

    /// Game-days elapsed since `other`, assuming 24 000 ticks per game-day.
    #[must_use]
    pub fn days_since(&self, other: &Self) -> f32 {
        const TICKS_PER_DAY: f32 = 24_000.0;
        (self.tick.saturating_sub(other.tick)) as f32 / TICKS_PER_DAY
    }

    /// Game-hours elapsed since `other`, assuming 1 000 ticks per game-hour.
    #[must_use]
    pub fn hours_since(&self, other: &Self) -> f32 {
        const TICKS_PER_HOUR: f32 = 1_000.0;
        (self.tick.saturating_sub(other.tick)) as f32 / TICKS_PER_HOUR
    }
}

// ---------------------------------------------------------------------------
// Emotional Model — PAD (Pleasure-Arousal-Dominance)
// ---------------------------------------------------------------------------

/// PAD emotional state based on Russell & Mehrabian (1977).
///
/// Each axis ranges from -1.0 to 1.0:
/// - **Pleasure**: unhappy (-1) → happy (+1)
/// - **Arousal**: calm (-1) → excited (+1)
/// - **Dominance**: submissive (-1) → dominant (+1)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PADState {
    /// Unhappy (-1.0) to happy (+1.0).
    pub pleasure: f32,
    /// Calm (-1.0) to excited (+1.0).
    pub arousal: f32,
    /// Submissive (-1.0) to dominant (+1.0).
    pub dominance: f32,
}

impl PADState {
    /// Neutral emotional state.
    pub const NEUTRAL: Self = Self {
        pleasure: 0.0,
        arousal: 0.0,
        dominance: 0.0,
    };

    /// Create a new PAD state, clamping values to [-1, 1].
    #[must_use]
    pub fn new(pleasure: f32, arousal: f32, dominance: f32) -> Self {
        Self {
            pleasure: pleasure.clamp(-1.0, 1.0),
            arousal: arousal.clamp(-1.0, 1.0),
            dominance: dominance.clamp(-1.0, 1.0),
        }
    }

    /// Overall emotional intensity (magnitude of the PAD vector).
    #[must_use]
    pub fn intensity(&self) -> f32 {
        (self.pleasure * self.pleasure
            + self.arousal * self.arousal
            + self.dominance * self.dominance)
            .sqrt()
    }

    /// Blend two PAD states with a weight (0.0 = self, 1.0 = other).
    #[must_use]
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self::new(
            self.pleasure + (other.pleasure - self.pleasure) * t,
            self.arousal + (other.arousal - self.arousal) * t,
            self.dominance + (other.dominance - self.dominance) * t,
        )
    }
}

impl Default for PADState {
    fn default() -> Self {
        Self::NEUTRAL
    }
}

// ---------------------------------------------------------------------------
// Personality Traits
// ---------------------------------------------------------------------------

/// Big-Five-inspired personality traits. Each ranges 0.0–1.0.
/// Used to modulate memory formation, retrieval weights, and behavior.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PersonalityTraits {
    /// How easily the NPC believes rumors (0 = skeptic, 1 = gullible).
    pub credulity: f32,
    /// How open to new experiences and ideas (0 = rigid, 1 = flexible).
    pub openness: f32,
    /// How likely to share information with others (0 = secretive, 1 = gossiper).
    pub gossip_tendency: f32,
    /// How strongly events affect emotional state (0 = stoic, 1 = volatile).
    pub emotional_volatility: f32,
    /// How brave / confrontational (0 = cowardly, 1 = fearless).
    pub bravery: f32,
}

impl Default for PersonalityTraits {
    fn default() -> Self {
        Self {
            credulity: 0.5,
            openness: 0.5,
            gossip_tendency: 0.5,
            emotional_volatility: 0.5,
            bravery: 0.5,
        }
    }
}

// ---------------------------------------------------------------------------
// Memory Embedding Vector
// ---------------------------------------------------------------------------

/// A dense vector embedding for semantic similarity search.
/// Typically 384 dimensions (all-MiniLM-L6-v2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding(pub Vec<f32>);

impl Embedding {
    /// Cosine similarity between two embeddings.
    /// Returns 0.0 if either vector is zero-length.
    #[must_use]
    pub fn cosine_similarity(&self, other: &Self) -> f32 {
        if self.0.len() != other.0.len() || self.0.is_empty() {
            return 0.0;
        }
        let (mut dot, mut norm_a, mut norm_b) = (0.0_f32, 0.0_f32, 0.0_f32);
        for (a, b) in self.0.iter().zip(other.0.iter()) {
            dot += a * b;
            norm_a += a * a;
            norm_b += b * b;
        }
        let denom = norm_a.sqrt() * norm_b.sqrt();
        if denom < f32::EPSILON {
            0.0
        } else {
            dot / denom
        }
    }

    /// Dimensionality of the embedding.
    #[must_use]
    pub fn dimensions(&self) -> usize {
        self.0.len()
    }
}

// ---------------------------------------------------------------------------
// Retrieval Score
// ---------------------------------------------------------------------------

/// Composite score used to rank memories during retrieval.
/// See §12.4 of the design doc for the scoring formula.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct RetrievalScore(pub OrderedFloat<f32>);

impl RetrievalScore {
    /// Create a retrieval score from a raw f32.
    #[must_use]
    pub fn new(score: f32) -> Self {
        Self(OrderedFloat(score))
    }

    /// Get the raw score value.
    #[must_use]
    pub fn value(self) -> f32 {
        self.0.into_inner()
    }
}
