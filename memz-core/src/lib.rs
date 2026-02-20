//! # MEMZ Core Library
//!
//! Game-agnostic persistent memory layer for game characters.
//!
//! Every character (NPC, player, creature) gets a [`MemoryBank`] containing
//! structured memories grounded in cognitive science:
//!
//! - **Episodic** — "What happened" (Tulving, 1972)
//! - **Semantic** — "What I know" (Tulving, 1985)
//! - **Emotional** — "How I feel" (Russell & Mehrabian PAD model, 1977)
//! - **Social** — "What I've heard" (Dunbar social brain hypothesis, 1996)
//! - **Reflective** — "What I think" (Flavell metacognition, 1979)
//! - **Procedural** — "What I know how to do" (Anderson ACT-R, 1993)
//! - **Injected** — "My backstory" (player-authored memories)
//!
//! ## Performance Contract
//!
//! All operations in this crate are designed for real-time game use:
//! - Memory creation: < 10μs
//! - Memory retrieval (top-5): < 500μs
//! - Memory decay pass (50 NPCs): < 50μs
//! - Serialization (100 memories): < 2ms

#![deny(clippy::unwrap_used)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod config;
pub mod consolidation;
pub mod decay;
pub mod error;
pub mod memory;
pub mod reflection;
pub mod retrieval;
pub mod safety;
pub mod social;
pub mod types;

pub use config::MemoryConfig;
pub use error::MemzError;
pub use memory::{MemoryBank, MemoryEntry};
pub use types::*;
