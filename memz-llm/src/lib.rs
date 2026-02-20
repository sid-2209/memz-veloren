//! # memz-llm — LLM Abstraction Layer for MEMZ
//!
//! Provides a unified interface for LLM inference across multiple backends:
//!   - **Ollama** (local, recommended default)
//!   - **OpenAI-compatible API** (also works with Anthropic, Together, etc.)
//!   - **llama.cpp** (direct integration via FFI — future)
//!
//! All LLM calls in MEMZ go through this crate, ensuring:
//!   - Structured output enforcement (JSON mode / GBNF grammars)
//!   - Timeout management
//!   - Retry with fallback
//!   - Graceful degradation
//!
//! # Architecture
//!
//! ```text
//! Tier 0: Rule-Based (No LLM) — 90% of operations          [<0.1ms]
//! Tier 1: Small Local Model (1B–3B) — 9% of operations      [50–200ms, async]
//! Tier 2: Large Model (7B+ or cloud) — 1% of operations     [200ms–2s, async]
//! Tier 3: Structured Output — All LLM calls use JSON/GBNF
//! ```

pub mod client;
pub mod error;
pub mod prompt;
pub mod types;

pub use client::LlmClient;
pub use error::LlmError;
pub use types::{LlmRequest, LlmResponse, LlmTier};
