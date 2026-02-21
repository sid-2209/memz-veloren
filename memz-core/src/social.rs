//! Social Memory Propagation — Gossip, Trust Networks, Belief Updates (§9, §12.8)
//!
//! Models the spread of information through NPC social networks,
//! grounded in Dunbar's social brain hypothesis and Bayesian belief updating.
//!
//! Key features:
//!   - Trust-weighted information propagation
//!   - Gossip chain degradation (telephone game effect)
//!   - Personality-modulated credulity
//!   - Conflict resolution via Bayesian-inspired belief update (§9.3)
//!
//! References:
//!   - Dunbar, R. (1996). "Grooming, Gossip, and the Evolution of Language."
//!   - Tenenbaum et al. (2011). "How to Grow a Mind."

use crate::memory::social::SocialMemory;
use crate::types::{EntityId, GameTimestamp, PersonalityTraits};

/// The result of attempting to propagate a social memory to an NPC.
#[derive(Debug, Clone)]
pub enum PropagationResult {
    /// NPC accepted the information.
    Accepted {
        /// The new social memory created for the receiving NPC.
        new_memory: SocialMemory,
        /// How strongly the NPC believes the claim (0.0–1.0).
        belief_strength: f32,
    },
    /// NPC rejected the information (didn't believe it).
    Rejected {
        /// Why the NPC rejected the claim.
        reason: String,
    },
    /// NPC is uncertain — may seek corroborating evidence.
    Uncertain {
        /// Whether the NPC will actively investigate the claim.
        will_investigate: bool,
    },
}

/// Belief update thresholds (configurable via `MemzConfig`).
const BELIEF_THRESHOLD: f32 = 0.5;
const HYSTERESIS: f32 = 0.05;

/// Attempt to propagate a social memory from one NPC to another.
///
/// This implements the Bayesian-inspired belief update model from §9.3.
///
/// # Arguments
/// * `claim` — The social memory being shared.
/// * `receiver_personality` — The receiving NPC's personality traits.
/// * `trust_in_source` — How much the receiver trusts the source (0.0–1.0).
/// * `has_direct_experience` — Whether the receiver has personally witnessed something about the subject.
/// * `direct_sentiment` — Receiver's direct sentiment toward the subject (-1.0 to 1.0), if any.
/// * `existing_belief_consistency` — How consistent this claim is with the receiver's existing beliefs (0.0–1.0).
/// * `receiver_emotional_state_toward_subject` — Receiver's current emotional state toward the claim's subject (-1.0 to 1.0).
/// * `source_reliability` — Track record of the source's past claims (0.0–1.0).
/// * `current_time` — Current game timestamp.
#[must_use] 
pub fn propagate_memory(
    claim: &SocialMemory,
    _receiver_id: EntityId,
    receiver_personality: &PersonalityTraits,
    trust_in_source: f32,
    has_direct_experience: bool,
    direct_sentiment: Option<f32>,
    existing_belief_consistency: f32,
    receiver_emotional_state_toward_subject: f32,
    source_reliability: f32,
    current_time: GameTimestamp,
) -> PropagationResult {
    // --- 1. Prior: Direct experience vs hearsay ---
    let (prior_weight, hearsay_weight) = if has_direct_experience {
        (0.8_f32, 0.2_f32)
    } else {
        (0.0_f32, 1.0_f32)
    };

    let direct_evidence = direct_sentiment.unwrap_or(0.0).clamp(0.0, 1.0);

    // --- 2. Source credibility (Dunbar-informed trust network) ---
    let credibility = trust_in_source * 0.6 + source_reliability * 0.4;

    // --- 3. Consistency with existing belief network ---
    let consistency = existing_belief_consistency;

    // --- 4. Personality modulation (Big Five-inspired traits) ---
    let personality_bias = receiver_personality.credulity;
    let openness = receiver_personality.openness;

    // --- 5. Emotional state modifier (confirmation bias) ---
    let emotional_bias = if receiver_emotional_state_toward_subject > 0.5 {
        -0.1 // Positive feelings → resistant to negative info
    } else if receiver_emotional_state_toward_subject < -0.5 {
        0.1 // Negative feelings → receptive to negative info
    } else {
        0.0
    };

    // --- 6. Information freshness (temporal discounting) ---
    let days_since_claim = {
        let delta_ticks = current_time.tick.saturating_sub(claim.received_at.tick);
        delta_ticks as f32 / 72_000.0
    };
    let freshness = (-0.1 * days_since_claim).exp();

    // --- 7. Information chain depth (first-hand > second-hand > rumor) ---
    let chain_discount = 1.0 / (1.0 + claim.propagation_depth as f32);

    // --- 8. Final belief score (Bayesian posterior approximation) ---
    let belief = (prior_weight * direct_evidence
        + hearsay_weight * credibility * consistency * freshness * chain_discount
        + personality_bias * openness * 0.15
        + emotional_bias)
        .clamp(0.0, 1.0);

    // --- 9. Threshold with hysteresis (prevent belief flip-flopping) ---
    if belief > BELIEF_THRESHOLD + HYSTERESIS {
        // Create new social memory for the receiver with incremented propagation depth.
        let new_memory = SocialMemory::new(
            claim.about,
            claim.source, // original source, not the gossiper
            claim.claim.clone(),
            trust_in_source,
            claim.propagation_depth + 1,
            current_time,
        );
        PropagationResult::Accepted {
            new_memory,
            belief_strength: belief,
        }
    } else if belief < BELIEF_THRESHOLD - HYSTERESIS {
        PropagationResult::Rejected {
            reason: format!(
                "Belief score too low: {belief:.2} (threshold: {BELIEF_THRESHOLD})"
            ),
        }
    } else {
        PropagationResult::Uncertain {
            will_investigate: openness > 0.5,
        }
    }
}

/// Compute the gossip tendency of an NPC — how likely they are to share
/// a particular social memory during an interaction.
///
/// Returns a probability (0.0–1.0).
#[must_use]
pub fn gossip_probability(
    personality: &PersonalityTraits,
    memory_importance: f32,
    memory_emotional_weight: f32,
) -> f32 {
    let base = personality.gossip_tendency;
    let boost = memory_importance * 0.3 + memory_emotional_weight * 0.2;
    (base + boost).clamp(0.0, 1.0)
}

/// Maximum gossip chain depth before information is considered "worthless rumor".
///
/// From §16 (memz.toml): `max_gossip_chain_depth = 4`.
pub const MAX_CHAIN_DEPTH: u32 = 4;

/// Should this social memory still be propagated, or has it degraded too far?
#[must_use]
pub fn is_propagatable(memory: &SocialMemory) -> bool {
    memory.propagation_depth < MAX_CHAIN_DEPTH && memory.trust_in_source > 0.1
}

/// Compute trust decay over time (trust erodes slowly without reinforcement).
///
/// `trust_new` = trust × e^(-decay_rate × days)
#[must_use]
pub fn decay_trust(current_trust: f32, days_without_interaction: f32, decay_rate: f32) -> f32 {
    (current_trust * (-decay_rate * days_without_interaction).exp()).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{EntityId, GameTimestamp, PersonalityTraits};

    fn make_claim() -> SocialMemory {
        SocialMemory::new(
            EntityId::new(),   // about
            EntityId::new(),   // source
            "Player helped fight off bandits".to_string(),
            0.8, // trust in source
            0, // first-hand
            GameTimestamp::now(36_000),
        )
    }

    fn default_personality() -> PersonalityTraits {
        PersonalityTraits::default()
    }

    #[test]
    fn credulous_npc_accepts_gossip() {
        let claim = make_claim();
        let mut personality = default_personality();
        personality.credulity = 0.9;
        personality.openness = 0.9;

        let result = propagate_memory(
            &claim,
            EntityId::new(),
            &personality,
            0.8,   // high trust
            false,  // no direct experience
            None,
            0.7,   // consistent
            0.0,   // neutral emotional state
            0.8,   // source is reliable
            GameTimestamp::now(40_000),
        );

        assert!(matches!(result, PropagationResult::Accepted { .. }));
    }

    #[test]
    fn skeptical_npc_rejects_gossip() {
        let claim = make_claim();
        let mut personality = default_personality();
        personality.credulity = 0.1;
        personality.openness = 0.1;

        let result = propagate_memory(
            &claim,
            EntityId::new(),
            &personality,
            0.2,   // low trust
            false,
            None,
            0.2,   // inconsistent
            0.0,
            0.2,   // source unreliable
            GameTimestamp::now(200_000), // stale info
        );

        assert!(matches!(
            result,
            PropagationResult::Rejected { .. } | PropagationResult::Uncertain { .. }
        ));
    }

    #[test]
    fn direct_experience_overrides_gossip() {
        let claim = make_claim();
        let personality = default_personality();

        // NPC has had a positive direct experience with the subject.
        let result = propagate_memory(
            &claim,
            EntityId::new(),
            &personality,
            0.5,
            true,          // has direct experience
            Some(0.9),     // strong positive sentiment
            0.8,
            0.5,
            0.5,
            GameTimestamp::now(40_000),
        );

        assert!(matches!(result, PropagationResult::Accepted { .. }));
    }

    #[test]
    fn gossip_chain_degrades() {
        let mut claim = make_claim();
        claim.propagation_depth = 3;
        assert!(is_propagatable(&claim));

        claim.propagation_depth = 4;
        assert!(!is_propagatable(&claim));
    }

    #[test]
    fn trust_decays_over_time() {
        let trust = decay_trust(1.0, 0.0, 0.01);
        assert!((trust - 1.0).abs() < 0.001);

        let trust_later = decay_trust(1.0, 100.0, 0.01);
        assert!(trust_later < 0.5);
    }
}
