//! Memory Retrieval — HNSW-powered semantic search + multi-factor scoring (§12.4)
//!
//! The retrieval algorithm combines five weighted factors:
//!   Score = w₁·Recency + w₂·Relevance + w₃·Importance + w₄·Emotional + w₅·Social
//!
//! Based on the Stanford Generative Agents retrieval function, enhanced with
//! Ebbinghaus-curve decay, emotional flashbulb effect, trust-weighted hearsay,
//! and personality-modulated weights.

pub mod scoring;

use crate::config::RetrievalConfig;
use crate::error::MemzError;
use crate::memory::MemoryEntry;
use crate::types::{Embedding, GameTimestamp};

/// A scored retrieval result.
#[derive(Debug, Clone)]
pub struct RetrievalResult {
    /// The retrieved memory.
    pub memory: MemoryEntry,
    /// Combined retrieval score (0.0 to 1.0+).
    pub score: f64,
    /// Per-factor breakdown (useful for debugging / Memory Inspector).
    pub breakdown: ScoreBreakdown,
}

/// Breakdown of a retrieval score into its component factors.
#[derive(Debug, Clone, Default)]
pub struct ScoreBreakdown {
    /// Recency factor contribution.
    pub recency: f64,
    /// Relevance factor contribution.
    pub relevance: f64,
    /// Importance factor contribution.
    pub importance: f64,
    /// Emotional intensity contribution.
    pub emotional: f64,
    /// Social trust contribution.
    pub social: f64,
}

/// The retrieval engine that finds relevant memories for a given context.
pub struct RetrievalEngine {
    config: RetrievalConfig,
}

impl RetrievalEngine {
    /// Create a new retrieval engine with the given configuration.
    #[must_use]
    pub fn new(config: RetrievalConfig) -> Self {
        Self { config }
    }

    /// Retrieve the top-K most relevant memories given a context embedding.
    ///
    /// Performance target: < 0.5ms P50 for 200 memories.
    pub fn retrieve(
        &self,
        context_embedding: &Embedding,
        memories: &[MemoryEntry],
        current_time: &GameTimestamp,
        _personality_weights: Option<&PersonalityWeightOverrides>,
    ) -> Result<Vec<RetrievalResult>, MemzError> {
        let weights = &self.config.weights;
        let top_k = self.config.top_k;

        let mut results: Vec<RetrievalResult> = memories
            .iter()
            .map(|memory| {
                let breakdown = scoring::compute_breakdown(
                    memory,
                    context_embedding,
                    current_time,
                    weights.recency,
                    weights.relevance,
                    weights.importance,
                    weights.emotional,
                    weights.social,
                );

                let score = breakdown.recency
                    + breakdown.relevance
                    + breakdown.importance
                    + breakdown.emotional
                    + breakdown.social;

                RetrievalResult {
                    memory: memory.clone(),
                    score,
                    breakdown,
                }
            })
            .collect();

        // Sort descending by score.
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results.truncate(top_k);
        Ok(results)
    }
}

/// Optional per-personality weight overrides (e.g., a sentimental NPC
/// weighs emotional factor higher; a logical NPC weighs relevance higher).
#[derive(Debug, Clone)]
pub struct PersonalityWeightOverrides {
    /// Multiplier for recency weight.
    pub recency_mult: f32,
    /// Multiplier for relevance weight.
    pub relevance_mult: f32,
    /// Multiplier for importance weight.
    pub importance_mult: f32,
    /// Multiplier for emotional weight.
    pub emotional_mult: f32,
    /// Multiplier for social weight.
    pub social_mult: f32,
}

impl Default for PersonalityWeightOverrides {
    fn default() -> Self {
        Self {
            recency_mult: 1.0,
            relevance_mult: 1.0,
            importance_mult: 1.0,
            emotional_mult: 1.0,
            social_mult: 1.0,
        }
    }
}
