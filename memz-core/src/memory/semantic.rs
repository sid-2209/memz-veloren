//! Semantic Memory — "What I know" (§8.2)
//!
//! Distilled facts and knowledge derived from episodic memories.
//! These are the NPC's beliefs about the world.
//!
//! Grounded in Tulving's semantic memory theory (1985).

use serde::{Deserialize, Serialize};

use crate::types::{Embedding, GameTimestamp, MemoryId};

/// A distilled fact or belief held by the character.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticMemory {
    /// Unique identifier.
    pub id: MemoryId,
    /// The fact or belief in natural language.
    pub fact: String,
    /// Confidence in this fact (0.0 to 1.0).
    pub confidence: f32,
    /// IDs of episodic memories that support this fact.
    pub derived_from: Vec<MemoryId>,
    /// Category for organization (e.g., "person_knowledge", "world_fact", "skill_knowledge").
    pub category: String,
    /// When this fact was last reinforced by new evidence.
    pub last_reinforced: GameTimestamp,
    /// When this fact was first formed.
    pub created_at: GameTimestamp,
    /// Vector embedding for semantic retrieval.
    #[serde(skip)]
    pub embedding: Option<Embedding>,
}

impl SemanticMemory {
    /// Create a new semantic memory.
    #[must_use]
    pub fn new(
        fact: impl Into<String>,
        confidence: f32,
        derived_from: Vec<MemoryId>,
        category: impl Into<String>,
        timestamp: GameTimestamp,
    ) -> Self {
        Self {
            id: MemoryId::new(),
            fact: fact.into(),
            confidence: confidence.clamp(0.0, 1.0),
            derived_from,
            category: category.into(),
            last_reinforced: timestamp,
            created_at: timestamp,
            embedding: None,
        }
    }

    /// Reinforce this fact with new evidence, boosting confidence.
    pub fn reinforce(&mut self, new_source: MemoryId, now: GameTimestamp) {
        self.derived_from.push(new_source);
        self.confidence = (self.confidence + 0.1).min(1.0);
        self.last_reinforced = now;
    }

    /// Weaken confidence when contradictory evidence arrives.
    pub fn contradict(&mut self, amount: f32) {
        self.confidence = (self.confidence - amount.abs()).max(0.0);
    }
}
