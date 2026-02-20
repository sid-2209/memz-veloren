//! NPC Behavior Modification — memories shaping actions (§10, §12.5)
//!
//! This module computes how an NPC's accumulated memories modify their
//! behavior in specific situations:
//!
//! - **Greeting changes**: NPCs who remember you greet you differently
//! - **Price adjustments**: Trade prices influenced by past dealings
//! - **Quest modifications**: NPCs offer or refuse quests based on history
//! - **Combat disposition**: Past experience changes fight-or-flee decisions
//! - **Gossip selection**: What an NPC chooses to share depends on memory
//!
//! Performance target: < 0.2ms per behavior check (§12.6)

use crate::memory::MemoryBank;
use crate::memory::episodic::EpisodicMemory;
use crate::memory::emotional::EmotionalMemory;
use crate::memory::social::SocialMemory;
use crate::types::EntityId;

/// Overall disposition of an NPC toward a specific entity.
#[derive(Debug, Clone)]
pub struct Disposition {
    /// Net sentiment toward the entity (-1.0 hostile to +1.0 friendly).
    pub sentiment: f32,
    /// Confidence in this sentiment (0.0 no data, 1.0 very certain).
    pub confidence: f32,
    /// Number of interactions this is based on.
    pub interaction_count: u32,
    /// How this sentiment was derived.
    pub basis: DispositionBasis,
}

/// How a disposition was computed.
#[derive(Debug, Clone)]
pub enum DispositionBasis {
    /// Based on direct personal experience.
    DirectExperience {
        /// Number of positive interactions.
        positive_count: u32,
        /// Number of negative interactions.
        negative_count: u32,
    },
    /// Based on social gossip / hearsay.
    Hearsay {
        /// Number of sources.
        source_count: u32,
        /// Average trust in sources.
        avg_trust: f32,
    },
    /// Mixed direct experience + hearsay.
    Mixed {
        /// Weight of direct experience (0.0–1.0).
        direct_weight: f32,
    },
    /// No data — default neutral.
    Unknown,
}

/// Compute an NPC's overall disposition toward a target entity.
///
/// Combines episodic, emotional, and social memories with
/// personality-weighted scoring.
///
/// # Performance
/// Target: < 0.1ms
pub fn compute_disposition(
    bank: &MemoryBank,
    target: EntityId,
) -> Disposition {
    let direct = compute_direct_sentiment(bank, target);
    let social = compute_social_sentiment(bank, target);

    match (direct, social) {
        (Some(d), Some(s)) => {
            // Direct experience weighted 3× hearsay
            let weight = 0.75;
            let combined = d.sentiment * weight + s.sentiment * (1.0 - weight);
            let confidence = (d.confidence * weight + s.confidence * (1.0 - weight))
                .min(1.0);
            Disposition {
                sentiment: combined.clamp(-1.0, 1.0),
                confidence,
                interaction_count: d.interaction_count + s.interaction_count,
                basis: DispositionBasis::Mixed {
                    direct_weight: weight,
                },
            }
        }
        (Some(d), None) => d,
        (None, Some(s)) => s,
        (None, None) => Disposition {
            sentiment: 0.0,
            confidence: 0.0,
            interaction_count: 0,
            basis: DispositionBasis::Unknown,
        },
    }
}

/// Compute direct sentiment from episodic and emotional memories.
fn compute_direct_sentiment(
    bank: &MemoryBank,
    target: EntityId,
) -> Option<Disposition> {
    // Find episodic memories involving this target
    let relevant_episodic: Vec<&EpisodicMemory> = bank
        .episodic
        .iter()
        .filter(|m| m.participants.contains(&target))
        .collect();

    // Find emotional memories about this target
    let relevant_emotional: Vec<&EmotionalMemory> = bank
        .emotional
        .iter()
        .filter(|m| m.target == target)
        .collect();

    if relevant_episodic.is_empty() && relevant_emotional.is_empty() {
        return None;
    }

    // Weighted average of episodic valences (more recent → higher weight)
    let episodic_sentiment = if relevant_episodic.is_empty() {
        0.0
    } else {
        let total_weight: f32 = relevant_episodic
            .iter()
            .map(|m| m.strength * m.importance)
            .sum();
        if total_weight == 0.0 {
            0.0
        } else {
            relevant_episodic
                .iter()
                .map(|m| m.emotional_valence * m.strength * m.importance)
                .sum::<f32>()
                / total_weight
        }
    };

    // Emotional memory sentiment (latest PAD state)
    let emotional_sentiment = relevant_emotional
        .last()
        .map(|m| m.intensity * if m.pad_state.pleasure > 0.0 { 1.0 } else { -1.0 })
        .unwrap_or(0.0);

    let positive = relevant_episodic
        .iter()
        .filter(|m| m.emotional_valence > 0.1)
        .count() as u32;
    let negative = relevant_episodic
        .iter()
        .filter(|m| m.emotional_valence < -0.1)
        .count() as u32;

    let sentiment = (episodic_sentiment * 0.6 + emotional_sentiment * 0.4).clamp(-1.0, 1.0);
    let count = (relevant_episodic.len() + relevant_emotional.len()) as u32;
    let confidence = (count as f32 / 10.0).min(1.0);

    Some(Disposition {
        sentiment,
        confidence,
        interaction_count: count,
        basis: DispositionBasis::DirectExperience {
            positive_count: positive,
            negative_count: negative,
        },
    })
}

/// Compute social sentiment from gossip / hearsay.
fn compute_social_sentiment(
    bank: &MemoryBank,
    target: EntityId,
) -> Option<Disposition> {
    let relevant: Vec<&SocialMemory> = bank
        .social
        .iter()
        .filter(|m| m.about == target && m.believed)
        .collect();

    if relevant.is_empty() {
        return None;
    }

    let avg_trust: f32 = relevant.iter().map(|m| m.trust_in_source).sum::<f32>()
        / relevant.len() as f32;

    // Sentiment from social memories based on their content sentiment
    let social_sentiment: f32 = relevant
        .iter()
        .map(|m| m.sentiment * m.trust_in_source)
        .sum::<f32>()
        / relevant.iter().map(|m| m.trust_in_source).sum::<f32>().max(0.01);

    Some(Disposition {
        sentiment: social_sentiment.clamp(-1.0, 1.0),
        confidence: (avg_trust * relevant.len() as f32 / 5.0).min(1.0),
        interaction_count: relevant.len() as u32,
        basis: DispositionBasis::Hearsay {
            source_count: relevant.len() as u32,
            avg_trust,
        },
    })
}

/// Greeting style based on disposition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GreetingStyle {
    /// Warm, friendly greeting (references past positive interactions).
    Warm,
    /// Neutral, standard greeting.
    Neutral,
    /// Cool, distant greeting.
    Cool,
    /// Hostile or suspicious greeting.
    Hostile,
    /// Excited greeting (first meeting after long absence or notable event).
    Excited,
    /// No greeting (NPC refuses to acknowledge).
    Silent,
}

/// Determine how an NPC should greet a target entity.
#[must_use]
pub fn compute_greeting_style(disposition: &Disposition) -> GreetingStyle {
    if disposition.confidence < 0.1 {
        return GreetingStyle::Neutral;
    }

    match disposition.sentiment {
        s if s > 0.6 => GreetingStyle::Warm,
        s if s > 0.2 => GreetingStyle::Neutral,
        s if s > -0.2 => GreetingStyle::Neutral,
        s if s > -0.5 => GreetingStyle::Cool,
        s if s > -0.8 => GreetingStyle::Hostile,
        _ => GreetingStyle::Silent,
    }
}

/// Trade price modifier based on disposition.
///
/// Returns a multiplier: 1.0 = normal price, < 1.0 = discount, > 1.0 = markup.
///
/// From §10: "A merchant who witnessed you defending their shop might give
/// 15% discounts. One who knows you stole from a friend charges 25% extra."
#[must_use]
pub fn compute_price_modifier(disposition: &Disposition) -> f32 {
    if disposition.confidence < 0.05 {
        return 1.0; // Not enough data
    }

    // Linear mapping: sentiment -1.0 → 1.25 (25% markup), +1.0 → 0.85 (15% discount)
    let modifier = 1.0 - disposition.sentiment * 0.15;
    modifier.clamp(0.75, 1.35) // Hard limits to prevent extreme prices
}

/// Combat disposition — fight, flee, or negotiate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CombatDisposition {
    /// Eager to fight (negative sentiment + brave personality).
    Aggressive,
    /// Will fight if provoked.
    Defensive,
    /// Prefers to talk or negotiate.
    Diplomatic,
    /// Wants to flee (very negative experience or cowardly).
    Flee,
    /// Neutral — follows default AI.
    Default,
}

/// Determine combat disposition based on memories and personality.
#[must_use]
pub fn compute_combat_disposition(
    disposition: &Disposition,
    bravery: f32,
) -> CombatDisposition {
    if disposition.confidence < 0.1 {
        return CombatDisposition::Default;
    }

    let aggression = -disposition.sentiment * bravery;

    if aggression > 0.6 {
        CombatDisposition::Aggressive
    } else if aggression > 0.3 {
        CombatDisposition::Defensive
    } else if disposition.sentiment > 0.3 {
        CombatDisposition::Diplomatic
    } else if bravery < 0.3 && disposition.sentiment < -0.3 {
        CombatDisposition::Flee
    } else {
        CombatDisposition::Default
    }
}

/// Memory-based quest eligibility check.
///
/// Returns (eligible, reason). NPCs may refuse quests based on past experience.
#[must_use]
pub fn check_quest_eligibility(
    bank: &MemoryBank,
    player: EntityId,
) -> (bool, String) {
    let disposition = compute_disposition(bank, player);

    if disposition.sentiment < -0.5 && disposition.confidence > 0.3 {
        return (
            false,
            "I don't trust you enough for this task.".to_string(),
        );
    }

    if disposition.sentiment > 0.5 && disposition.confidence > 0.3 {
        return (
            true,
            "You've proven yourself reliable. I have something for you.".to_string(),
        );
    }

    (true, "I have a task that needs doing.".to_string())
}

/// Select gossip to share based on memories and social context.
///
/// Returns up to `max_count` social memories suitable for sharing,
/// sorted by gossip priority.
pub fn select_gossip(
    bank: &MemoryBank,
    listener: EntityId,
    max_count: usize,
) -> Vec<SocialMemory> {
    let mut candidates: Vec<SocialMemory> = bank
        .social
        .iter()
        .filter(|m| {
            // Don't gossip about someone to their face
            m.about != listener
                // Only share things we believe
                && m.believed
                // Don't share deeply propagated rumors
                && m.propagation_depth < 3
        })
        .cloned()
        .collect();

    // Sort by importance heuristic: trust × recency
    candidates.sort_by(|a, b| {
        let score_a = a.trust_in_source * (1.0 / (1.0 + a.propagation_depth as f32));
        let score_b = b.trust_in_source * (1.0 / (1.0 + b.propagation_depth as f32));
        score_b
            .partial_cmp(&score_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    candidates.truncate(max_count);
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::episodic::EpisodicMemory;
    use crate::memory::emotional::EmotionalMemory;
    use crate::memory::social::SocialMemory;
    use crate::types::{EntityId, GameTimestamp, Location, PADState};

    fn make_bank_with_positive_history(target: EntityId) -> MemoryBank {
        let mut bank = MemoryBank::new();
        let ts = GameTimestamp::now(36_000);

        for _ in 0..5 {
            bank.episodic.push(EpisodicMemory::new(
                "Helped with task",
                vec![target],
                Location::default(),
                ts,
                0.7,
                0.5,
            ));
        }

        bank.emotional.push(EmotionalMemory::new(
            target,
            "gratitude".to_string(),
            0.8,
            PADState { pleasure: 0.7, arousal: 0.3, dominance: 0.5 },
            vec![],
            ts,
        ));

        bank
    }

    fn make_bank_with_negative_history(target: EntityId) -> MemoryBank {
        let mut bank = MemoryBank::new();
        let ts = GameTimestamp::now(36_000);

        for _ in 0..5 {
            bank.episodic.push(EpisodicMemory::new(
                "Stole from shop",
                vec![target],
                Location::default(),
                ts,
                -0.8,
                0.7,
            ));
        }

        bank.emotional.push(EmotionalMemory::new(
            target,
            "anger".to_string(),
            0.9,
            PADState { pleasure: -0.8, arousal: 0.6, dominance: 0.4 },
            vec![],
            ts,
        ));

        bank
    }

    #[test]
    fn positive_history_yields_warm_greeting() {
        let target = EntityId::new();
        let bank = make_bank_with_positive_history(target);
        let disp = compute_disposition(&bank, target);

        assert!(disp.sentiment > 0.0);
        assert!(disp.confidence > 0.0);
        let greeting = compute_greeting_style(&disp);
        assert!(matches!(greeting, GreetingStyle::Warm | GreetingStyle::Neutral));
    }

    #[test]
    fn negative_history_yields_hostile_greeting() {
        let target = EntityId::new();
        let bank = make_bank_with_negative_history(target);
        let disp = compute_disposition(&bank, target);

        assert!(disp.sentiment < 0.0);
        let greeting = compute_greeting_style(&disp);
        assert!(matches!(
            greeting,
            GreetingStyle::Cool | GreetingStyle::Hostile | GreetingStyle::Silent
        ));
    }

    #[test]
    fn positive_disposition_gives_discount() {
        let target = EntityId::new();
        let bank = make_bank_with_positive_history(target);
        let disp = compute_disposition(&bank, target);
        let modifier = compute_price_modifier(&disp);
        assert!(modifier < 1.0, "Expected discount, got {}", modifier);
    }

    #[test]
    fn negative_disposition_gives_markup() {
        let target = EntityId::new();
        let bank = make_bank_with_negative_history(target);
        let disp = compute_disposition(&bank, target);
        let modifier = compute_price_modifier(&disp);
        assert!(modifier > 1.0, "Expected markup, got {}", modifier);
    }

    #[test]
    fn unknown_entity_gets_neutral() {
        let unknown = EntityId::new();
        let bank = MemoryBank::new();
        let disp = compute_disposition(&bank, unknown);

        assert_eq!(disp.sentiment, 0.0);
        assert_eq!(disp.confidence, 0.0);
        assert!(matches!(disp.basis, DispositionBasis::Unknown));
        assert_eq!(compute_greeting_style(&disp), GreetingStyle::Neutral);
        assert!((compute_price_modifier(&disp) - 1.0).abs() < 0.01);
    }

    #[test]
    fn combat_disposition_brave_hostile() {
        let target = EntityId::new();
        let bank = make_bank_with_negative_history(target);
        let disp = compute_disposition(&bank, target);
        let combat = compute_combat_disposition(&disp, 0.9); // very brave
        assert!(matches!(
            combat,
            CombatDisposition::Aggressive | CombatDisposition::Defensive
        ));
    }

    #[test]
    fn quest_eligibility_trusted_player() {
        let player = EntityId::new();
        let bank = make_bank_with_positive_history(player);
        let (eligible, reason) = check_quest_eligibility(&bank, player);
        assert!(eligible);
        assert!(reason.contains("proven"));
    }

    #[test]
    fn quest_eligibility_distrusted_player() {
        let player = EntityId::new();
        let bank = make_bank_with_negative_history(player);
        let (eligible, _reason) = check_quest_eligibility(&bank, player);
        assert!(!eligible);
    }

    #[test]
    fn gossip_selection_filters() {
        let mut bank = MemoryBank::new();
        let listener = EntityId::new();
        let about = EntityId::new();
        let ts = GameTimestamp::now(36_000);

        // Regular gossip — should be included
        bank.social.push(SocialMemory::new(
            about,
            EntityId::new(),
            "They helped fight bandits".to_string(),
            0.8,
            0,
            ts,
        ));

        // Gossip about the listener — should be excluded
        bank.social.push(SocialMemory::new(
            listener,
            EntityId::new(),
            "About the listener".to_string(),
            0.8,
            0,
            ts,
        ));

        let gossip = select_gossip(&bank, listener, 5);
        assert_eq!(gossip.len(), 1);
        assert_eq!(gossip[0].about, about);
    }
}
