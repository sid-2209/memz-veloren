//! Memory Rule — binds Veloren game events to MEMZ memory creation (§12.2).
//!
//! In a full Veloren integration this would implement `rtsim::Rule` and
//! bind to `OnDeath`, `OnTheft`, `OnHelped`, `OnTick`, etc. via `RtState::bind`.
//!
//! Since we can't directly depend on the Veloren rtsim crate (circular dep),
//! we model the same pattern: a struct that holds state and functions that
//! process event types, ready to be wired in by a thin Veloren-side adapter.

use memz_core::config::MemoryConfig;
use memz_core::decay;
use memz_core::memory::episodic::EpisodicMemory;
use memz_core::memory::social::SocialMemory;
use memz_core::memory::MemoryBank;
use memz_core::reflection::{self, ReflectionConfig};
use memz_core::reputation::{ReputationBoard, NotableDeed};
use memz_core::social;
use memz_core::types::{EntityId, GameTimestamp, Location, PersonalityTraits, SettlementId};

use crate::bridge::EntityRegistry;
use crate::events::{CombatOutcome, GameEvent};

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Memory Rule State
// ---------------------------------------------------------------------------

/// Central state for the MEMZ memory rule.
///
/// Holds per-NPC memory banks, the entity registry, configuration,
/// and settlement reputation boards. In a full integration, this would
/// be stored as the Rule state inside `RtState`.
pub struct MemoryRule {
    /// Per-entity memory banks.
    pub banks: HashMap<EntityId, MemoryBank>,
    /// Per-entity personality traits (cached from Veloren Personality).
    pub personalities: HashMap<EntityId, PersonalityTraits>,
    /// Entity ID registry (Veloren ↔ MEMZ).
    pub registry: EntityRegistry,
    /// Per-settlement reputation boards.
    pub reputation_boards: HashMap<SettlementId, ReputationBoard>,
    /// Memory system configuration.
    pub config: MemoryConfig,
    /// Current game tick (updated each frame).
    pub current_tick: u64,
}

impl MemoryRule {
    /// Create a new memory rule with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self {
            banks: HashMap::new(),
            personalities: HashMap::new(),
            registry: EntityRegistry::new(),
            reputation_boards: HashMap::new(),
            config: MemoryConfig::default(),
            current_tick: 0,
        }
    }

    /// Create with a specific configuration.
    #[must_use]
    pub fn with_config(config: MemoryConfig) -> Self {
        Self {
            config,
            ..Self::new()
        }
    }

    /// Get or create a memory bank for an entity.
    pub fn bank_mut(&mut self, entity: EntityId) -> &mut MemoryBank {
        self.banks.entry(entity).or_insert_with(MemoryBank::new)
    }

    /// Get a memory bank (read-only).
    #[must_use]
    pub fn bank(&self, entity: EntityId) -> Option<&MemoryBank> {
        self.banks.get(&entity)
    }

    /// Get or create personality for an entity.
    pub fn personality(&self, entity: &EntityId) -> PersonalityTraits {
        self.personalities
            .get(entity)
            .copied()
            .unwrap_or_default()
    }

    /// Set personality for an entity (called when NPC is first loaded).
    pub fn set_personality(&mut self, entity: EntityId, traits: PersonalityTraits) {
        self.personalities.insert(entity, traits);
    }

    /// Get or create a reputation board for a settlement.
    pub fn reputation_board(&mut self, settlement: SettlementId, timestamp: GameTimestamp) -> &mut ReputationBoard {
        self.reputation_boards
            .entry(settlement)
            .or_insert_with(|| ReputationBoard::new(settlement, timestamp))
    }

    /// Number of active memory banks.
    #[must_use]
    pub fn active_npc_count(&self) -> usize {
        self.banks.len()
    }
}

impl Default for MemoryRule {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Event Handlers
// ---------------------------------------------------------------------------

/// Process a death event — witnesses create episodic+emotional memories,
/// and the settlement reputation board is updated.
///
/// Maps to Veloren `OnDeath { actor, wpos, killer }`.
pub fn on_death(
    rule: &mut MemoryRule,
    deceased: EntityId,
    killer: Option<EntityId>,
    witnesses: &[EntityId],
    location: Location,
    settlement: Option<SettlementId>,
    timestamp: GameTimestamp,
) {
    let cause = if let Some(k) = killer {
        format!("killed by entity {k}")
    } else {
        "died of unknown causes".to_string()
    };

    // Create episodic memory for each witness
    for &witness in witnesses {
        let description = format!(
            "Witnessed the death of entity {deceased}: {cause}"
        );

        let episodic = EpisodicMemory::new(
            description,
            {
                let mut p = vec![deceased];
                if let Some(k) = killer {
                    p.push(k);
                }
                p
            },
            location,
            timestamp,
            -0.8, // Death is very negative
            0.9,  // Death is very important
        );

        rule.bank_mut(witness).episodic.push(episodic);

        // Gossip propagation — witnesses will remember and may tell others
        if let Some(k) = killer {
            let claim = format!(
                "Entity {k} killed entity {deceased}"
            );
            let _personality = rule.personality(&witness);
            let social = SocialMemory::new(
                k,
                witness, // self-sourced (direct observation)
                claim,
                1.0, // full trust — witnessed it
                0,   // first-hand
                timestamp,
            );
            rule.bank_mut(witness).social.push(social);

            // Update reputation if in a settlement
            if let Some(settlement_id) = settlement {
                let board = rule.reputation_board(settlement_id, timestamp);
                board.report_sentiment(k, -0.5, timestamp); // Killing is bad
                board.record_deed(NotableDeed {
                    actor: k,
                    description: format!("Killed {deceased}"),
                    valence: -0.8,
                    timestamp,
                    witness_count: witnesses.len() as u32,
                });

                // If killer was defending, less negative
                // (Future: check combat context for justified killing)
            }
        }
    }
}

/// Process a theft event — nearby NPCs form negative memories of the thief.
///
/// Maps to Veloren `OnTheft { actor, wpos, sprite, site }`.
pub fn on_theft(
    rule: &mut MemoryRule,
    thief: EntityId,
    witnesses: &[EntityId],
    item_description: &str,
    location: Location,
    settlement: Option<SettlementId>,
    timestamp: GameTimestamp,
) {
    for &witness in witnesses {
        let description = format!(
            "Witnessed entity {thief} steal {item_description}"
        );

        let episodic = EpisodicMemory::new(
            description,
            vec![thief],
            location,
            timestamp,
            -0.5,
            0.6,
        );
        rule.bank_mut(witness).episodic.push(episodic);

        // Social memory — can gossip about the thief
        let social = SocialMemory::new(
            thief,
            witness,
            format!("{thief} is a thief — stole {item_description}"),
            1.0,
            0,
            timestamp,
        );
        rule.bank_mut(witness).social.push(social);
    }

    // Reputation hit
    if let Some(settlement_id) = settlement {
        let board = rule.reputation_board(settlement_id, timestamp);
        board.report_sentiment(thief, -0.3, timestamp);
        board.record_deed(NotableDeed {
            actor: thief,
            description: format!("Stole {item_description}"),
            valence: -0.5,
            timestamp,
            witness_count: witnesses.len() as u32,
        });
    }
}

/// Process a help event — the helped NPC and witnesses form positive memories.
///
/// Maps to Veloren `OnHelped { actor, saver }`.
pub fn on_helped(
    rule: &mut MemoryRule,
    helped: EntityId,
    helper: EntityId,
    action: &str,
    witnesses: &[EntityId],
    location: Location,
    settlement: Option<SettlementId>,
    timestamp: GameTimestamp,
) {
    // The helped entity remembers vividly
    let description = format!("Entity {helper} helped me: {action}");
    let episodic = EpisodicMemory::new(
        description,
        vec![helper],
        location,
        timestamp,
        0.7,
        0.7,
    );
    rule.bank_mut(helped).episodic.push(episodic);

    // Witnesses also remember
    for &witness in witnesses {
        if witness == helped {
            continue;
        }
        let desc = format!("Witnessed entity {helper} help entity {helped}: {action}");
        let ep = EpisodicMemory::new(
            desc,
            vec![helper, helped],
            location,
            timestamp,
            0.5,
            0.5,
        );
        rule.bank_mut(witness).episodic.push(ep);
    }

    // Positive reputation
    if let Some(settlement_id) = settlement {
        let board = rule.reputation_board(settlement_id, timestamp);
        board.report_sentiment(helper, 0.3, timestamp);
        board.record_deed(NotableDeed {
            actor: helper,
            description: format!("Helped {helped}: {action}"),
            valence: 0.5,
            timestamp,
            witness_count: (witnesses.len() + 1) as u32,
        });
    }
}

/// Process a trade event — both parties remember the fairness.
pub fn on_trade(
    rule: &mut MemoryRule,
    buyer: EntityId,
    seller: EntityId,
    item: &str,
    fairness: f32,
    location: Location,
    timestamp: GameTimestamp,
) {
    let buyer_desc = if fairness > 0.3 {
        format!("Got a good deal buying {item} from entity {seller}")
    } else if fairness < -0.3 {
        format!("Got ripped off buying {item} from entity {seller}")
    } else {
        format!("Bought {item} from entity {seller} at a fair price")
    };

    let seller_desc = if fairness > 0.3 {
        format!("Sold {item} to entity {buyer} at a generous price")
    } else if fairness < -0.3 {
        format!("Made a great profit selling {item} to entity {buyer}")
    } else {
        format!("Sold {item} to entity {buyer} at a fair price")
    };

    let buyer_ep = EpisodicMemory::new(
        buyer_desc,
        vec![seller],
        location,
        timestamp,
        fairness * 0.5,
        0.3,
    );
    rule.bank_mut(buyer).episodic.push(buyer_ep);

    let seller_ep = EpisodicMemory::new(
        seller_desc,
        vec![buyer],
        location,
        timestamp,
        -fairness * 0.3, // Seller has inverse feeling about fairness
        0.3,
    );
    rule.bank_mut(seller).episodic.push(seller_ep);
}

/// Process a combat event — attacker, defender, and witnesses all form memories.
pub fn on_combat(
    rule: &mut MemoryRule,
    attacker: EntityId,
    defender: EntityId,
    attacker_won: bool,
    witnesses: &[EntityId],
    location: Location,
    settlement: Option<SettlementId>,
    timestamp: GameTimestamp,
) {
    let outcome_str = if attacker_won { "won" } else { "lost" };

    // Attacker's memory
    let atk_desc = format!(
        "Fought entity {defender} and {outcome_str}"
    );
    let atk_valence = if attacker_won { 0.3 } else { -0.4 };
    rule.bank_mut(attacker).episodic.push(EpisodicMemory::new(
        atk_desc,
        vec![defender],
        location,
        timestamp,
        atk_valence,
        0.7,
    ));

    // Defender's memory
    let def_desc = format!(
        "Was attacked by entity {attacker} — {}", if attacker_won { "I lost" } else { "I won" }
    );
    let def_valence = if attacker_won { -0.5 } else { 0.2 };
    rule.bank_mut(defender).episodic.push(EpisodicMemory::new(
        def_desc,
        vec![attacker],
        location,
        timestamp,
        def_valence,
        0.8,
    ));

    // Witnesses
    for &witness in witnesses {
        if witness == attacker || witness == defender {
            continue;
        }
        let w_desc = format!(
            "Witnessed a fight between entity {attacker} and entity {defender} — attacker {outcome_str}"
        );
        rule.bank_mut(witness).episodic.push(EpisodicMemory::new(
            w_desc,
            vec![attacker, defender],
            location,
            timestamp,
            -0.3,
            0.6,
        ));
    }

    // Reputation: fighting in a settlement is generally bad
    if let Some(settlement_id) = settlement {
        let board = rule.reputation_board(settlement_id, timestamp);
        board.report_sentiment(attacker, -0.2, timestamp); // Attacker reputation hit
    }
}

/// Run periodic tick processing for all active NPCs.
///
/// Called from Veloren's `OnTick` handler. Performs:
/// 1. Memory decay (every 60 ticks)
/// 2. Reflection check (every 5000 ticks ≈ 5 game-minutes)
/// 3. Memory limit enforcement
///
/// Budget: < 0.5ms for 50 active NPCs.
pub fn on_tick(
    rule: &mut MemoryRule,
    tick: u64,
    _dt: f32,
) {
    rule.current_tick = tick;
    let config = rule.config.clone();
    let timestamp = GameTimestamp::now(tick);
    let reflection_config = ReflectionConfig::default();

    // Decay runs every 60 ticks (~1 second at 60 FPS)
    if tick % 60 == 0 {
        for bank in rule.banks.values_mut() {
            decay::decay_episodic_memories(&mut bank.episodic, &timestamp, &config);
            decay::decay_social_memories(&mut bank.social, &timestamp, f64::from(config.decay_rate));
        }
    }

    // Reflection check runs every 5000 ticks (~5 game-minutes)
    if tick % 5000 == 0 {
        let entities: Vec<EntityId> = rule.banks.keys().copied().collect();
        for entity in entities {
            let _personality = rule.personality(&entity);
            let bank = rule.banks.get(&entity);
            if let Some(bank) = bank {
                let last_reflection_tick = bank
                    .reflective
                    .last()
                    .map(|r| r.generated_at.tick)
                    .unwrap_or(0);
                let _should = reflection::should_reflect(
                    last_reflection_tick,
                    tick,
                    bank.episodic.len(),
                    bank.episodic.iter().map(|e| e.emotional_valence.abs()).fold(0.0_f32, f32::max),
                    &reflection_config,
                );
                // If should_reflect is true, queue an async LLM reflection job.
                // The actual LLM call happens off-thread via memz-llm.
                // For now, we log the intent.
                // TODO: Wire up to LlmQueue for async reflection generation.
            }
        }
    }

    // Memory limit enforcement runs every 300 ticks
    if tick % 300 == 0 {
        for bank in rule.banks.values_mut() {
            bank.episodic.truncate(config.max_episodic_per_npc);
            bank.semantic.truncate(config.max_semantic_per_npc);
            bank.social.truncate(config.max_social_per_npc);
            bank.procedural.truncate(config.max_procedural_per_npc);
            bank.reflective.truncate(config.max_reflective_per_npc);
        }
    }

    // Reputation decay runs every 10000 ticks
    if tick % 10_000 == 0 {
        for board in rule.reputation_boards.values_mut() {
            board.decay_reputations(0.02, timestamp);
        }
    }
}

/// Process a GameEvent through the full MEMZ pipeline.
///
/// This is the main entry point for converting high-level game events
/// to memory operations. It dispatches to the appropriate handler
/// based on event type.
pub fn process_game_event(
    rule: &mut MemoryRule,
    event: &GameEvent,
    settlement: Option<SettlementId>,
) {
    let timestamp = *event.timestamp();
    let location = *event.location();

    match event {
        GameEvent::Death {
            entity,
            cause: _,
            witnesses,
            ..
        } => {
            // We don't have killer info in the Death event directly,
            // so we pass None for killer.
            on_death(rule, *entity, None, witnesses, location, settlement, timestamp);
        }

        GameEvent::Harmed {
            perpetrator,
            victim,
            action,
            witnesses,
            ..
        } => {
            // Treat harm similar to theft for reputation purposes
            on_theft(rule, *perpetrator, witnesses, action, location, settlement, timestamp);

            // Victim also remembers
            let desc = format!("Entity {perpetrator} harmed me: {action}");
            rule.bank_mut(*victim).episodic.push(EpisodicMemory::new(
                desc,
                vec![*perpetrator],
                location,
                timestamp,
                -0.7,
                0.7,
            ));
        }

        GameEvent::Helped {
            helper,
            helped,
            action,
            witnesses,
            ..
        } => {
            on_helped(rule, *helped, *helper, action, witnesses, location, settlement, timestamp);
        }

        GameEvent::Combat {
            attacker,
            defender,
            outcome,
            witnesses,
            ..
        } => {
            let attacker_won = matches!(outcome, CombatOutcome::AttackerWon);
            on_combat(
                rule,
                *attacker,
                *defender,
                attacker_won,
                witnesses,
                location,
                settlement,
                timestamp,
            );
        }

        GameEvent::Trade {
            buyer,
            seller,
            item,
            perceived_fairness,
            ..
        } => {
            on_trade(rule, *buyer, *seller, item, *perceived_fairness, location, timestamp);
        }

        GameEvent::Dialogue {
            speaker,
            listener,
            content,
            ..
        } => {
            let desc = format!("Entity {speaker} said: \"{content}\"");
            rule.bank_mut(*listener).episodic.push(EpisodicMemory::new(
                desc.clone(),
                vec![*speaker],
                location,
                timestamp,
                0.1,
                0.3,
            ));
            // Speaker also remembers what they said
            let speaker_desc = format!("I told entity {listener}: \"{content}\"");
            rule.bank_mut(*speaker).episodic.push(EpisodicMemory::new(
                speaker_desc,
                vec![*listener],
                location,
                timestamp,
                0.1,
                0.2,
            ));
        }

        GameEvent::Arrival {
            entity, observers, ..
        } => {
            for &observer in observers {
                let desc = format!("Entity {entity} arrived at {location}");
                rule.bank_mut(observer).episodic.push(EpisodicMemory::new(
                    desc,
                    vec![*entity],
                    location,
                    timestamp,
                    0.0,
                    0.2,
                ));
            }
        }

        GameEvent::QuestEvent {
            entity,
            quest_name,
            event_type,
            witnesses,
            ..
        } => {
            let desc = format!("Quest '{quest_name}': {event_type}");
            rule.bank_mut(*entity).episodic.push(EpisodicMemory::new(
                desc.clone(),
                witnesses.clone(),
                location,
                timestamp,
                0.5,
                0.8,
            ));
            for &witness in witnesses {
                if witness == *entity {
                    continue;
                }
                rule.bank_mut(witness).episodic.push(EpisodicMemory::new(
                    format!("Witnessed {desc} by entity {entity}"),
                    vec![*entity],
                    location,
                    timestamp,
                    0.3,
                    0.5,
                ));
            }
        }

        GameEvent::Custom {
            description,
            participants,
            emotional_valence,
            importance,
            ..
        } => {
            for &participant in participants {
                rule.bank_mut(participant).episodic.push(EpisodicMemory::new(
                    description.clone(),
                    participants.clone(),
                    location,
                    timestamp,
                    *emotional_valence,
                    *importance,
                ));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Gossip Propagation
// ---------------------------------------------------------------------------

/// When two NPCs interact, they may share gossip based on personality.
///
/// Called during NPC-NPC interactions (dialogue, proximity in taverns, etc.).
/// Budget: < 0.3ms per interaction pair.
pub fn propagate_gossip(
    rule: &mut MemoryRule,
    speaker: EntityId,
    listener: EntityId,
    timestamp: GameTimestamp,
) {
    let speaker_personality = rule.personality(&speaker);
    let listener_personality = rule.personality(&listener);

    // Get speaker's social memories to potentially share
    let gossip_candidates: Vec<SocialMemory> = rule
        .bank(speaker)
        .map(|b| b.social.clone())
        .unwrap_or_default();

    if gossip_candidates.is_empty() {
        return;
    }

    // Sort by most interesting gossip (highest |sentiment|)
    let mut candidates = gossip_candidates;
    candidates.sort_by(|a, b| {
        b.sentiment
            .abs()
            .partial_cmp(&a.sentiment.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Share at most 2 pieces of gossip per interaction
    for gossip in candidates.iter().take(2) {
        // Speaker decides whether to gossip based on personality + memory importance
        let gossip_prob = social::gossip_probability(
            &speaker_personality,
            gossip.sentiment.abs(), // use |sentiment| as importance proxy
            gossip.sentiment.abs() * 0.8, // emotional weight
        );

        if gossip_prob <= 0.5 {
            continue;
        }

        let result = social::propagate_memory(
            gossip,
            listener,
            &listener_personality,
            gossip.trust_in_source * 0.8, // Trust degrades in transmission
            false,  // listener has no direct experience
            None,   // no direct sentiment
            0.7,    // moderately consistent with existing beliefs
            0.0,    // neutral emotional state
            0.7,    // reasonably reliable source
            timestamp,
        );
        if let social::PropagationResult::Accepted { new_memory, .. } = result {
            rule.bank_mut(listener).social.push(new_memory);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(tick: u64) -> GameTimestamp {
        GameTimestamp::now(tick)
    }

    fn loc() -> Location {
        Location::default()
    }

    #[test]
    fn death_creates_witness_memories() {
        let mut rule = MemoryRule::new();
        let deceased = EntityId::new();
        let killer = EntityId::new();
        let w1 = EntityId::new();
        let w2 = EntityId::new();
        let settlement = SettlementId::new();

        on_death(
            &mut rule,
            deceased,
            Some(killer),
            &[w1, w2],
            loc(),
            Some(settlement),
            ts(1000),
        );

        // Both witnesses should have episodic + social memories
        assert_eq!(rule.bank(w1).unwrap().episodic.len(), 1);
        assert_eq!(rule.bank(w1).unwrap().social.len(), 1);
        assert_eq!(rule.bank(w2).unwrap().episodic.len(), 1);

        // Reputation should be updated
        let board = rule.reputation_board(settlement, ts(1000));
        assert!(board.get_reputation(killer).unwrap().score < 0.0);
    }

    #[test]
    fn theft_creates_negative_memories() {
        let mut rule = MemoryRule::new();
        let thief = EntityId::new();
        let witness = EntityId::new();
        let settlement = SettlementId::new();

        on_theft(
            &mut rule,
            thief,
            &[witness],
            "a loaf of bread",
            loc(),
            Some(settlement),
            ts(2000),
        );

        let bank = rule.bank(witness).unwrap();
        assert_eq!(bank.episodic.len(), 1);
        assert!(bank.episodic[0].event.contains("steal"));
        assert_eq!(bank.social.len(), 1);
        assert!(bank.social[0].claim.contains("thief"));
    }

    #[test]
    fn help_creates_positive_memories() {
        let mut rule = MemoryRule::new();
        let helper = EntityId::new();
        let helped = EntityId::new();
        let witness = EntityId::new();
        let settlement = SettlementId::new();

        on_helped(
            &mut rule,
            helped,
            helper,
            "defended from wolves",
            &[helped, witness],
            loc(),
            Some(settlement),
            ts(3000),
        );

        // Helped entity has vivid memory
        assert_eq!(rule.bank(helped).unwrap().episodic.len(), 1);
        assert!(rule.bank(helped).unwrap().episodic[0].emotional_valence > 0.0);

        // Witness also remembers
        assert_eq!(rule.bank(witness).unwrap().episodic.len(), 1);

        // Helper gets positive reputation
        let board = rule.reputation_board(settlement, ts(3000));
        assert!(board.get_reputation(helper).unwrap().score > 0.0);
    }

    #[test]
    fn trade_creates_dual_memories() {
        let mut rule = MemoryRule::new();
        let buyer = EntityId::new();
        let seller = EntityId::new();

        on_trade(&mut rule, buyer, seller, "iron sword", 0.5, loc(), ts(4000));

        assert_eq!(rule.bank(buyer).unwrap().episodic.len(), 1);
        assert_eq!(rule.bank(seller).unwrap().episodic.len(), 1);

        // Buyer got a good deal → positive memory
        assert!(rule.bank(buyer).unwrap().episodic[0].emotional_valence > 0.0);
    }

    #[test]
    fn combat_creates_memories_for_all() {
        let mut rule = MemoryRule::new();
        let attacker = EntityId::new();
        let defender = EntityId::new();
        let witness = EntityId::new();
        let settlement = SettlementId::new();

        on_combat(
            &mut rule,
            attacker,
            defender,
            true, // attacker won
            &[witness],
            loc(),
            Some(settlement),
            ts(5000),
        );

        assert_eq!(rule.bank(attacker).unwrap().episodic.len(), 1);
        assert_eq!(rule.bank(defender).unwrap().episodic.len(), 1);
        assert_eq!(rule.bank(witness).unwrap().episodic.len(), 1);
    }

    #[test]
    fn process_game_event_dialogue() {
        let mut rule = MemoryRule::new();
        let speaker = EntityId::new();
        let listener = EntityId::new();

        let event = GameEvent::Dialogue {
            speaker,
            listener,
            content: "Hello, friend!".to_string(),
            location: loc(),
            timestamp: ts(6000),
        };

        process_game_event(&mut rule, &event, None);

        // Both speaker and listener should remember
        assert_eq!(rule.bank(speaker).unwrap().episodic.len(), 1);
        assert_eq!(rule.bank(listener).unwrap().episodic.len(), 1);
    }

    #[test]
    fn on_tick_enforces_limits() {
        let mut rule = MemoryRule::new();
        let entity = EntityId::new();

        // Add many memories
        for i in 0..300 {
            rule.bank_mut(entity).episodic.push(EpisodicMemory::new(
                format!("Event {i}"),
                vec![],
                loc(),
                ts(i),
                0.0,
                0.1,
            ));
        }

        assert_eq!(rule.bank(entity).unwrap().episodic.len(), 300);

        // Tick 300 triggers enforcement
        on_tick(&mut rule, 300, 1.0 / 60.0);

        assert!(rule.bank(entity).unwrap().episodic.len() <= rule.config.max_episodic_per_npc);
    }

    #[test]
    fn gossip_propagation_between_npcs() {
        let mut rule = MemoryRule::new();
        let gossiper = EntityId::new();
        let listener = EntityId::new();
        let subject = EntityId::new();

        // Set gossiper as talkative
        rule.set_personality(
            gossiper,
            PersonalityTraits {
                gossip_tendency: 0.9,
                openness: 0.8,
                ..Default::default()
            },
        );

        // Set listener as very credulous and open
        rule.set_personality(
            listener,
            PersonalityTraits {
                credulity: 1.0,
                openness: 1.0,
                ..Default::default()
            },
        );

        // Give gossiper something to talk about — high trust, strong sentiment
        let mut social = SocialMemory::new(
            subject,
            gossiper,
            "Subject stole the mayor's pig",
            1.0,
            0,
            ts(1000),
        );
        social.sentiment = -0.8;
        rule.bank_mut(gossiper).social.push(social);

        propagate_gossip(&mut rule, gossiper, listener, ts(1001));

        // Listener should have received the gossip (credulous + high trust + very recent)
        let listener_social = rule
            .bank(listener)
            .map(|b| b.social.len())
            .unwrap_or(0);
        assert!(listener_social > 0, "Credulous listener should accept high-trust recent gossip");
    }
}
