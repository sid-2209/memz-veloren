//! Per-factor scoring functions for the memory retrieval algorithm (§12.4).
//!
//! Score = w₁·Recency(m) + w₂·Relevance(m) + w₃·Importance(m) + w₄·Emotional(m) + w₅·Social(m)
//!
//! Where:
//!   Recency(m)    = exp(-λ · ΔT)        (Ebbinghaus forgetting curve)
//!   Relevance(m)  = cosine_similarity(context_embedding, memory_embedding)
//!   Importance(m)  = pre-computed importance score (0–1)
//!   Emotional(m)  = |emotional_valence| × emotional_volatility
//!   Social(m)     = trust_in_source × recency_of_social_transmission

use crate::memory::MemoryEntry;
use crate::retrieval::ScoreBreakdown;
use crate::types::{Embedding, GameTimestamp};

/// Default Ebbinghaus decay constant λ (per game-day).
const DEFAULT_DECAY_LAMBDA: f64 = 0.05;

/// Compute the full score breakdown for a single memory.
pub fn compute_breakdown(
    memory: &MemoryEntry,
    context_embedding: &Embedding,
    current_time: &GameTimestamp,
    w_recency: f32,
    w_relevance: f32,
    w_importance: f32,
    w_emotional: f32,
    w_social: f32,
) -> ScoreBreakdown {
    let recency = f64::from(w_recency) * recency_score(memory, current_time);
    let relevance = f64::from(w_relevance) * relevance_score(memory, context_embedding);
    let importance = f64::from(w_importance) * importance_score(memory);
    let emotional = f64::from(w_emotional) * emotional_score(memory);
    let social = f64::from(w_social) * social_score(memory);

    ScoreBreakdown {
        recency,
        relevance,
        importance,
        emotional,
        social,
    }
}

/// Recency score: Ebbinghaus forgetting curve R = e^(-λ · ΔT).
///
/// ΔT is measured in game-days (computed from tick difference).
fn recency_score(memory: &MemoryEntry, current_time: &GameTimestamp) -> f64 {
    let memory_timestamp = match memory {
        MemoryEntry::Episodic(m) => &m.timestamp,
        MemoryEntry::Social(m) => &m.received_at,
        MemoryEntry::Reflective(m) => &m.generated_at,
        MemoryEntry::Procedural(m) => &m.last_practiced,
        MemoryEntry::Injected(m) => &m.memory_timestamp,
        // Semantic and emotional memories don't have a single timestamp;
        // use a high recency score by default (they don't decay the same way).
        MemoryEntry::Semantic(_) | MemoryEntry::Emotional(_) => {
            return 0.8; // stable knowledge is always somewhat recent
        }
    };

    let delta_ticks = current_time.tick.saturating_sub(memory_timestamp.tick);
    // Approximate: 1 game-day ≈ 72000 ticks (assuming 20 ticks/sec, 1 hour of real time ≈ 1 day)
    // This is configurable but we use a sensible default.
    let delta_days = delta_ticks as f64 / 72_000.0;

    (-DEFAULT_DECAY_LAMBDA * delta_days).exp()
}

/// Relevance score: cosine similarity between context embedding and memory embedding.
fn relevance_score(memory: &MemoryEntry, context_embedding: &Embedding) -> f64 {
    let memory_embedding = match memory {
        MemoryEntry::Episodic(m) => m.embedding.as_ref(),
        MemoryEntry::Semantic(m) => m.embedding.as_ref(),
        MemoryEntry::Injected(m) => m.embedding.as_ref(),
        // These types don't have embeddings — return a neutral score.
        MemoryEntry::Social(_)
        | MemoryEntry::Emotional(_)
        | MemoryEntry::Reflective(_)
        | MemoryEntry::Procedural(_) => None,
    };

    match memory_embedding {
        Some(emb) => {
            let sim = context_embedding.cosine_similarity(emb);
            // Clamp to [0, 1] — negative similarity is treated as 0.
            f64::from(sim.max(0.0))
        }
        None => 0.5, // neutral relevance for memories without embeddings
    }
}

/// Importance score: pre-computed importance (0–1).
fn importance_score(memory: &MemoryEntry) -> f64 {
    let raw = match memory {
        MemoryEntry::Episodic(m) => m.importance,
        MemoryEntry::Semantic(m) => m.confidence,
        MemoryEntry::Emotional(m) => m.intensity,
        MemoryEntry::Social(_) => 0.5,
        MemoryEntry::Reflective(m) => m.confidence,
        MemoryEntry::Procedural(m) => m.proficiency,
        MemoryEntry::Injected(m) => m.importance(),
    };
    f64::from(raw.clamp(0.0, 1.0))
}

/// Emotional score: |emotional_valence| × volatility factor.
///
/// Strong emotions are more memorable (flashbulb memory effect — Brown & Kulik, 1977).
fn emotional_score(memory: &MemoryEntry) -> f64 {
    match memory {
        MemoryEntry::Episodic(m) => f64::from(m.emotional_valence.abs()),
        MemoryEntry::Emotional(m) => f64::from(m.intensity),
        MemoryEntry::Injected(m) => f64::from(m.emotional_weight),
        // Non-emotional memory types contribute a small baseline.
        _ => 0.1,
    }
}

/// Social score: trust_in_source × chain depth discount.
///
/// First-hand > second-hand > rumor (with Dunbar-informed decay).
fn social_score(memory: &MemoryEntry) -> f64 {
    match memory {
        MemoryEntry::Social(m) => f64::from(m.trust_in_source * m.chain_reliability()),
        // Non-social memories score 0 on the social axis (which is correct —
        // the social weight is only 0.10 by default).
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::GameTimestamp;

    #[test]
    fn recency_decays_over_time() {
        let t0 = GameTimestamp::now(0);
        let t1 = GameTimestamp::now(72_000);
        let t10 = GameTimestamp::now(720_000);

        let episodic = MemoryEntry::Episodic(crate::memory::episodic::EpisodicMemory::new(
            "test event",
            vec![],
            crate::types::Location::default(),
            t0,
            0.5,
            0.5,
        ));

        let score_at_1 = recency_score(&episodic, &t1);
        let score_at_10 = recency_score(&episodic, &t10);

        assert!(score_at_1 > score_at_10, "Recency should decay over time");
        assert!(score_at_1 > 0.9, "1 day should still be quite recent");
        assert!(score_at_10 < 0.7, "10 days should show significant decay");
    }

    #[test]
    fn relevance_uses_cosine_similarity() {
        let emb_a = Embedding(vec![1.0, 0.0, 0.0]);
        let emb_b = Embedding(vec![1.0, 0.0, 0.0]);
        let emb_c = Embedding(vec![0.0, 1.0, 0.0]);

        let sim_same = emb_a.cosine_similarity(&emb_b);
        let sim_ortho = emb_a.cosine_similarity(&emb_c);

        assert!((sim_same - 1.0).abs() < 0.001);
        assert!(sim_ortho.abs() < 0.001);
    }
}
