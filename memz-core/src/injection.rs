//! Player Memory Injection Pipeline — backstory memories (§11)
//!
//! Players can inject custom backstory memories into NPCs they interact with.
//! This pipeline validates, sanitises, and integrates player-authored memories.
//!
//! ## Pipeline Stages
//!
//! 1. **Input parsing** — extract memory content from player text
//! 2. **Safety validation** — profanity, plausibility, game-breaking checks
//! 3. **Priority classification** — how important is this backstory element?
//! 4. **Embedding computation** — vectorise for retrieval
//! 5. **Integration** — insert into NPC's memory bank with proper metadata
//!
//! ## Limits
//!
//! - Max 3 injections per NPC per player (prevents spam)
//! - Injected memories decay faster than organic ones
//! - NPCs can "reject" implausible injections based on personality

use crate::config::SafetyConfig;
use crate::memory::injected::{InjectedMemory, InjectedPriority};
use crate::memory::MemoryBank;
use crate::safety::{self, SafetyVerdict};
use crate::types::{EntityId, GameTimestamp, PersonalityTraits};

/// Configuration for the injection pipeline.
#[derive(Debug, Clone)]
pub struct InjectionConfig {
    /// Maximum injections per NPC per player.
    pub max_per_npc_per_player: usize,
    /// Maximum content length (characters).
    pub max_content_length: usize,
    /// Whether LLM validation is enabled (vs rule-based only).
    pub llm_validation_enabled: bool,
    /// Minimum plausibility score to accept (0.0–1.0).
    pub min_plausibility: f32,
}

impl Default for InjectionConfig {
    fn default() -> Self {
        Self {
            max_per_npc_per_player: 3,
            max_content_length: 500,
            llm_validation_enabled: false,
            min_plausibility: 0.3,
        }
    }
}

/// Result of attempting to inject a memory.
#[derive(Debug)]
pub enum InjectionResult {
    /// Memory was accepted and added to the NPC's bank.
    Accepted {
        /// The created memory.
        memory: InjectedMemory,
        /// Any modifications made to the original content.
        modifications: Vec<String>,
    },
    /// Memory was rejected.
    Rejected {
        /// Why it was rejected.
        reason: String,
    },
    /// Memory needs further review (LLM validation pending).
    Pending {
        /// Temporary ID for tracking.
        tracking_id: String,
    },
}

/// Validate and inject a player-authored backstory memory.
///
/// This is the main entry point for the injection pipeline.
pub fn inject_memory(
    content: &str,
    player: EntityId,
    npc_personality: &PersonalityTraits,
    bank: &mut MemoryBank,
    timestamp: GameTimestamp,
    config: &InjectionConfig,
) -> InjectionResult {
    // --- 1. Check rate limit ---
    let existing_from_player = bank
        .injected
        .iter()
        .filter(|m| m.known_to_npcs.contains(&player))
        .count();

    if existing_from_player >= config.max_per_npc_per_player {
        return InjectionResult::Rejected {
            reason: format!(
                "This NPC already has {} backstory memories from you (max: {})",
                existing_from_player, config.max_per_npc_per_player
            ),
        };
    }

    // --- 2. Content length check ---
    if content.len() > config.max_content_length {
        return InjectionResult::Rejected {
            reason: format!(
                "Content too long ({} chars, max: {})",
                content.len(),
                config.max_content_length
            ),
        };
    }

    if content.trim().is_empty() {
        return InjectionResult::Rejected {
            reason: "Content is empty".to_string(),
        };
    }

    // --- 3. Safety validation (rule-based) ---
    let safety_config = SafetyConfig::default();
    match safety::validate_injection(content, &safety_config) {
        Ok(SafetyVerdict::Approved) => {}
        Ok(SafetyVerdict::Rejected { reason }) => {
            return InjectionResult::Rejected { reason };
        }
        Ok(SafetyVerdict::Flagged { .. }) => {
            // Flagged but not rejected — proceed with a note
        }
        Err(e) => {
            return InjectionResult::Rejected {
                reason: format!("Safety check error: {e}"),
            };
        }
    }

    // --- 4. Personality-based plausibility ---
    let plausibility = assess_plausibility(content, npc_personality);
    if plausibility < config.min_plausibility {
        return InjectionResult::Rejected {
            reason: format!(
                "This NPC doesn't find this backstory plausible (score: {:.2}, min: {:.2})",
                plausibility, config.min_plausibility
            ),
        };
    }

    // --- 5. Classify priority ---
    let priority = classify_priority(content);

    // --- 6. Determine emotional weight ---
    let emotional_weight = estimate_emotional_weight(content);

    // --- 7. Create and insert the memory ---
    let mut modifications = Vec::new();

    // Sanitize: trim excessive whitespace
    let clean_content = content.split_whitespace().collect::<Vec<_>>().join(" ");
    if clean_content != content {
        modifications.push("Normalized whitespace".to_string());
    }

    let memory = InjectedMemory::new(
        clean_content,
        emotional_weight,
        timestamp,
        priority,
    )
    .with_known_npcs(vec![player]);

    let memory_clone = memory.clone();
    bank.injected.push(memory);

    InjectionResult::Accepted {
        memory: memory_clone,
        modifications,
    }
}

/// Assess plausibility of an injected memory based on NPC personality.
///
/// More credulous, open NPCs accept more varied backstories.
fn assess_plausibility(content: &str, personality: &PersonalityTraits) -> f32 {
    let base = 0.5;
    let credulity_bonus = personality.credulity * 0.3;
    let openness_bonus = personality.openness * 0.2;

    // Penalize obviously implausible content
    let content_lower = content.to_lowercase();
    let implausibility_penalty = if content_lower.contains("killed a god")
        || content_lower.contains("destroyed the world")
        || content_lower.contains("king of everything")
        || content_lower.contains("immortal")
    {
        0.4
    } else if content_lower.contains("saved the kingdom")
        || content_lower.contains("defeated an army")
    {
        0.2
    } else {
        0.0
    };

    (base + credulity_bonus + openness_bonus - implausibility_penalty).clamp(0.0, 1.0)
}

/// Classify the priority of an injected memory.
fn classify_priority(content: &str) -> InjectedPriority {
    let content_lower = content.to_lowercase();

    if content_lower.contains("tragedy")
        || content_lower.contains("lost")
        || content_lower.contains("died")
        || content_lower.contains("quest")
        || content_lower.contains("sworn")
    {
        InjectedPriority::High
    } else if content_lower.contains("friend")
        || content_lower.contains("family")
        || content_lower.contains("home")
        || content_lower.contains("trained")
    {
        InjectedPriority::Normal
    } else {
        InjectedPriority::Low
    }
}

/// Estimate emotional weight from content keywords.
fn estimate_emotional_weight(content: &str) -> f32 {
    let content_lower = content.to_lowercase();

    let positive_words = [
        "love", "friend", "happy", "saved", "helped", "kind", "brave", "joy",
    ];
    let negative_words = [
        "hate", "enemy", "sad", "killed", "betrayed", "cruel", "fear", "loss",
    ];

    let positive: f32 = positive_words
        .iter()
        .filter(|w| content_lower.contains(*w))
        .count() as f32
        * 0.15;

    let negative: f32 = negative_words
        .iter()
        .filter(|w| content_lower.contains(*w))
        .count() as f32
        * 0.15;

    (positive - negative).clamp(-1.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{EntityId, GameTimestamp, PersonalityTraits};

    fn default_config() -> InjectionConfig {
        InjectionConfig::default()
    }

    fn default_personality() -> PersonalityTraits {
        PersonalityTraits::default()
    }

    #[test]
    fn valid_injection_accepted() {
        let mut bank = MemoryBank::new();
        let player = EntityId::new();
        let ts = GameTimestamp::now(36_000);

        let result = inject_memory(
            "I once helped a merchant caravan cross the desert.",
            player,
            &default_personality(),
            &mut bank,
            ts,
            &default_config(),
        );

        assert!(matches!(result, InjectionResult::Accepted { .. }));
        assert_eq!(bank.injected.len(), 1);
    }

    #[test]
    fn empty_content_rejected() {
        let mut bank = MemoryBank::new();
        let result = inject_memory(
            "",
            EntityId::new(),
            &default_personality(),
            &mut bank,
            GameTimestamp::now(36_000),
            &default_config(),
        );

        assert!(matches!(result, InjectionResult::Rejected { .. }));
    }

    #[test]
    fn rate_limit_enforced() {
        let mut bank = MemoryBank::new();
        let player = EntityId::new();
        let ts = GameTimestamp::now(36_000);
        let config = InjectionConfig {
            max_per_npc_per_player: 2,
            ..Default::default()
        };

        // First two should succeed
        inject_memory("Memory 1", player, &default_personality(), &mut bank, ts, &config);
        inject_memory("Memory 2", player, &default_personality(), &mut bank, ts, &config);

        // Third should fail
        let result = inject_memory(
            "Memory 3",
            player,
            &default_personality(),
            &mut bank,
            ts,
            &config,
        );

        assert!(matches!(result, InjectionResult::Rejected { .. }));
    }

    #[test]
    fn too_long_content_rejected() {
        let mut bank = MemoryBank::new();
        let long_content = "a".repeat(600);

        let result = inject_memory(
            &long_content,
            EntityId::new(),
            &default_personality(),
            &mut bank,
            GameTimestamp::now(36_000),
            &default_config(),
        );

        assert!(matches!(result, InjectionResult::Rejected { .. }));
    }

    #[test]
    fn implausible_content_rejected_by_skeptical_npc() {
        let mut bank = MemoryBank::new();
        let mut personality = default_personality();
        personality.credulity = 0.1;
        personality.openness = 0.1;

        let config = InjectionConfig {
            min_plausibility: 0.5,
            ..Default::default()
        };

        let result = inject_memory(
            "I killed a god and destroyed the world once.",
            EntityId::new(),
            &personality,
            &mut bank,
            GameTimestamp::now(36_000),
            &config,
        );

        assert!(matches!(result, InjectionResult::Rejected { .. }));
    }

    #[test]
    fn credulous_npc_accepts_wild_stories() {
        let mut bank = MemoryBank::new();
        let mut personality = default_personality();
        personality.credulity = 0.9;
        personality.openness = 0.9;

        let result = inject_memory(
            "I saved the kingdom from a terrible drought.",
            EntityId::new(),
            &personality,
            &mut bank,
            GameTimestamp::now(36_000),
            &default_config(),
        );

        assert!(matches!(result, InjectionResult::Accepted { .. }));
    }

    #[test]
    fn emotional_weight_classification() {
        let positive = estimate_emotional_weight("My friend helped me with joy and love");
        assert!(positive > 0.0);

        let negative = estimate_emotional_weight("My enemy betrayed me with cruel hatred");
        assert!(negative < 0.0);

        let neutral = estimate_emotional_weight("I walked to the market yesterday");
        assert!(neutral.abs() < 0.01);
    }

    #[test]
    fn priority_classification() {
        assert_eq!(
            classify_priority("I lost my family in a tragedy"),
            InjectedPriority::High
        );
        assert_eq!(
            classify_priority("I trained with the warriors"),
            InjectedPriority::Normal
        );
        assert_eq!(
            classify_priority("I like the color blue"),
            InjectedPriority::Low
        );
    }
}
