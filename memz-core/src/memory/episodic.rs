//! Episodic Memory — "What happened" (§8.1)
//!
//! Raw stream of experiences. Every significant event a character witnesses
//! or participates in becomes an episodic memory.
//!
//! Grounded in Tulving's episodic memory theory (1972).

use serde::{Deserialize, Serialize};

use crate::types::{Embedding, EntityId, GameTimestamp, Location, MemoryId};

/// A single episodic memory — a recorded event from the character's perspective.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicMemory {
    /// Unique identifier for this memory.
    pub id: MemoryId,
    /// Natural language description of the event.
    pub event: String,
    /// Entities involved in the event.
    pub participants: Vec<EntityId>,
    /// Where the event occurred.
    pub location: Location,
    /// When the event occurred (game time).
    pub timestamp: GameTimestamp,
    /// How this event made the character feel (-1.0 to +1.0).
    pub emotional_valence: f32,
    /// How significant this event is (0.0 to 1.0).
    /// Boosted by novelty, emotional intensity, relevance to goals.
    pub importance: f32,
    /// How fast this memory fades (Ebbinghaus decay constant).
    pub decay_rate: f32,
    /// Current strength / retention (1.0 = perfect, 0.0 = forgotten).
    pub strength: f32,
    /// How many times this memory has been accessed/recalled.
    pub access_count: u32,
    /// Last time this memory was accessed.
    pub last_accessed: GameTimestamp,
    /// Whether this is a first-meeting memory (protected from eviction).
    pub is_first_meeting: bool,
    /// Vector embedding for semantic retrieval (lazily computed).
    #[serde(skip)]
    pub embedding: Option<Embedding>,
}

impl EpisodicMemory {
    /// Create a new episodic memory with default strength of 1.0.
    #[must_use]
    pub fn new(
        event: impl Into<String>,
        participants: Vec<EntityId>,
        location: Location,
        timestamp: GameTimestamp,
        emotional_valence: f32,
        importance: f32,
    ) -> Self {
        let ev = emotional_valence.clamp(-1.0, 1.0);
        let imp = importance.clamp(0.0, 1.0);
        // Base decay rate: less important & less emotional → fades faster.
        // Flashbulb memories (high |valence|) decay slower.
        let base_decay = 0.05;
        let decay_rate = base_decay * (1.0 - imp * 0.5) * (1.0 - ev.abs() * 0.3);

        Self {
            id: MemoryId::new(),
            event: event.into(),
            participants,
            location,
            timestamp,
            emotional_valence: ev,
            importance: imp,
            decay_rate,
            strength: 1.0,
            access_count: 0,
            last_accessed: timestamp,
            is_first_meeting: false,
            embedding: None,
        }
    }

    /// Mark this as a first-meeting memory (protected from eviction).
    #[must_use] 
    pub fn with_first_meeting(mut self) -> Self {
        self.is_first_meeting = true;
        self
    }

    /// Record an access (recall), boosting strength slightly (rehearsal effect).
    pub fn record_access(&mut self, now: GameTimestamp) {
        self.access_count += 1;
        self.last_accessed = now;
        // Rehearsal strengthens the memory (spacing effect).
        self.strength = (self.strength + 0.1).min(1.0);
    }
}
