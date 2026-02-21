//! Memory type definitions and the `MemoryBank` aggregate.
//!
//! This module contains all 7 memory types from the MEMZ taxonomy (§8),
//! plus the [`MemoryBank`] that aggregates them per-character.

pub mod emotional;
pub mod episodic;
pub mod injected;
pub mod procedural;
pub mod reflective;
pub mod semantic;
pub mod social;

pub use emotional::EmotionalMemory;
pub use episodic::EpisodicMemory;
pub use injected::InjectedMemory;
pub use procedural::ProceduralMemory;
pub use reflective::ReflectiveMemory;
pub use semantic::SemanticMemory;
pub use social::SocialMemory;

use serde::{Deserialize, Serialize};

/// A unified memory entry that can hold any of the 7 memory types.
///
/// Used by the retrieval engine to score and rank memories of different types
/// through a single interface.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryEntry {
    /// An episodic (event) memory.
    Episodic(EpisodicMemory),
    /// A semantic (fact/belief) memory.
    Semantic(SemanticMemory),
    /// An emotional association memory.
    Emotional(EmotionalMemory),
    /// A social (gossip/hearsay) memory.
    Social(SocialMemory),
    /// A reflective (insight) memory.
    Reflective(ReflectiveMemory),
    /// A procedural (skill) memory.
    Procedural(ProceduralMemory),
    /// An injected (backstory) memory.
    Injected(InjectedMemory),
}

/// The memory bank — per-character aggregate of all memory types.
///
/// Every NPC, player, and creature gets one `MemoryBank` that holds
/// all of their memories, subject to configurable capacity limits.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryBank {
    /// Episodic memories — "What happened."
    pub episodic: Vec<EpisodicMemory>,
    /// Semantic memories — "What I know."
    pub semantic: Vec<SemanticMemory>,
    /// Emotional memories — "How I feel."
    pub emotional: Vec<EmotionalMemory>,
    /// Social memories — "What I've heard."
    pub social: Vec<SocialMemory>,
    /// Reflective memories — "What I think."
    pub reflective: Vec<ReflectiveMemory>,
    /// Procedural memories — "What I know how to do."
    pub procedural: Vec<ProceduralMemory>,
    /// Injected memories — "My backstory."
    pub injected: Vec<InjectedMemory>,
}

impl MemoryBank {
    /// Create a new empty memory bank.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Total number of memories across all types.
    #[must_use]
    pub fn total_count(&self) -> usize {
        self.episodic.len()
            + self.semantic.len()
            + self.emotional.len()
            + self.social.len()
            + self.reflective.len()
            + self.procedural.len()
            + self.injected.len()
    }

    /// Collect all memories into a flat list of [`MemoryEntry`] for retrieval.
    #[must_use]
    pub fn all_entries(&self) -> Vec<MemoryEntry> {
        let mut entries = Vec::with_capacity(self.total_count());
        entries.extend(self.episodic.iter().cloned().map(MemoryEntry::Episodic));
        entries.extend(self.semantic.iter().cloned().map(MemoryEntry::Semantic));
        entries.extend(self.emotional.iter().cloned().map(MemoryEntry::Emotional));
        entries.extend(self.social.iter().cloned().map(MemoryEntry::Social));
        entries.extend(self.reflective.iter().cloned().map(MemoryEntry::Reflective));
        entries.extend(self.procedural.iter().cloned().map(MemoryEntry::Procedural));
        entries.extend(self.injected.iter().cloned().map(MemoryEntry::Injected));
        entries
    }
}
