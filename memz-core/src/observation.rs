//! Event Observation Pipeline — game events → memory creation (§12.2)
//!
//! The observation system is the entry point for all memory creation.
//! Game events flow through an observation pipeline that:
//!
//! 1. Classifies the event type and computes emotional/importance scores
//! 2. Creates the appropriate memory type(s) for each observer
//! 3. Detects first-meeting events (protected from eviction)
//! 4. Optionally triggers secondary effects (gossip, reflection)
//!
//! Performance target: < 0.1ms per event (§12.6)

use crate::memory::episodic::EpisodicMemory;
use crate::memory::emotional::EmotionalMemory;
use crate::memory::social::SocialMemory;
use crate::memory::MemoryBank;
use crate::types::{EntityId, GameTimestamp, Location, PADState};

/// A game event observed by the memory system.
///
/// This is game-engine agnostic — the Veloren integration layer
/// converts Veloren-specific events into this format.
#[derive(Debug, Clone)]
pub struct ObservedEvent {
    /// What kind of event is this?
    pub kind: EventKind,
    /// Natural language description for episodic memory.
    pub description: String,
    /// Primary entities involved (actor, target).
    pub participants: Vec<EntityId>,
    /// All entities who witnessed this event (includes participants).
    pub witnesses: Vec<EntityId>,
    /// Where the event occurred.
    pub location: Location,
    /// When the event occurred.
    pub timestamp: GameTimestamp,
    /// Emotional valence (-1.0 to +1.0).
    pub emotional_valence: f32,
    /// Importance score (0.0 to 1.0).
    pub importance: f32,
    /// Optional PAD emotional state shift caused by this event.
    pub pad_shift: Option<PADState>,
}

/// Classification of game events for memory creation routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    /// First encounter with another entity.
    FirstMeeting,
    /// Dialogue / conversation.
    Dialogue,
    /// Combat (attack, defense, death).
    Combat,
    /// Trade / economic exchange.
    Trade,
    /// Helpful action (rescue, gift, defense of another).
    Help,
    /// Harmful action (theft, attack, betrayal).
    Harm,
    /// Arrival at a new location.
    Arrival,
    /// Quest-related event.
    Quest,
    /// Death of an entity.
    Death,
    /// Witnessing an event (secondary observer).
    Witness,
    /// Generic / custom event.
    Custom,
}

/// Result of processing an observed event for a single entity.
#[derive(Debug)]
pub struct ObservationResult {
    /// Number of episodic memories created.
    pub episodic_created: u32,
    /// Number of emotional memories created or updated.
    pub emotional_created: u32,
    /// Whether this was a first-meeting event.
    pub is_first_meeting: bool,
    /// Whether reflection should be triggered (high emotional intensity).
    pub should_trigger_reflection: bool,
    /// Whether gossip propagation should be triggered.
    pub should_trigger_gossip: bool,
}

/// Process an observed event and create memories for the observer.
///
/// This is the core observation pipeline entry point.
///
/// # Performance
/// Target: < 0.1ms per call (§12.6)
pub fn observe(
    event: &ObservedEvent,
    observer: EntityId,
    bank: &mut MemoryBank,
    known_entities: &[EntityId],
) -> ObservationResult {
    let mut result = ObservationResult {
        episodic_created: 0,
        emotional_created: 0,
        is_first_meeting: false,
        should_trigger_reflection: false,
        should_trigger_gossip: false,
    };

    // --- 1. Check for first meeting ---
    let is_first_meeting = event.kind == EventKind::FirstMeeting
        || detect_first_meeting(&event.participants, observer, known_entities);

    result.is_first_meeting = is_first_meeting;

    // --- 2. Create episodic memory ---
    let mut episodic = EpisodicMemory::new(
        &event.description,
        event.participants.clone(),
        event.location,
        event.timestamp,
        event.emotional_valence,
        if is_first_meeting {
            // First meetings are always important
            event.importance.max(0.7)
        } else {
            event.importance
        },
    );

    if is_first_meeting {
        episodic = episodic.with_first_meeting();
    }

    bank.episodic.push(episodic);
    result.episodic_created = 1;

    // --- 3. Create emotional memory for high-valence events ---
    if event.emotional_valence.abs() > 0.4 {
        if let Some(target) = primary_target(&event.participants, observer) {
            let emotion = classify_emotion(event.emotional_valence, &event.kind);
            let emotional = EmotionalMemory::new(
                target,
                emotion,
                event.emotional_valence.abs(),
                event.pad_shift.unwrap_or_default(),
                vec![], // basis memory IDs — populated during consolidation
                event.timestamp,
            );
            bank.emotional.push(emotional);
            result.emotional_created = 1;
        }
    }

    // --- 4. Determine secondary triggers ---
    result.should_trigger_reflection = event.emotional_valence.abs() > 0.7
        || event.importance > 0.8;

    result.should_trigger_gossip = event.importance > 0.5
        && matches!(
            event.kind,
            EventKind::Combat
                | EventKind::Help
                | EventKind::Harm
                | EventKind::Death
                | EventKind::Quest
        );

    result
}

/// Process a witnessed event — creates a less-detailed memory.
///
/// Witnesses remember events with reduced emotional intensity
/// and importance compared to direct participants.
pub fn observe_as_witness(
    event: &ObservedEvent,
    witness: EntityId,
    bank: &mut MemoryBank,
    known_entities: &[EntityId],
) -> ObservationResult {
    let mut witness_event = event.clone();
    witness_event.emotional_valence *= 0.6; // Reduced emotional impact
    witness_event.importance *= 0.7; // Less personally important
    witness_event.description = format!("Witnessed: {}", event.description);

    observe(&witness_event, witness, bank, known_entities)
}

/// Detect if any participant is being met for the first time.
fn detect_first_meeting(
    participants: &[EntityId],
    observer: EntityId,
    known_entities: &[EntityId],
) -> bool {
    participants
        .iter()
        .any(|p| *p != observer && !known_entities.contains(p))
}

/// Extract the primary target of an event (the "other" entity from the observer's perspective).
fn primary_target(participants: &[EntityId], observer: EntityId) -> Option<EntityId> {
    participants.iter().find(|p| **p != observer).copied()
}

/// Classify an emotion label from valence and event kind.
fn classify_emotion(valence: f32, kind: &EventKind) -> String {
    match kind {
        EventKind::Combat => {
            if valence > 0.0 {
                "pride".to_string()
            } else {
                "fear".to_string()
            }
        }
        EventKind::Help => "gratitude".to_string(),
        EventKind::Harm => "anger".to_string(),
        EventKind::Death => "grief".to_string(),
        EventKind::Trade => {
            if valence > 0.0 {
                "satisfaction".to_string()
            } else {
                "resentment".to_string()
            }
        }
        EventKind::FirstMeeting => "curiosity".to_string(),
        _ => {
            if valence > 0.3 {
                "joy".to_string()
            } else if valence < -0.3 {
                "sadness".to_string()
            } else {
                "surprise".to_string()
            }
        }
    }
}

/// Create a social memory from a gossip event.
///
/// When one NPC tells another about something, the listener gets
/// a social memory recording the claim.
pub fn observe_gossip(
    about: EntityId,
    source: EntityId,
    claim: &str,
    trust_in_source: f32,
    propagation_depth: u32,
    timestamp: GameTimestamp,
    bank: &mut MemoryBank,
) {
    let social = SocialMemory::new(
        about,
        source,
        claim.to_string(),
        trust_in_source,
        propagation_depth,
        timestamp,
    );
    bank.social.push(social);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(kind: EventKind, valence: f32, importance: f32) -> ObservedEvent {
        ObservedEvent {
            kind,
            description: "Test event".to_string(),
            participants: vec![EntityId::new(), EntityId::new()],
            witnesses: vec![],
            location: Location::default(),
            timestamp: GameTimestamp::now(36_000),
            emotional_valence: valence,
            importance,
            pad_shift: None,
        }
    }

    #[test]
    fn observe_creates_episodic_memory() {
        let event = make_event(EventKind::Dialogue, 0.2, 0.3);
        let observer = event.participants[0];
        let mut bank = MemoryBank::new();

        let result = observe(&event, observer, &mut bank, &[]);
        assert_eq!(result.episodic_created, 1);
        assert_eq!(bank.episodic.len(), 1);
    }

    #[test]
    fn high_valence_creates_emotional_memory() {
        let event = make_event(EventKind::Help, 0.8, 0.6);
        let observer = event.participants[0];
        let mut bank = MemoryBank::new();

        let result = observe(&event, observer, &mut bank, &[]);
        assert_eq!(result.emotional_created, 1);
        assert_eq!(bank.emotional.len(), 1);
        assert_eq!(bank.emotional[0].emotion, "gratitude");
    }

    #[test]
    fn first_meeting_detection() {
        let event = make_event(EventKind::Dialogue, 0.1, 0.3);
        let observer = event.participants[0];
        let known = vec![]; // observer hasn't met anyone

        let mut bank = MemoryBank::new();
        let result = observe(&event, observer, &mut bank, &known);
        assert!(result.is_first_meeting);
        assert!(bank.episodic[0].is_first_meeting);
        // First meetings get boosted importance
        assert!(bank.episodic[0].importance >= 0.7);
    }

    #[test]
    fn known_entity_not_first_meeting() {
        let event = make_event(EventKind::Dialogue, 0.1, 0.3);
        let observer = event.participants[0];
        let other = event.participants[1];
        let known = vec![other]; // already met

        let mut bank = MemoryBank::new();
        let result = observe(&event, observer, &mut bank, &known);
        assert!(!result.is_first_meeting);
    }

    #[test]
    fn witness_gets_reduced_impact() {
        let event = make_event(EventKind::Combat, -0.8, 0.9);
        let witness = EntityId::new();
        let mut bank = MemoryBank::new();

        let result = observe_as_witness(&event, witness, &mut bank, &[]);
        assert_eq!(result.episodic_created, 1);
        assert!(bank.episodic[0].event.starts_with("Witnessed:"));
        // Witness gets reduced importance
        assert!(bank.episodic[0].importance < 0.9);
    }

    #[test]
    fn combat_triggers_gossip() {
        let event = make_event(EventKind::Combat, -0.6, 0.7);
        let observer = event.participants[0];
        let mut bank = MemoryBank::new();

        let result = observe(&event, observer, &mut bank, &[]);
        assert!(result.should_trigger_gossip);
    }

    #[test]
    fn high_emotion_triggers_reflection() {
        let event = make_event(EventKind::Help, 0.9, 0.5);
        let observer = event.participants[0];
        let mut bank = MemoryBank::new();

        let result = observe(&event, observer, &mut bank, &[]);
        assert!(result.should_trigger_reflection);
    }

    #[test]
    fn gossip_creates_social_memory() {
        let mut bank = MemoryBank::new();
        let about = EntityId::new();
        let source = EntityId::new();

        observe_gossip(
            about,
            source,
            "They say that player helped fight bandits",
            0.7,
            1,
            GameTimestamp::now(36_000),
            &mut bank,
        );

        assert_eq!(bank.social.len(), 1);
        assert_eq!(bank.social[0].propagation_depth, 1);
    }
}
