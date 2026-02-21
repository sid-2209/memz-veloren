//! Game events that trigger memory creation (§12.2).
//!
//! These events are observed by the `MemorySystem` and converted
//! into appropriate memory types.

use memz_core::types::{EntityId, GameTimestamp, Location};

/// A game event that can trigger memory creation.
#[derive(Debug, Clone)]
pub enum GameEvent {
    /// Player or NPC spoke to another entity.
    Dialogue {
        speaker: EntityId,
        listener: EntityId,
        content: String,
        location: Location,
        timestamp: GameTimestamp,
    },

    /// Combat occurred between entities.
    Combat {
        attacker: EntityId,
        defender: EntityId,
        outcome: CombatOutcome,
        witnesses: Vec<EntityId>,
        location: Location,
        timestamp: GameTimestamp,
    },

    /// A trade occurred between entities.
    Trade {
        buyer: EntityId,
        seller: EntityId,
        item: String,
        perceived_fairness: f32, // -1.0 (rip-off) to 1.0 (generous)
        location: Location,
        timestamp: GameTimestamp,
    },

    /// An entity helped another entity.
    Helped {
        helper: EntityId,
        helped: EntityId,
        action: String,
        witnesses: Vec<EntityId>,
        location: Location,
        timestamp: GameTimestamp,
    },

    /// An entity harmed another entity (theft, destruction, etc.).
    Harmed {
        perpetrator: EntityId,
        victim: EntityId,
        action: String,
        witnesses: Vec<EntityId>,
        location: Location,
        timestamp: GameTimestamp,
    },

    /// An entity arrived at a new location.
    Arrival {
        entity: EntityId,
        location: Location,
        observers: Vec<EntityId>,
        timestamp: GameTimestamp,
    },

    /// A quest-related event occurred.
    QuestEvent {
        entity: EntityId,
        quest_name: String,
        event_type: String,
        witnesses: Vec<EntityId>,
        location: Location,
        timestamp: GameTimestamp,
    },

    /// An entity died.
    Death {
        entity: EntityId,
        cause: String,
        witnesses: Vec<EntityId>,
        location: Location,
        timestamp: GameTimestamp,
    },

    /// A custom/generic event.
    Custom {
        description: String,
        participants: Vec<EntityId>,
        emotional_valence: f32,
        importance: f32,
        location: Location,
        timestamp: GameTimestamp,
    },
}

/// Outcome of a combat event.
#[derive(Debug, Clone)]
pub enum CombatOutcome {
    AttackerWon,
    DefenderWon,
    Draw,
    Fled { who: EntityId },
    Interrupted,
}

impl GameEvent {
    /// Get the timestamp of this event.
    #[must_use]
    pub fn timestamp(&self) -> &GameTimestamp {
        match self {
            Self::Dialogue { timestamp, .. }
            | Self::Combat { timestamp, .. }
            | Self::Trade { timestamp, .. }
            | Self::Helped { timestamp, .. }
            | Self::Harmed { timestamp, .. }
            | Self::Arrival { timestamp, .. }
            | Self::QuestEvent { timestamp, .. }
            | Self::Death { timestamp, .. }
            | Self::Custom { timestamp, .. } => timestamp,
        }
    }

    /// Get the location of this event.
    #[must_use]
    pub fn location(&self) -> &Location {
        match self {
            Self::Dialogue { location, .. }
            | Self::Combat { location, .. }
            | Self::Trade { location, .. }
            | Self::Helped { location, .. }
            | Self::Harmed { location, .. }
            | Self::Arrival { location, .. }
            | Self::QuestEvent { location, .. }
            | Self::Death { location, .. }
            | Self::Custom { location, .. } => location,
        }
    }

    /// Get all participants/witnesses of this event.
    #[must_use]
    pub fn all_entities(&self) -> Vec<EntityId> {
        match self {
            Self::Dialogue {
                speaker, listener, ..
            } => vec![*speaker, *listener],
            Self::Combat {
                attacker,
                defender,
                witnesses,
                ..
            } => {
                let mut entities = vec![*attacker, *defender];
                entities.extend(witnesses);
                entities
            }
            Self::Trade {
                buyer, seller, ..
            } => vec![*buyer, *seller],
            Self::Helped {
                helper,
                helped,
                witnesses,
                ..
            } => {
                let mut entities = vec![*helper, *helped];
                entities.extend(witnesses);
                entities
            }
            Self::Harmed {
                perpetrator,
                victim,
                witnesses,
                ..
            } => {
                let mut entities = vec![*perpetrator, *victim];
                entities.extend(witnesses);
                entities
            }
            Self::Arrival {
                entity, observers, ..
            } => {
                let mut entities = vec![*entity];
                entities.extend(observers);
                entities
            }
            Self::QuestEvent {
                entity, witnesses, ..
            } => {
                let mut entities = vec![*entity];
                entities.extend(witnesses);
                entities
            }
            Self::Death {
                entity, witnesses, ..
            } => {
                let mut entities = vec![*entity];
                entities.extend(witnesses);
                entities
            }
            Self::Custom { participants, .. } => participants.clone(),
        }
    }

    /// Compute the emotional valence of this event.
    #[must_use]
    pub fn emotional_valence(&self) -> f32 {
        match self {
            Self::Dialogue { .. } => 0.1,                       // neutral-positive
            Self::Combat { .. } => -0.3,                        // slightly negative
            Self::Trade { perceived_fairness, .. } => *perceived_fairness * 0.5,
            Self::Helped { .. } => 0.7,                         // strongly positive
            Self::Harmed { .. } => -0.7,                        // strongly negative
            Self::Arrival { .. } => 0.1,                        // neutral
            Self::QuestEvent { .. } => 0.5,                     // positive
            Self::Death { .. } => -0.9,                         // very negative
            Self::Custom { emotional_valence, .. } => *emotional_valence,
        }
    }

    /// Compute the importance of this event (0.0–1.0).
    #[must_use]
    pub fn importance(&self) -> f32 {
        match self {
            Self::Dialogue { .. } => 0.3,
            Self::Combat { .. } => 0.7,
            Self::Trade { .. } => 0.4,
            Self::Helped { .. } => 0.6,
            Self::Harmed { .. } => 0.7,
            Self::Arrival { .. } => 0.2,
            Self::QuestEvent { .. } => 0.8,
            Self::Death { .. } => 0.9,
            Self::Custom { importance, .. } => *importance,
        }
    }
}
