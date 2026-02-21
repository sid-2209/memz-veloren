//! ECS Systems for the MEMZ memory layer (§12.2).
//!
//! In a full Veloren integration, these would implement `specs::System`.
//! For now, they are standalone functions that operate on memory components.
//!
//! ## System Budget (per frame):
//! 
//! | System              | Budget   | Frequency        |
//! |---------------------|----------|------------------|
//! | Observation         | 0.1ms    | Every frame      |
//! | Decay               | 0.05ms   | Every 60 frames  |
//! | Reflection          | Async    | Every 5 min      |
//! | Social Propagation  | 0.3ms    | On NPC interact  |
//! | Behavior Mod        | 0.2ms    | On interaction   |
//! | Compact/Evict       | 0.1ms    | On memory add    |

use memz_core::config::MemoryConfig;
use memz_core::decay;
use memz_core::memory::MemoryBank;
use memz_core::memory::episodic::EpisodicMemory;
use memz_core::types::GameTimestamp;

use crate::events::GameEvent;

/// Process a game event and create appropriate memories for all observing entities.
///
/// Performance target: < 0.1ms per event (§12.6).
pub fn observe_event(
    event: &GameEvent,
    observer_bank: &mut MemoryBank,
) {
    let description = event_to_description(event);
    let participants = event.all_entities();
    let location = *event.location();
    let timestamp = *event.timestamp();
    let valence = event.emotional_valence();
    let importance = event.importance();

    let episodic = EpisodicMemory::new(
        description,
        participants,
        location,
        timestamp,
        valence,
        importance,
    );

    observer_bank.episodic.push(episodic);
}

/// Run the memory decay pass for a single NPC.
///
/// Performance target: < 0.05ms per NPC (§12.6).
pub fn run_decay(
    bank: &mut MemoryBank,
    current_time: &GameTimestamp,
    config: &MemoryConfig,
) {
    decay::decay_episodic_memories(&mut bank.episodic, current_time, config);
    decay::decay_social_memories(&mut bank.social, current_time, f64::from(config.decay_rate));
}

/// Enforce memory limits by evicting low-priority memories.
///
/// Performance target: < 0.1ms per NPC (§12.6).
pub fn enforce_limits(bank: &mut MemoryBank, config: &MemoryConfig) {
    // Simple truncation — more sophisticated eviction (hot/warm/cold rings)
    // will be implemented in memz-core MemoryBank.
    bank.episodic.truncate(config.max_episodic_per_npc);
    bank.semantic.truncate(config.max_semantic_per_npc);
    bank.social.truncate(config.max_social_per_npc);
    bank.procedural.truncate(config.max_procedural_per_npc);
    bank.reflective.truncate(config.max_reflective_per_npc);
}

/// Generate a human-readable description from a game event.
fn event_to_description(event: &GameEvent) -> String {
    match event {
        GameEvent::Dialogue {
            speaker,
            content,
            ..
        } => format!("Entity {speaker:?} said: \"{content}\""),

        GameEvent::Combat {
            attacker,
            defender,
            outcome,
            ..
        } => {
            let outcome_str = match outcome {
                crate::events::CombatOutcome::AttackerWon => "and won",
                crate::events::CombatOutcome::DefenderWon => "but lost",
                crate::events::CombatOutcome::Draw => "and it ended in a draw",
                crate::events::CombatOutcome::Fled { .. } => "but someone fled",
                crate::events::CombatOutcome::Interrupted => "but it was interrupted",
            };
            format!(
                "Entity {attacker:?} attacked entity {defender:?} {outcome_str}"
            )
        }

        GameEvent::Trade {
            buyer, seller, item, ..
        } => format!(
            "Entity {buyer:?} bought {item} from entity {seller:?}"
        ),

        GameEvent::Helped {
            helper,
            helped,
            action,
            ..
        } => format!(
            "Entity {helper:?} helped entity {helped:?}: {action}"
        ),

        GameEvent::Harmed {
            perpetrator,
            victim,
            action,
            ..
        } => format!(
            "Entity {perpetrator:?} harmed entity {victim:?}: {action}"
        ),

        GameEvent::Arrival { entity, location, .. } => {
            format!(
                "Entity {entity:?} arrived at {location}"
            )
        }

        GameEvent::QuestEvent {
            entity,
            quest_name,
            event_type,
            ..
        } => format!(
            "Entity {entity:?} triggered quest event '{event_type}' in quest '{quest_name}'"
        ),

        GameEvent::Death {
            entity, cause, ..
        } => format!("Entity {entity:?} died: {cause}"),

        GameEvent::Custom { description, .. } => description.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use memz_core::types::{EntityId, GameTimestamp, Location};

    #[test]
    fn observe_creates_episodic_memory() {
        let mut bank = MemoryBank::new();
        let event = GameEvent::Helped {
            helper: EntityId::new(),
            helped: EntityId::new(),
            action: "defended from wolves".to_string(),
            witnesses: vec![EntityId::new()],
            location: Location::default(),
            timestamp: GameTimestamp::now(36_000),
        };

        observe_event(&event, &mut bank);
        assert_eq!(bank.episodic.len(), 1);
        assert!(bank.episodic[0].event.contains("defended from wolves"));
    }

    #[test]
    fn event_descriptions_are_reasonable() {
        let event = GameEvent::Trade {
            buyer: EntityId::new(),
            seller: EntityId::new(),
            item: "iron sword".to_string(),
            perceived_fairness: 0.5,
            location: Location::default(),
            timestamp: GameTimestamp::now(36_000),
        };

        let desc = event_to_description(&event);
        assert!(desc.contains("iron sword"));
    }
}
