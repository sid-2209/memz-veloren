//! Memory-aware dialogue generation hooks (§12.3).
//!
//! This module provides functions that integrate with Veloren's dialogue system
//! to produce memory-informed NPC responses. It bridges the gap between MEMZ's
//! retrieval engine and Veloren's `DialogueSession` / `Content` system.
//!
//! ## Tiered Approach
//!
//! - **Tier 0 (Rule-based):** Template strings filled with retrieved memories.
//!   Always available, < 0.1ms. Used as fallback when LLM is unavailable.
//! - **Tier 1 (Small LLM):** Memory context → local 1–3B model → response.
//!   50–200ms, async. Used for NPC-initiated dialogue, gossip sharing.
//! - **Tier 2 (Large LLM):** Full context → 7B+ model → rich response.
//!   200ms–2s, async. Used for deep conversations, reflection, bard songs.

use memz_core::behavior::{self, GreetingStyle};
use memz_core::memory::MemoryBank;
use memz_core::replay;
use memz_core::types::{EntityId, GameTimestamp, PersonalityTraits};

use crate::bridge::{DialogueContext, MemorySnippet, SentimentLevel};

// ---------------------------------------------------------------------------
// Greeting Generation (Tier 0)
// ---------------------------------------------------------------------------

/// Generate a memory-aware greeting when a player approaches an NPC.
///
/// Uses the behavior module to compute disposition and greeting style,
/// then selects an appropriate template string. This is the Tier 0
/// (rule-based) path — always fast, always available.
///
/// Returns `(greeting_text, greeting_style)`.
#[must_use]
pub fn generate_greeting(
    bank: &MemoryBank,
    _npc_personality: &PersonalityTraits,
    player: EntityId,
    npc_name: &str,
    current_time: &GameTimestamp,
) -> (String, GreetingStyle) {
    let disposition = behavior::compute_disposition(bank, player);
    let style = behavior::compute_greeting_style(&disposition);

    let text = match style {
        GreetingStyle::Warm => {
            // Check for specific memories to reference
            let replay = replay::select_replay(bank, player, current_time, 0.3);
            if let Some(replay) = replay {
                format!(
                    "Welcome back, friend! {} What brings you here today?",
                    replay.dialogue_hint
                )
            } else {
                format!("Ah, good to see you again! How can {npc_name} help?")
            }
        }
        GreetingStyle::Neutral => {
            "Greetings, traveler. What can I do for you?".to_string()
        }
        GreetingStyle::Cool => {
            "Hmm. What do you want?".to_string()
        }
        GreetingStyle::Hostile => {
            // Check if we remember why we're hostile
            let replay = replay::select_replay(bank, player, current_time, 0.3);
            if let Some(replay) = replay {
                format!("You dare show your face here? {} Leave!", replay.dialogue_hint)
            } else {
                "I don't trust you. State your business or leave.".to_string()
            }
        }
        GreetingStyle::Silent => {
            "...".to_string()
        }
        GreetingStyle::Excited => {
            let replay = replay::select_replay(bank, player, current_time, 0.3);
            if let Some(replay) = replay {
                format!(
                    "Oh! It's you! {} I'm so glad you're here!",
                    replay.dialogue_hint
                )
            } else {
                format!("Oh wonderful! Welcome, welcome! {npc_name} is delighted to see you!")
            }
        }
    };

    (text, style)
}

// ---------------------------------------------------------------------------
// Memory-Enriched Dialogue Response (Tier 0)
// ---------------------------------------------------------------------------

/// Generate a rule-based dialogue response that references NPC memories.
///
/// This is the Tier 0 fallback when no LLM is available. It selects
/// relevant memories and weaves them into template responses.
///
/// Returns the response text.
#[must_use]
pub fn generate_response_rule_based(
    bank: &MemoryBank,
    _npc_personality: &PersonalityTraits,
    player: EntityId,
    _player_action: &str,
    npc_name: &str,
    current_time: &GameTimestamp,
) -> String {
    let disposition = behavior::compute_disposition(bank, player);

    // Try to find a relevant memory to reference
    let replay = replay::select_replay(bank, player, current_time, 0.3);

    let base_response = match disposition.sentiment {
        s if s > 0.5 => format!(
            "I appreciate your kindness, friend. {npc_name} remembers those who treat us well."
        ),
        s if s > 0.0 => {
            "That's interesting. Tell me more.".to_string()
        }
        s if s > -0.5 => {
            "Hmm, I see.".to_string()
        }
        _ => format!(
            "I have no reason to trust you. {npc_name} has a long memory."
        ),
    };

    // Enrich with memory reference if available
    if let Some(replay) = replay {
        format!("{base_response} {}", replay.dialogue_hint)
    } else {
        base_response
    }
}

// ---------------------------------------------------------------------------
// Sentiment Dialogue (Tier 0)
// ---------------------------------------------------------------------------

/// Generate what the NPC says when asked "What do you think of me?"
///
/// This replaces Veloren's simple 3-tier sentiment response with a
/// memory-enriched version that references specific past events.
#[must_use]
pub fn generate_sentiment_response(
    bank: &MemoryBank,
    _npc_personality: &PersonalityTraits,
    player: EntityId,
    npc_name: &str,
    sentiment: SentimentLevel,
    current_time: &GameTimestamp,
) -> String {
    let replay = replay::select_replay(bank, player, current_time, 0.3);
    let memory_ref = replay
        .as_ref()
        .map(|r| r.dialogue_hint.clone())
        .unwrap_or_default();

    match sentiment {
        SentimentLevel::Hero => format!(
            "You are a true hero to {npc_name} and everyone here! {memory_ref}"
        ),
        SentimentLevel::Friend => format!(
            "I consider you a dear friend. {memory_ref}"
        ),
        SentimentLevel::Ally => format!(
            "You've been good to us. I trust you. {memory_ref}"
        ),
        SentimentLevel::Positive => format!(
            "You seem decent enough. {memory_ref}"
        ),
        SentimentLevel::Neutral => {
            if memory_ref.is_empty() {
                "I don't know much about you, to be honest.".to_string()
            } else {
                format!("I'm not sure what to make of you yet. {memory_ref}")
            }
        }
        SentimentLevel::Negative => format!(
            "I'm wary of you, if I'm being honest. {memory_ref}"
        ),
        SentimentLevel::Rival => format!(
            "I don't trust you one bit. {memory_ref}"
        ),
        SentimentLevel::Enemy => format!(
            "You've caused enough trouble. Stay away from me. {memory_ref}"
        ),
        SentimentLevel::Villain => format!(
            "You are despised here. {memory_ref} Leave before something bad happens."
        ),
    }
}

// ---------------------------------------------------------------------------
// Gossip Sharing (Tier 0)
// ---------------------------------------------------------------------------

/// Generate gossip text when an NPC shares information during dialogue.
///
/// The NPC selects the most interesting social memory to share with the player.
#[must_use]
pub fn generate_gossip_text(
    bank: &MemoryBank,
    _npc_personality: &PersonalityTraits,
    _npc_name: &str,
) -> Option<String> {
    // Use a dummy listener ID — we just want the top gossip items
    let dummy_listener = EntityId::new();
    let gossip_candidates = behavior::select_gossip(bank, dummy_listener, 3);

    if gossip_candidates.is_empty() {
        return None;
    }

    // Pick the most impactful gossip
    let best = &gossip_candidates[0];

    let reliability = if best.propagation_depth == 0 {
        "I saw it with my own eyes"
    } else if best.propagation_depth == 1 {
        "I heard from someone who was there"
    } else {
        "Word around town is"
    };

    Some(format!(
        "{reliability} — {}",
        best.claim
    ))
}

// ---------------------------------------------------------------------------
// Price Modifier for Trading (Tier 0)
// ---------------------------------------------------------------------------

/// Get the price modifier based on NPC's memories of the player.
///
/// Integrates with Veloren's trading system. Returns a multiplier:
/// - < 1.0 = discount (NPC likes player)
/// - 1.0 = standard price
/// - > 1.0 = markup (NPC dislikes player)
#[must_use]
pub fn get_price_modifier(
    bank: &MemoryBank,
    _npc_personality: &PersonalityTraits,
    player: &EntityId,
) -> f32 {
    let disposition = behavior::compute_disposition(bank, *player);
    behavior::compute_price_modifier(&disposition)
}

// ---------------------------------------------------------------------------
// Dialogue Context Assembly
// ---------------------------------------------------------------------------

/// Assemble a full `DialogueContext` from MEMZ state for LLM prompt generation.
///
/// This is used by Tier 1/2 when an LLM is available. The context is
/// serialized into template variables and fed to the prompt engine.
#[must_use]
pub fn assemble_dialogue_context(
    bank: &MemoryBank,
    npc_name: &str,
    npc_profession: &str,
    settlement_name: &str,
    npc_personality: &PersonalityTraits,
    player: EntityId,
    player_action: &str,
    sentiment: SentimentLevel,
    current_time: &GameTimestamp,
) -> DialogueContext {
    // Retrieve top memories about this player
    let snippets = extract_memory_snippets(bank, &player, current_time, 5);

    DialogueContext {
        npc_name: npc_name.to_string(),
        npc_profession: npc_profession.to_string(),
        settlement_name: settlement_name.to_string(),
        personality_description: DialogueContext::describe_personality(npc_personality),
        pad_state: sentiment.to_pad_modifier(),
        sentiment,
        top_memories: snippets,
        player_action: player_action.to_string(),
        context_description: format!(
            "Player approaches {npc_name} the {npc_profession} in {settlement_name}"
        ),
    }
}

/// Extract the top-K most relevant memory snippets about a target entity.
fn extract_memory_snippets(
    bank: &MemoryBank,
    target: &EntityId,
    current_time: &GameTimestamp,
    top_k: usize,
) -> Vec<MemorySnippet> {
    let mut snippets = Vec::new();

    // Episodic memories involving the target
    for ep in &bank.episodic {
        if ep.participants.contains(target) {
            snippets.push(MemorySnippet {
                memory_type: "episodic".to_string(),
                summary: ep.event.clone(),
                strength: ep.strength,
                age_days: current_time.days_since(&ep.timestamp),
            });
        }
    }

    // Social memories about the target
    for soc in &bank.social {
        if soc.about == *target {
            snippets.push(MemorySnippet {
                memory_type: "social".to_string(),
                summary: soc.claim.clone(),
                strength: soc.trust_in_source,
                age_days: current_time.days_since(&soc.received_at),
            });
        }
    }

    // Emotional memories about the target
    for emo in &bank.emotional {
        if emo.target == *target {
            snippets.push(MemorySnippet {
                memory_type: "emotional".to_string(),
                summary: format!("{} toward entity (intensity: {:.1})", emo.emotion, emo.intensity),
                strength: emo.intensity,
                age_days: current_time.days_since(&emo.last_updated),
            });
        }
    }

    // Sort by a composite score: strength * recency
    snippets.sort_by(|a, b| {
        let score_a = a.strength / (1.0 + a.age_days);
        let score_b = b.strength / (1.0 + b.age_days);
        score_b
            .partial_cmp(&score_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    snippets.truncate(top_k);
    snippets
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use memz_core::memory::episodic::EpisodicMemory;
    use memz_core::memory::social::SocialMemory;
    use memz_core::types::Location;

    fn ts(tick: u64) -> GameTimestamp {
        GameTimestamp::now(tick)
    }

    fn make_bank_with_history(player: EntityId) -> MemoryBank {
        use memz_core::memory::emotional::EmotionalMemory;
        use memz_core::types::PADState;

        let mut bank = MemoryBank::new();

        bank.episodic.push(EpisodicMemory::new(
            format!("Entity {player} helped defend the forge from bandits"),
            vec![player],
            Location::default(),
            ts(1000),
            0.9,
            0.9,
        ));

        bank.episodic.push(EpisodicMemory::new(
            format!("Entity {player} saved my life from a fire"),
            vec![player],
            Location::default(),
            ts(1500),
            0.9,
            0.9,
        ));

        bank.episodic.push(EpisodicMemory::new(
            format!("Entity {player} bought an iron sword"),
            vec![player],
            Location::default(),
            ts(2000),
            0.3,
            0.5,
        ));

        // Emotional memory: strong positive feeling toward player
        bank.emotional.push(EmotionalMemory::new(
            player,
            "gratitude",
            0.9,
            PADState::new(0.8, 0.3, 0.2),
            vec![],
            ts(1500),
        ));

        let mut social = SocialMemory::new(
            player,
            EntityId::new(),
            format!("Entity {player} is known for helping travelers"),
            0.8,
            1,
            ts(500),
        );
        social.sentiment = 0.7;
        bank.social.push(social);

        bank
    }

    #[test]
    fn warm_greeting_for_positive_history() {
        let player = EntityId::new();
        let bank = make_bank_with_history(player);
        let personality = PersonalityTraits::default();

        let (text, style) = generate_greeting(
            &bank,
            &personality,
            player,
            "Goran",
            &ts(3000),
        );

        // Should be warm or excited
        assert!(
            matches!(style, GreetingStyle::Warm | GreetingStyle::Excited),
            "Expected warm/excited, got {:?}",
            style
        );
        assert!(!text.is_empty());
    }

    #[test]
    fn neutral_greeting_for_unknown_player() {
        let player = EntityId::new();
        let bank = MemoryBank::new();
        let personality = PersonalityTraits::default();

        let (text, style) = generate_greeting(
            &bank,
            &personality,
            player,
            "Mira",
            &ts(1000),
        );

        assert_eq!(style, GreetingStyle::Neutral);
        assert!(text.contains("traveler") || text.contains("Greetings"));
    }

    #[test]
    fn sentiment_response_references_memories() {
        let player = EntityId::new();
        let bank = make_bank_with_history(player);
        let personality = PersonalityTraits::default();

        let response = generate_sentiment_response(
            &bank,
            &personality,
            player,
            "Goran",
            SentimentLevel::Ally,
            &ts(3000),
        );

        assert!(response.contains("trust") || response.contains("good"));
    }

    #[test]
    fn gossip_from_social_memories() {
        let mut bank = MemoryBank::new();
        let subject = EntityId::new();

        bank.social.push({
            let mut s = SocialMemory::new(
                subject,
                EntityId::new(),
                "The mayor was seen sneaking out at night",
                0.9,
                0,
                ts(1000),
            );
            s.sentiment = -0.5;
            s
        });

        let personality = PersonalityTraits {
            gossip_tendency: 0.8,
            ..Default::default()
        };

        let gossip = generate_gossip_text(&bank, &personality, "Mira");
        assert!(gossip.is_some());
        assert!(gossip.unwrap().contains("mayor"));
    }

    #[test]
    fn price_modifier_for_liked_player() {
        let player = EntityId::new();
        let bank = make_bank_with_history(player);
        let personality = PersonalityTraits::default();

        let modifier = get_price_modifier(&bank, &personality, &player);
        // Positive history → discount (modifier < 1.0)
        assert!(modifier <= 1.0);
    }

    #[test]
    fn dialogue_context_assembly() {
        let player = EntityId::new();
        let bank = make_bank_with_history(player);
        let personality = PersonalityTraits::default();

        let ctx = assemble_dialogue_context(
            &bank,
            "Goran",
            "Blacksmith",
            "Ironhaven",
            &personality,
            player,
            "asked about swords",
            SentimentLevel::Ally,
            &ts(3000),
        );

        assert_eq!(ctx.npc_name, "Goran");
        assert!(!ctx.top_memories.is_empty());

        let vars = ctx.to_template_vars();
        assert!(vars.contains_key("npc_name"));
        assert!(vars.contains_key("top_memories"));
    }

    #[test]
    fn extract_snippets_ranked_by_relevance() {
        let player = EntityId::new();
        let bank = make_bank_with_history(player);

        let snippets = extract_memory_snippets(&bank, &player, &ts(3000), 5);
        assert!(!snippets.is_empty());

        // Should be sorted by composite score (strength * recency)
        if snippets.len() >= 2 {
            let score_a = snippets[0].strength / (1.0 + snippets[0].age_days);
            let score_b = snippets[1].strength / (1.0 + snippets[1].age_days);
            assert!(score_a >= score_b);
        }
    }
}
