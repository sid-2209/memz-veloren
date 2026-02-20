//! ECS Components for the MEMZ memory system (§12.2).
//!
//! These components are attached to entities (NPCs, players, creatures)
//! to give them memory capability within Veloren's ECS.

use memz_core::memory::MemoryBank;
use memz_core::types::MemoryStats;

/// The primary memory component. Wraps `memz_core::MemoryBank` for
/// integration with Veloren's `specs` ECS.
///
/// In a full Veloren integration, this would derive `specs::Component`.
/// For now, it's a standalone wrapper.
#[derive(Debug)]
pub struct MemoryComponent {
    /// The underlying memory bank.
    pub bank: MemoryBank,
    /// Whether this entity has active memory processing.
    pub active: bool,
    /// Whether this entity is within the player's perception radius.
    pub in_loaded_chunk: bool,
}

impl MemoryComponent {
    /// Create a new memory component with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self {
            bank: MemoryBank::new(),
            active: true,
            in_loaded_chunk: false,
        }
    }

    /// Create a memory component with a pre-configured memory bank.
    #[must_use]
    pub fn with_bank(bank: MemoryBank) -> Self {
        Self {
            bank,
            active: true,
            in_loaded_chunk: false,
        }
    }

    /// Get runtime statistics.
    #[must_use]
    pub fn stats(&self) -> MemoryStats {
        MemoryStats {
            last_retrieval_us: 0,
            total_memories: self.bank.total_count() as u32,
            pending_reflections: 0,
            last_decay_tick: 0,
        }
    }
}

impl Default for MemoryComponent {
    fn default() -> Self {
        Self::new()
    }
}

/// Marker component for entities that are memory-enabled NPCs.
#[derive(Debug, Default)]
pub struct MemoryEnabled;

/// Marker component for player characters (enables memory injection).
#[derive(Debug, Default)]
pub struct PlayerMemory;
