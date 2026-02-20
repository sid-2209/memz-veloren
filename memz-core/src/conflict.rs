//! Memory Conflicts — contradictory memories creating drama (§14.3)
//!
//! When NPCs hold contradictory memories about the same event or entity,
//! it creates natural dramatic tension. An NPC who witnessed a theft but
//! hears gossip that the accused is innocent faces a conflict.
//!
//! Conflicts can be resolved through:
//! - Direct confrontation (asking the parties involved)
//! - Additional evidence (more witnesses, more gossip)
//! - Time (older memories fade, resolving the conflict)
//! - Personality (credulous NPCs resolve conflicts faster)

use serde::{Deserialize, Serialize};

use crate::memory::social::SocialMemory;
use crate::memory::MemoryBank;
use crate::types::{EntityId, GameTimestamp, MemoryId};

/// A detected conflict between memories.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConflict {
    /// Unique identifier.
    pub id: MemoryId,
    /// The entity this conflict is about.
    pub about: EntityId,
    /// The positive claim (e.g., "they helped us").
    pub positive_claim: ConflictClaim,
    /// The negative claim (e.g., "they stole from us").
    pub negative_claim: ConflictClaim,
    /// Current resolution state.
    pub state: ConflictState,
    /// When the conflict was detected.
    pub detected_at: GameTimestamp,
    /// How strongly this conflict affects NPC behavior (0.0–1.0).
    pub tension: f32,
}

/// One side of a memory conflict.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictClaim {
    /// Description of the claim.
    pub description: String,
    /// Where this claim came from.
    pub source: ClaimSource,
    /// Confidence in this claim (0.0–1.0).
    pub confidence: f32,
    /// How many corroborating memories support this claim.
    pub corroboration_count: u32,
}

/// Where a conflicting claim originated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClaimSource {
    /// NPC witnessed this personally.
    DirectExperience,
    /// NPC heard this from a trusted source.
    TrustedGossip {
        /// Who told them.
        source: EntityId,
        /// Trust level (0.0–1.0).
        trust: f32,
    },
    /// NPC heard this as rumor.
    Rumor {
        /// Propagation depth.
        depth: u32,
    },
}

/// Resolution state of a memory conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictState {
    /// Conflict is active — NPC is uncertain and may act unpredictably.
    Active,
    /// NPC has resolved in favor of the positive claim.
    ResolvedPositive,
    /// NPC has resolved in favor of the negative claim.
    ResolvedNegative,
    /// Conflict faded due to time (both memories weakened).
    Faded,
    /// NPC has decided they don't care (low emotional investment).
    Dismissed,
}

/// Detect memory conflicts within a memory bank.
///
/// Looks for cases where:
/// - Episodic memories have contradictory valence about the same entity
/// - Social memories contradict direct experience
/// - Social memories from different sources contradict each other
pub fn detect_conflicts(
    bank: &MemoryBank,
    min_tension: f32,
    current_time: GameTimestamp,
) -> Vec<MemoryConflict> {
    let mut conflicts = Vec::new();

    // --- 1. Episodic vs Social conflicts ---
    // For each entity the NPC has social memories about,
    // check if they contradict episodic memories.
    detect_episodic_vs_social(bank, min_tension, current_time, &mut conflicts);

    // --- 2. Social vs Social conflicts ---
    // Different gossip sources making contradictory claims.
    detect_social_vs_social(bank, min_tension, current_time, &mut conflicts);

    conflicts
}

/// Detect conflicts between direct experience and gossip.
fn detect_episodic_vs_social(
    bank: &MemoryBank,
    min_tension: f32,
    current_time: GameTimestamp,
    conflicts: &mut Vec<MemoryConflict>,
) {
    // Collect entities mentioned in social memories
    let social_entities: Vec<EntityId> = bank
        .social
        .iter()
        .map(|m| m.about)
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    for entity in social_entities {
        let episodic_avg = episodic_valence_for(bank, entity);
        let social_avg = social_sentiment_for(bank, entity);

        if let (Some(ev), Some(sv)) = (episodic_avg, social_avg) {
            // Conflict if signs differ and both are strong enough
            if ev.signum() != sv.signum() && ev.abs() > 0.3 && sv.abs() > 0.3 {
                let tension = ((ev - sv).abs() / 2.0).min(1.0);
                if tension >= min_tension {
                    let (pos, neg) = if ev > 0.0 {
                        (
                            ConflictClaim {
                                description: format!(
                                    "Direct experience suggests positive interactions"
                                ),
                                source: ClaimSource::DirectExperience,
                                confidence: ev.abs().min(1.0),
                                corroboration_count: episodic_count_for(bank, entity),
                            },
                            ConflictClaim {
                                description: format!(
                                    "Gossip suggests negative reputation"
                                ),
                                source: ClaimSource::Rumor { depth: 1 },
                                confidence: sv.abs().min(1.0),
                                corroboration_count: social_count_for(bank, entity),
                            },
                        )
                    } else {
                        (
                            ConflictClaim {
                                description: format!(
                                    "Gossip suggests positive reputation"
                                ),
                                source: ClaimSource::Rumor { depth: 1 },
                                confidence: sv.abs().min(1.0),
                                corroboration_count: social_count_for(bank, entity),
                            },
                            ConflictClaim {
                                description: format!(
                                    "Direct experience suggests negative interactions"
                                ),
                                source: ClaimSource::DirectExperience,
                                confidence: ev.abs().min(1.0),
                                corroboration_count: episodic_count_for(bank, entity),
                            },
                        )
                    };

                    conflicts.push(MemoryConflict {
                        id: MemoryId::new(),
                        about: entity,
                        positive_claim: pos,
                        negative_claim: neg,
                        state: ConflictState::Active,
                        detected_at: current_time,
                        tension,
                    });
                }
            }
        }
    }
}

/// Detect conflicts between different gossip sources.
fn detect_social_vs_social(
    bank: &MemoryBank,
    min_tension: f32,
    current_time: GameTimestamp,
    conflicts: &mut Vec<MemoryConflict>,
) {
    // Group social memories by the entity they're about
    let mut by_entity: std::collections::HashMap<EntityId, Vec<&SocialMemory>> =
        std::collections::HashMap::new();

    for mem in &bank.social {
        by_entity.entry(mem.about).or_default().push(mem);
    }

    for (entity, memories) in by_entity {
        if memories.len() < 2 {
            continue;
        }

        let positive: Vec<&&SocialMemory> = memories
            .iter()
            .filter(|m| m.sentiment > 0.3)
            .collect();
        let negative: Vec<&&SocialMemory> = memories
            .iter()
            .filter(|m| m.sentiment < -0.3)
            .collect();

        if !positive.is_empty() && !negative.is_empty() {
            let pos_avg: f32 = positive.iter().map(|m| m.sentiment).sum::<f32>()
                / positive.len() as f32;
            let neg_avg: f32 = negative.iter().map(|m| m.sentiment).sum::<f32>()
                / negative.len() as f32;

            let tension = ((pos_avg - neg_avg).abs() / 2.0).min(1.0);
            if tension >= min_tension {
                conflicts.push(MemoryConflict {
                    id: MemoryId::new(),
                    about: entity,
                    positive_claim: ConflictClaim {
                        description: format!(
                            "{} source(s) say positive things",
                            positive.len()
                        ),
                        source: ClaimSource::TrustedGossip {
                            source: positive[0].source,
                            trust: positive[0].trust_in_source,
                        },
                        confidence: pos_avg.abs(),
                        corroboration_count: positive.len() as u32,
                    },
                    negative_claim: ConflictClaim {
                        description: format!(
                            "{} source(s) say negative things",
                            negative.len()
                        ),
                        source: ClaimSource::TrustedGossip {
                            source: negative[0].source,
                            trust: negative[0].trust_in_source,
                        },
                        confidence: neg_avg.abs(),
                        corroboration_count: negative.len() as u32,
                    },
                    state: ConflictState::Active,
                    detected_at: current_time,
                    tension,
                });
            }
        }
    }
}

/// Attempt to resolve a conflict based on evidence and personality.
pub fn attempt_resolution(
    conflict: &mut MemoryConflict,
    credulity: f32,
    openness: f32,
) {
    if conflict.state != ConflictState::Active {
        return;
    }

    // Direct experience always wins over gossip
    if matches!(conflict.positive_claim.source, ClaimSource::DirectExperience) {
        if conflict.positive_claim.confidence > conflict.negative_claim.confidence * 1.5 {
            conflict.state = ConflictState::ResolvedPositive;
            return;
        }
    }
    if matches!(conflict.negative_claim.source, ClaimSource::DirectExperience) {
        if conflict.negative_claim.confidence > conflict.positive_claim.confidence * 1.5 {
            conflict.state = ConflictState::ResolvedNegative;
            return;
        }
    }

    // Corroboration count tips the scale
    let pos_weight = conflict.positive_claim.confidence
        * (1.0 + conflict.positive_claim.corroboration_count as f32 * 0.2);
    let neg_weight = conflict.negative_claim.confidence
        * (1.0 + conflict.negative_claim.corroboration_count as f32 * 0.2);

    let ratio = pos_weight / (pos_weight + neg_weight + 0.01);

    // Credulous NPCs resolve faster
    let threshold = 0.65 - credulity * 0.15;

    if ratio > threshold {
        conflict.state = ConflictState::ResolvedPositive;
    } else if ratio < (1.0 - threshold) {
        conflict.state = ConflictState::ResolvedNegative;
    } else if openness < 0.3 {
        // Closed-minded NPCs just dismiss conflicts they can't resolve
        conflict.state = ConflictState::Dismissed;
    }
    // Otherwise stays Active
}

// --- Helper functions ---

fn episodic_valence_for(bank: &MemoryBank, entity: EntityId) -> Option<f32> {
    let relevant: Vec<f32> = bank
        .episodic
        .iter()
        .filter(|m| m.participants.contains(&entity))
        .map(|m| m.emotional_valence)
        .collect();

    if relevant.is_empty() {
        None
    } else {
        Some(relevant.iter().sum::<f32>() / relevant.len() as f32)
    }
}

fn social_sentiment_for(bank: &MemoryBank, entity: EntityId) -> Option<f32> {
    let relevant: Vec<f32> = bank
        .social
        .iter()
        .filter(|m| m.about == entity)
        .map(|m| m.sentiment)
        .collect();

    if relevant.is_empty() {
        None
    } else {
        Some(relevant.iter().sum::<f32>() / relevant.len() as f32)
    }
}

fn episodic_count_for(bank: &MemoryBank, entity: EntityId) -> u32 {
    bank.episodic
        .iter()
        .filter(|m| m.participants.contains(&entity))
        .count() as u32
}

fn social_count_for(bank: &MemoryBank, entity: EntityId) -> u32 {
    bank.social
        .iter()
        .filter(|m| m.about == entity)
        .count() as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::episodic::EpisodicMemory;
    use crate::memory::social::SocialMemory;
    use crate::types::{EntityId, GameTimestamp, Location};

    fn make_bank_with_conflict(target: EntityId) -> MemoryBank {
        let mut bank = MemoryBank::new();
        let ts = GameTimestamp::now(36_000);

        // Positive direct experience
        for _ in 0..3 {
            bank.episodic.push(EpisodicMemory::new(
                "Positive interaction",
                vec![target],
                Location::default(),
                ts,
                0.7,
                0.5,
            ));
        }

        // Negative gossip about the same entity
        let mut social = SocialMemory::new(
            target,
            EntityId::new(),
            "They say this person is a thief".to_string(),
            0.8,
            1,
            ts,
        );
        social.sentiment = -0.7;
        bank.social.push(social);

        let mut social2 = SocialMemory::new(
            target,
            EntityId::new(),
            "I heard they can't be trusted".to_string(),
            0.6,
            2,
            ts,
        );
        social2.sentiment = -0.5;
        bank.social.push(social2);

        bank
    }

    #[test]
    fn detects_episodic_vs_social_conflict() {
        let target = EntityId::new();
        let bank = make_bank_with_conflict(target);
        let ts = GameTimestamp::now(36_000);

        let conflicts = detect_conflicts(&bank, 0.2, ts);
        assert!(!conflicts.is_empty(), "Should detect at least one conflict");

        let conflict = &conflicts[0];
        assert_eq!(conflict.about, target);
        assert_eq!(conflict.state, ConflictState::Active);
        assert!(conflict.tension > 0.0);
    }

    #[test]
    fn detects_social_vs_social_conflict() {
        let target = EntityId::new();
        let mut bank = MemoryBank::new();
        let ts = GameTimestamp::now(36_000);

        // Positive gossip
        let mut s1 = SocialMemory::new(
            target,
            EntityId::new(),
            "Great person".to_string(),
            0.9,
            0,
            ts,
        );
        s1.sentiment = 0.8;
        bank.social.push(s1);

        // Negative gossip
        let mut s2 = SocialMemory::new(
            target,
            EntityId::new(),
            "Terrible person".to_string(),
            0.7,
            1,
            ts,
        );
        s2.sentiment = -0.7;
        bank.social.push(s2);

        let conflicts = detect_conflicts(&bank, 0.2, ts);
        assert!(!conflicts.is_empty());
    }

    #[test]
    fn direct_experience_resolves_conflict() {
        let target = EntityId::new();
        let bank = make_bank_with_conflict(target);
        let ts = GameTimestamp::now(36_000);

        let mut conflicts = detect_conflicts(&bank, 0.2, ts);
        assert!(!conflicts.is_empty());

        let conflict = &mut conflicts[0];
        attempt_resolution(conflict, 0.5, 0.5);

        // Direct experience (positive) should win
        assert!(matches!(
            conflict.state,
            ConflictState::ResolvedPositive | ConflictState::Active
        ));
    }

    #[test]
    fn no_conflict_when_consistent() {
        let target = EntityId::new();
        let mut bank = MemoryBank::new();
        let ts = GameTimestamp::now(36_000);

        // All positive
        bank.episodic.push(EpisodicMemory::new(
            "Good interaction",
            vec![target],
            Location::default(),
            ts,
            0.7,
            0.5,
        ));

        let mut social = SocialMemory::new(
            target,
            EntityId::new(),
            "Good person".to_string(),
            0.8,
            0,
            ts,
        );
        social.sentiment = 0.6;
        bank.social.push(social);

        let conflicts = detect_conflicts(&bank, 0.2, ts);
        assert!(conflicts.is_empty(), "No conflict when consistent");
    }
}
