//! Integration hooks for Veloren's existing systems.
//!
//! These hooks bridge Veloren's game events (combat, trade, dialogue, movement)
//! to the MEMZ memory system. In a full Veloren integration, these would
//! be injected into Veloren's existing ECS systems.

use memz_core::types::{EntityId, GameTimestamp, Location};

use crate::events::{CombatOutcome, GameEvent};

/// Create a dialogue event from Veloren's chat system.
#[must_use]
pub fn on_dialogue(
    speaker: EntityId,
    listener: EntityId,
    content: String,
    location: Location,
    timestamp: GameTimestamp,
) -> GameEvent {
    GameEvent::Dialogue {
        speaker,
        listener,
        content,
        location,
        timestamp,
    }
}

/// Create a combat event from Veloren's combat system.
#[must_use]
pub fn on_combat(
    attacker: EntityId,
    defender: EntityId,
    outcome: CombatOutcome,
    witnesses: Vec<EntityId>,
    location: Location,
    timestamp: GameTimestamp,
) -> GameEvent {
    GameEvent::Combat {
        attacker,
        defender,
        outcome,
        witnesses,
        location,
        timestamp,
    }
}

/// Create a trade event from Veloren's trading system.
#[must_use]
pub fn on_trade(
    buyer: EntityId,
    seller: EntityId,
    item: String,
    perceived_fairness: f32,
    location: Location,
    timestamp: GameTimestamp,
) -> GameEvent {
    GameEvent::Trade {
        buyer,
        seller,
        item,
        perceived_fairness,
        location,
        timestamp,
    }
}

/// Create a help event (e.g., player defended an NPC).
#[must_use]
pub fn on_help(
    helper: EntityId,
    helped: EntityId,
    action: String,
    witnesses: Vec<EntityId>,
    location: Location,
    timestamp: GameTimestamp,
) -> GameEvent {
    GameEvent::Helped {
        helper,
        helped,
        action,
        witnesses,
        location,
        timestamp,
    }
}

/// Create a harm event (e.g., theft, property destruction).
#[must_use]
pub fn on_harm(
    perpetrator: EntityId,
    victim: EntityId,
    action: String,
    witnesses: Vec<EntityId>,
    location: Location,
    timestamp: GameTimestamp,
) -> GameEvent {
    GameEvent::Harmed {
        perpetrator,
        victim,
        action,
        witnesses,
        location,
        timestamp,
    }
}

/// Create a death event.
#[must_use]
pub fn on_death(
    entity: EntityId,
    cause: String,
    witnesses: Vec<EntityId>,
    location: Location,
    timestamp: GameTimestamp,
) -> GameEvent {
    GameEvent::Death {
        entity,
        cause,
        witnesses,
        location,
        timestamp,
    }
}
