//! Error types for the MEMZ core library.

use thiserror::Error;

/// Top-level error type for all MEMZ operations.
#[derive(Error, Debug)]
pub enum MemzError {
    /// Memory bank has reached its configured capacity for this memory type.
    #[error("Memory capacity exceeded: {memory_type} (limit: {limit}, current: {current})")]
    CapacityExceeded {
        /// Which memory type hit the limit.
        memory_type: String,
        /// Maximum allowed.
        limit: usize,
        /// Current count.
        current: usize,
    },

    /// A memory with the given ID was not found.
    #[error("Memory not found: {0}")]
    MemoryNotFound(crate::MemoryId),

    /// Entity not found in the system.
    #[error("Entity not found: {0:?}")]
    EntityNotFound(crate::EntityId),

    /// Serialization or deserialization failure.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// SQLite persistence error.
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// Configuration error.
    #[error("Configuration error: {0}")]
    Config(String),

    /// An operation would violate the frame budget.
    #[error("Frame budget exceeded: {operation} took {elapsed_us}μs (budget: {budget_us}μs)")]
    BudgetExceeded {
        /// Which operation exceeded the budget.
        operation: String,
        /// Microseconds elapsed.
        elapsed_us: u64,
        /// Microseconds budgeted.
        budget_us: u64,
    },

    /// Content safety validation rejected the input.
    #[error("Content rejected: {reason}")]
    ContentRejected {
        /// Why the content was rejected.
        reason: String,
    },

    /// Generic I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Convenience Result type alias.
pub type Result<T> = std::result::Result<T, MemzError>;
