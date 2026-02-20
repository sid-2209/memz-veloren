//! # memz-veloren — Veloren Integration for MEMZ
//!
//! This crate provides the integration layer between the game-agnostic
//! `memz-core` library and Veloren's ECS (Entity Component System).
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │              Veloren ECS                 │
//! │  ┌───────────────────────────────────┐  │
//! │  │       memz-veloren                │  │
//! │  │  ┌─────────────┐ ┌─────────────┐ │  │
//! │  │  │ Components  │ │   Systems   │ │  │
//! │  │  └──────┬──────┘ └──────┬──────┘ │  │
//! │  │         │               │         │  │
//! │  │         ▼               ▼         │  │
//! │  │    ┌─────────────────────────┐    │  │
//! │  │    │      memz-core          │    │  │
//! │  │    └─────────────────────────┘    │  │
//! │  │    ┌─────────────────────────┐    │  │
//! │  │    │      memz-llm           │    │  │
//! │  │    └─────────────────────────┘    │  │
//! │  └───────────────────────────────────┘  │
//! └─────────────────────────────────────────┘
//! ```
//!
//! ## Modules
//!
//! - `components` — ECS components (MemoryBank wrapper, MemoryConfig, MemoryStats)
//! - `systems` — ECS systems (observation, decay, reflection, propagation)
//! - `events` — Game event types that trigger memory creation
//! - `hooks` — Integration points with Veloren's existing systems

pub mod components;
pub mod events;
pub mod hooks;
pub mod systems;
