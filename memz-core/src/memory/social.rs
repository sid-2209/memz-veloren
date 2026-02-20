//! Social Memory — "What I've heard" (§8.4)
//!
//! Information received from other NPCs — the gossip/rumor system.
//! NPCs don't automatically believe what they hear; they weigh it
//! against trust, personal experience, and consistency.
//!
//! Grounded in Dunbar's social brain hypothesis (1996).

use serde::{Deserialize, Serialize};

use crate::types::{EntityId, GameTimestamp, MemoryId};

/// A piece of information received from another character.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialMemory {
    /// Unique identifier.
    pub id: MemoryId,
    /// Who or what the claim is about.
    pub about: EntityId,
    /// Who told the character this information.
    pub source: EntityId,
    /// The claim in natural language.
    pub claim: String,
    /// Whether the character currently believes this claim.
    pub believed: bool,
    /// If not believed, the reason (natural language).
    pub disbelief_reason: Option<String>,
    /// Trust in the source at the time of transmission (0.0 to 1.0).
    pub trust_in_source: f32,
    /// How many NPCs the information passed through before reaching this one.
    /// 0 = first-hand (direct witness), 1 = second-hand, 2+ = rumor.
    pub propagation_depth: u32,
    /// When this information was received.
    pub received_at: GameTimestamp,
    /// Emotional valence of the claim (positive or negative about the subject).
    pub sentiment: f32,
}

impl SocialMemory {
    /// Create a new social memory from a gossip transmission.
    #[must_use]
    pub fn new(
        about: EntityId,
        source: EntityId,
        claim: impl Into<String>,
        trust_in_source: f32,
        propagation_depth: u32,
        timestamp: GameTimestamp,
    ) -> Self {
        Self {
            id: MemoryId::new(),
            about,
            source,
            claim: claim.into(),
            believed: trust_in_source > 0.5, // naive default; conflict resolution may override
            disbelief_reason: None,
            trust_in_source: trust_in_source.clamp(0.0, 1.0),
            propagation_depth,
            received_at: timestamp,
            sentiment: 0.0,
        }
    }

    /// Mark this claim as believed after conflict resolution.
    pub fn accept(&mut self) {
        self.believed = true;
        self.disbelief_reason = None;
    }

    /// Mark this claim as disbelieved with a reason.
    pub fn reject(&mut self, reason: impl Into<String>) {
        self.believed = false;
        self.disbelief_reason = Some(reason.into());
    }

    /// The information quality degrades with each hop (telephone game effect).
    /// Returns a degradation factor in [0.0, 1.0] — 1.0 = pristine, 0.0 = unreliable.
    #[must_use]
    pub fn chain_reliability(&self) -> f32 {
        1.0 / (1.0 + self.propagation_depth as f32)
    }
}
