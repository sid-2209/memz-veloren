//! Configuration for the MEMZ memory system.
//!
//! Maps directly to `memz.toml` — see §16 of the design doc.

use serde::{Deserialize, Serialize};

/// Top-level MEMZ configuration, loadable from TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct MemzConfig {
    /// General settings.
    #[serde(default)]
    pub general: GeneralConfig,
    /// Per-NPC memory limits and behavior.
    #[serde(default)]
    pub memory: MemoryConfig,
    /// Retrieval algorithm settings.
    #[serde(default)]
    pub retrieval: RetrievalConfig,
    /// LLM integration settings.
    #[serde(default)]
    pub llm: LlmConfig,
    /// Social memory propagation settings.
    #[serde(default)]
    pub social: SocialConfig,
    /// First-five-minutes experience tuning.
    #[serde(default)]
    pub first_five_minutes: FirstFiveMinutesConfig,
    /// Performance budget enforcement.
    #[serde(default)]
    pub performance: PerformanceConfig,
    /// Persistence / save settings.
    #[serde(default)]
    pub persistence: PersistenceConfig,
    /// Safety & content filtering.
    #[serde(default)]
    pub safety: SafetyConfig,
    /// Accessibility settings.
    #[serde(default)]
    pub accessibility: AccessibilityConfig,
    /// Telemetry & observability.
    #[serde(default)]
    pub telemetry: TelemetryConfig,
}


impl MemzConfig {
    /// Load configuration from a TOML string.
    ///
    /// # Errors
    /// Returns `MemzError::Config` if the TOML is invalid.
    pub fn from_toml(toml_str: &str) -> crate::error::Result<Self> {
        toml::from_str(toml_str).map_err(|e| crate::MemzError::Config(e.to_string()))
    }

    /// Load configuration from a TOML file.
    ///
    /// # Errors
    /// Returns an error if the file cannot be read or parsed.
    pub fn from_file(path: &std::path::Path) -> crate::error::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_toml(&content)
    }
}

// ---------------------------------------------------------------------------
// Sub-configs
// ---------------------------------------------------------------------------

/// General system settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Whether the memory system is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Log level: trace, debug, info, warn, error.
    #[serde(default = "default_log_level")]
    pub log_level: String,
    /// Hardware profile: auto, minimal, standard, high, server, dev.
    #[serde(default = "default_profile")]
    pub profile: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_level: "info".to_string(),
            profile: "auto".to_string(),
        }
    }
}

/// Per-character memory capacity and behavior configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Hard cap on episodic memories per NPC.
    #[serde(default = "default_200")]
    pub max_episodic_per_npc: usize,
    /// Distilled knowledge cap per NPC.
    #[serde(default = "default_50")]
    pub max_semantic_per_npc: usize,
    /// Gossip / hearsay cap per NPC.
    #[serde(default = "default_100")]
    pub max_social_per_npc: usize,
    /// Skills and routines cap per NPC.
    #[serde(default = "default_30")]
    pub max_procedural_per_npc: usize,
    /// Deep thoughts cap per NPC.
    #[serde(default = "default_20_usize")]
    pub max_reflective_per_npc: usize,
    /// Base Ebbinghaus decay constant per game-day.
    #[serde(default = "default_decay_rate")]
    pub decay_rate: f32,
    /// How often memory consolidation runs (game-days).
    #[serde(default = "default_1_usize")]
    pub consolidation_interval_days: usize,
    /// Max milliseconds per NPC per consolidation cycle.
    #[serde(default = "default_consolidation_budget")]
    pub consolidation_budget_ms: f32,
    /// Eviction ring configuration.
    #[serde(default)]
    pub eviction: EvictionConfig,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_episodic_per_npc: 200,
            max_semantic_per_npc: 50,
            max_social_per_npc: 100,
            max_procedural_per_npc: 30,
            max_reflective_per_npc: 20,
            decay_rate: 0.05,
            consolidation_interval_days: 1,
            consolidation_budget_ms: 0.1,
            eviction: EvictionConfig::default(),
        }
    }
}

/// Multi-tier eviction ring configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvictionConfig {
    /// In-memory hot ring: last N game-hours.
    #[serde(default = "default_24")]
    pub hot_ring_hours: u32,
    /// In-memory warm ring: last N game-days.
    #[serde(default = "default_7")]
    pub warm_ring_days: u32,
    /// `SQLite` cold ring: last N game-days.
    #[serde(default = "default_90")]
    pub cold_ring_days: u32,
    /// Protect memories with |`emotional_valence`| above this threshold.
    #[serde(default = "default_0_8")]
    pub protect_emotional_threshold: f32,
    /// Whether to protect first-meeting memories from eviction.
    #[serde(default = "default_true")]
    pub protect_first_meeting: bool,
}

impl Default for EvictionConfig {
    fn default() -> Self {
        Self {
            hot_ring_hours: 24,
            warm_ring_days: 7,
            cold_ring_days: 90,
            protect_emotional_threshold: 0.8,
            protect_first_meeting: true,
        }
    }
}

/// Memory retrieval algorithm settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalConfig {
    /// Algorithm: "hnsw", "`brute_force`", "tfidf".
    #[serde(default = "default_hnsw")]
    pub algorithm: String,
    /// Number of memories retrieved per interaction.
    #[serde(default = "default_5_usize")]
    pub top_k: usize,
    /// ONNX embedding model name.
    #[serde(default = "default_embedding_model")]
    pub embedding_model: String,
    /// Embedding vector dimensions.
    #[serde(default = "default_384")]
    pub embedding_dimensions: usize,
    /// Retrieval weight tuning.
    #[serde(default)]
    pub weights: RetrievalWeights,
}

impl Default for RetrievalConfig {
    fn default() -> Self {
        Self {
            algorithm: "hnsw".to_string(),
            top_k: 5,
            embedding_model: "all-MiniLM-L6-v2".to_string(),
            embedding_dimensions: 384,
            weights: RetrievalWeights::default(),
        }
    }
}

/// Retrieval scoring weights — must sum to ~1.0.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalWeights {
    /// Weight for recency factor.
    #[serde(default = "default_0_2")]
    pub recency: f32,
    /// Weight for semantic relevance.
    #[serde(default = "default_0_3")]
    pub relevance: f32,
    /// Weight for importance factor.
    #[serde(default = "default_0_2")]
    pub importance: f32,
    /// Weight for emotional intensity.
    #[serde(default = "default_0_2")]
    pub emotional: f32,
    /// Weight for social source trust.
    #[serde(default = "default_0_1")]
    pub social: f32,
}

impl Default for RetrievalWeights {
    fn default() -> Self {
        Self {
            recency: 0.20,
            relevance: 0.30,
            importance: 0.20,
            emotional: 0.20,
            social: 0.10,
        }
    }
}

/// LLM integration configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Provider: "ollama", "openai", "`llama_cpp`", "none".
    #[serde(default = "default_ollama")]
    pub provider: String,
    /// Base URL for the LLM API.
    #[serde(default = "default_ollama_url")]
    pub base_url: String,
    /// Model name for Tier 1 (small, fast, local).
    #[serde(default = "default_tier1_model")]
    pub tier1_model: String,
    /// Model name for Tier 2 (large, deep reasoning).
    #[serde(default = "default_tier2_model")]
    pub tier2_model: String,
    /// Max Tier 2 calls per hour.
    #[serde(default = "default_20")]
    pub max_tier2_calls_per_hour: u32,
    /// Hard timeout for any LLM call in milliseconds.
    #[serde(default = "default_5000")]
    pub request_timeout_ms: u64,
    /// Enforce structured output (GBNF/JSON mode).
    #[serde(default = "default_true")]
    pub structured_output: bool,
    /// Auto-retry with simplified prompt on parse failure.
    #[serde(default = "default_true")]
    pub retry_on_parse_failure: bool,
    /// Max retries before falling back to rule-based.
    #[serde(default = "default_2")]
    pub max_retries: u32,
    /// Fallback chain configuration.
    #[serde(default)]
    pub fallback: FallbackConfig,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: "ollama".to_string(),
            base_url: "http://localhost:11434".to_string(),
            tier1_model: "qwen2.5:1.5b".to_string(),
            tier2_model: "mistral:7b-instruct".to_string(),
            max_tier2_calls_per_hour: 20,
            request_timeout_ms: 5000,
            structured_output: true,
            retry_on_parse_failure: true,
            max_retries: 2,
            fallback: FallbackConfig::default(),
        }
    }
}

/// Graceful degradation chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackConfig {
    /// What to use when Tier 2 is unavailable.
    #[serde(default = "default_tier1")]
    pub tier2_fallback: String,
    /// What to use when Tier 1 is unavailable.
    #[serde(default = "default_templates")]
    pub tier1_fallback: String,
    /// What to use when templates fail.
    #[serde(default = "default_silent")]
    pub templates_fallback: String,
}

impl Default for FallbackConfig {
    fn default() -> Self {
        Self {
            tier2_fallback: "tier1".to_string(),
            tier1_fallback: "templates".to_string(),
            templates_fallback: "silent".to_string(),
        }
    }
}

/// Social memory propagation settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialConfig {
    /// Default gossip tendency for NPCs.
    #[serde(default = "default_0_5")]
    pub gossip_tendency_default: f32,
    /// Gossip propagation speed multiplier.
    #[serde(default = "default_1_0")]
    pub gossip_propagation_speed: f32,
    /// Trust decay rate per game-day without reinforcement.
    #[serde(default = "default_trust_decay")]
    pub trust_decay_rate: f32,
    /// Max gossip chain depth (telephone game degradation).
    #[serde(default = "default_4")]
    pub max_gossip_chain_depth: u32,
}

impl Default for SocialConfig {
    fn default() -> Self {
        Self {
            gossip_tendency_default: 0.5,
            gossip_propagation_speed: 1.0,
            trust_decay_rate: 0.01,
            max_gossip_chain_depth: 4,
        }
    }
}

/// New-player experience tuning (§14.0).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirstFiveMinutesConfig {
    /// Whether starter-area tuning is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Gossip speed multiplier in starter areas.
    #[serde(default = "default_5_0")]
    pub starter_area_gossip_speed_multiplier: f32,
    /// Duration (hours) for recency weight boost for new players.
    #[serde(default = "default_1_0")]
    pub recency_weight_boost_duration_hours: f32,
    /// Enable the "fuzzy seed" NPC near spawn.
    #[serde(default = "default_true")]
    pub fuzzy_seed_npc_enabled: bool,
    /// Guarantee NPC recognition on second visit.
    #[serde(default = "default_true")]
    pub guaranteed_recognition_on_second_visit: bool,
}

impl Default for FirstFiveMinutesConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            starter_area_gossip_speed_multiplier: 5.0,
            recency_weight_boost_duration_hours: 1.0,
            fuzzy_seed_npc_enabled: true,
            guaranteed_recognition_on_second_visit: true,
        }
    }
}

/// Performance budget enforcement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Hard limit: total MEMZ time per frame in milliseconds.
    #[serde(default = "default_2_0")]
    pub frame_budget_ms: f32,
    /// Max microseconds per memory creation.
    #[serde(default = "default_10")]
    pub memory_creation_budget_us: u32,
    /// Max microseconds per retrieval query.
    #[serde(default = "default_500")]
    pub retrieval_budget_us: u32,
    /// Only process NPCs within this chunk radius.
    #[serde(default = "default_3")]
    pub active_npc_radius_chunks: u32,
    /// Max concurrent LLM requests.
    #[serde(default = "default_2")]
    pub max_concurrent_llm_requests: u32,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            frame_budget_ms: 2.0,
            memory_creation_budget_us: 10,
            retrieval_budget_us: 500,
            active_npc_radius_chunks: 3,
            max_concurrent_llm_requests: 2,
        }
    }
}

/// Persistence / save configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceConfig {
    /// Backend: "sqlite" or "json" (debug).
    #[serde(default = "default_sqlite")]
    pub backend: String,
    /// Use WAL mode for concurrent reads.
    #[serde(default = "default_true")]
    pub wal_mode: bool,
    /// Auto-save interval in seconds.
    #[serde(default = "default_300")]
    pub auto_save_interval_seconds: u32,
    /// Number of save backups to keep.
    #[serde(default = "default_3")]
    pub backup_count: u32,
    /// Detect save corruption via checksums.
    #[serde(default = "default_true")]
    pub checksum_enabled: bool,
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            backend: "sqlite".to_string(),
            wal_mode: true,
            auto_save_interval_seconds: 300,
            backup_count: 3,
            checksum_enabled: true,
        }
    }
}

/// Safety and content filtering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    /// Whether to filter player memory injections.
    #[serde(default = "default_true")]
    pub content_filter_enabled: bool,
    /// Max injection attempts per minute.
    #[serde(default = "default_5")]
    pub injection_rate_limit_per_minute: u32,
    /// Max character length for injected memories.
    #[serde(default = "default_500")]
    pub max_injection_length_chars: u32,
    /// Profanity filter level: "off", "moderate", "strict".
    #[serde(default = "default_moderate")]
    pub profanity_filter: String,
    /// Audit trail for moderation events.
    #[serde(default = "default_true")]
    pub log_moderation_events: bool,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            content_filter_enabled: true,
            injection_rate_limit_per_minute: 5,
            max_injection_length_chars: 500,
            profanity_filter: "moderate".to_string(),
            log_moderation_events: true,
        }
    }
}

/// Accessibility configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessibilityConfig {
    /// Screen reader support.
    #[serde(default = "default_true")]
    pub screen_reader_support: bool,
    /// High contrast UI mode.
    #[serde(default)]
    pub high_contrast_ui: bool,
    /// Reduce motion / animations.
    #[serde(default)]
    pub reduce_motion: bool,
    /// Text size multiplier (1.0 = default).
    #[serde(default = "default_1_0")]
    pub text_size_multiplier: f32,
    /// Full keyboard navigation for Memory Journal.
    #[serde(default = "default_true")]
    pub memory_journal_keyboard_only: bool,
}

impl Default for AccessibilityConfig {
    fn default() -> Self {
        Self {
            screen_reader_support: true,
            high_contrast_ui: false,
            reduce_motion: false,
            text_size_multiplier: 1.0,
            memory_journal_keyboard_only: true,
        }
    }
}

/// Telemetry and observability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Opt-in only.
    #[serde(default)]
    pub enabled: bool,
    /// Prometheus metrics endpoint.
    #[serde(default = "default_prom_endpoint")]
    pub prometheus_endpoint: String,
    /// Export Tracy profiler spans.
    #[serde(default = "default_true")]
    pub export_tracy: bool,
    /// Log any operation exceeding this threshold (ms).
    #[serde(default = "default_5_0")]
    pub log_slow_operations_ms: f32,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            prometheus_endpoint: "127.0.0.1:9090".to_string(),
            export_tracy: true,
            log_slow_operations_ms: 5.0,
        }
    }
}

// ---------------------------------------------------------------------------
// Serde default helpers
// ---------------------------------------------------------------------------

fn default_true() -> bool { true }
fn default_log_level() -> String { "info".to_string() }
fn default_profile() -> String { "auto".to_string() }
fn default_hnsw() -> String { "hnsw".to_string() }
fn default_embedding_model() -> String { "all-MiniLM-L6-v2".to_string() }
fn default_ollama() -> String { "ollama".to_string() }
fn default_ollama_url() -> String { "http://localhost:11434".to_string() }
fn default_tier1_model() -> String { "qwen2.5:1.5b".to_string() }
fn default_tier2_model() -> String { "mistral:7b-instruct".to_string() }
fn default_tier1() -> String { "tier1".to_string() }
fn default_templates() -> String { "templates".to_string() }
fn default_silent() -> String { "silent".to_string() }
fn default_sqlite() -> String { "sqlite".to_string() }
fn default_moderate() -> String { "moderate".to_string() }
fn default_prom_endpoint() -> String { "127.0.0.1:9090".to_string() }
fn default_0_1() -> f32 { 0.1 }
fn default_0_2() -> f32 { 0.2 }
fn default_0_3() -> f32 { 0.3 }
fn default_0_5() -> f32 { 0.5 }
fn default_0_8() -> f32 { 0.8 }
fn default_1_0() -> f32 { 1.0 }
fn default_2_0() -> f32 { 2.0 }
fn default_5_0() -> f32 { 5.0 }
fn default_decay_rate() -> f32 { 0.05 }
fn default_trust_decay() -> f32 { 0.01 }
fn default_consolidation_budget() -> f32 { 0.1 }
fn default_1_usize() -> usize { 1 }
fn default_2() -> u32 { 2 }
fn default_3() -> u32 { 3 }
fn default_4() -> u32 { 4 }
fn default_5() -> u32 { 5 }
fn default_5_usize() -> usize { 5 }
fn default_7() -> u32 { 7 }
fn default_10() -> u32 { 10 }
fn default_20() -> u32 { 20 }
fn default_20_usize() -> usize { 20 }
fn default_24() -> u32 { 24 }
fn default_30() -> usize { 30 }
fn default_50() -> usize { 50 }
fn default_90() -> u32 { 90 }
fn default_100() -> usize { 100 }
fn default_200() -> usize { 200 }
fn default_300() -> u32 { 300 }
fn default_384() -> usize { 384 }
fn default_500() -> u32 { 500 }
fn default_5000() -> u64 { 5000 }
