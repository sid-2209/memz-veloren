//! Core types for LLM requests and responses.

use serde::{Deserialize, Serialize};

/// LLM tier classification (§12.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LlmTier {
    /// Tier 0: No LLM needed (rule-based).
    RuleBased,
    /// Tier 1: Small local model (1B–3B params). Fast, cheap.
    SmallLocal,
    /// Tier 2: Large model (7B+ local or cloud API). Deep reasoning.
    LargeModel,
}

/// A request to the LLM.
#[derive(Debug, Clone, Serialize)]
pub struct LlmRequest {
    /// System prompt (character persona, rules, constraints).
    pub system: String,
    /// User prompt (context, memories, instructions).
    pub user: String,
    /// Which tier is this request?
    pub tier: LlmTier,
    /// Maximum tokens to generate.
    pub max_tokens: u32,
    /// Temperature (0.0 = deterministic, 1.0 = creative).
    pub temperature: f32,
    /// Optional GBNF grammar for structured output.
    pub grammar: Option<String>,
    /// Request timeout in milliseconds.
    pub timeout_ms: u64,
}

impl LlmRequest {
    /// Create a new Tier 1 request.
    #[must_use]
    pub fn tier1(system: impl Into<String>, user: impl Into<String>) -> Self {
        Self {
            system: system.into(),
            user: user.into(),
            tier: LlmTier::SmallLocal,
            max_tokens: 150,
            temperature: 0.7,
            grammar: None,
            timeout_ms: 5000,
        }
    }

    /// Create a new Tier 2 request.
    #[must_use]
    pub fn tier2(system: impl Into<String>, user: impl Into<String>) -> Self {
        Self {
            system: system.into(),
            user: user.into(),
            tier: LlmTier::LargeModel,
            max_tokens: 300,
            temperature: 0.8,
            grammar: None,
            timeout_ms: 5000,
        }
    }

    /// Set a GBNF grammar for structured output.
    #[must_use]
    pub fn with_grammar(mut self, grammar: impl Into<String>) -> Self {
        self.grammar = Some(grammar.into());
        self
    }

    /// Set the timeout.
    #[must_use]
    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }
}

/// A response from the LLM.
#[derive(Debug, Clone, Deserialize)]
pub struct LlmResponse {
    /// The generated text.
    pub text: String,
    /// How many tokens were generated.
    pub tokens_generated: u32,
    /// Latency in milliseconds.
    pub latency_ms: u64,
    /// Which model was used.
    pub model: String,
}

/// Structured dialogue response from an NPC (matches GBNF grammar).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueResponse {
    /// The NPC's dialogue line.
    pub dialogue: String,
    /// Emotional shift caused by this interaction (-1.0 to 1.0).
    pub emotion_shift: f32,
    /// What the NPC will remember about this interaction.
    pub new_memory: String,
}

/// Structured reflection response from an NPC (matches GBNF grammar).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionResponse {
    /// The NPC's internal reflection.
    pub reflection: String,
    /// New beliefs formed during reflection.
    pub new_beliefs: Vec<String>,
    /// Questions the NPC now wonders about.
    pub questions: Vec<String>,
    /// Mood shift from reflection.
    pub mood_shift: MoodShift,
}

/// PAD mood shift from an LLM response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoodShift {
    pub pleasure: f32,
    pub arousal: f32,
    pub dominance: f32,
}

/// Structured gossip response (matches GBNF grammar).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GossipResponse {
    /// What the NPC says about the subject.
    pub gossip_text: String,
    /// How confident the NPC is in this information.
    pub confidence: f32,
    /// Whether the NPC embellished or changed the story.
    pub embellished: bool,
}

// Custom serialization for LlmTier since we use it in LlmRequest serialization.
impl Serialize for LlmTier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            LlmTier::RuleBased => serializer.serialize_str("rule_based"),
            LlmTier::SmallLocal => serializer.serialize_str("small_local"),
            LlmTier::LargeModel => serializer.serialize_str("large_model"),
        }
    }
}
