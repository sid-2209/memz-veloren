//! Veloren-specific configuration for the MEMZ memory system (§12.7).
//!
//! This module provides hardware-aware profiles and Veloren-specific
//! tuning parameters on top of the base `memz_core::config::MemoryConfig`.

use memz_core::config::MemoryConfig;

// ---------------------------------------------------------------------------
// Hardware Profiles (§12.7)
// ---------------------------------------------------------------------------

/// Hardware capability profile — auto-detected at startup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareProfile {
    /// 4-core CPU, 8GB RAM, no GPU. Rule-based only.
    UltraLow,
    /// 6-core CPU, 16GB RAM, iGPU. Embeddings only, no LLM.
    Low,
    /// Ryzen 5 / i5, 16GB RAM, RTX 3060. Local 1–3B model.
    Medium,
    /// Ryzen 7 / i7, 32GB RAM, RTX 4070+. Local 7B model.
    High,
    /// Any hardware + API key. Local for speed, cloud for depth.
    CloudAssisted,
}

impl HardwareProfile {
    /// Get a human-readable description.
    #[must_use]
    pub fn description(self) -> &'static str {
        match self {
            Self::UltraLow => "Ultra-Low: Rule-based only, keyword matching",
            Self::Low => "Low: Vector retrieval, no LLM",
            Self::Medium => "Medium: Full system with local 1-3B LLM",
            Self::High => "High: Full system with local 7B LLM",
            Self::CloudAssisted => "Cloud-Assisted: Local + cloud API",
        }
    }

    /// Whether this profile supports embedding generation.
    #[must_use]
    pub fn has_embeddings(self) -> bool {
        !matches!(self, Self::UltraLow)
    }

    /// Whether this profile supports LLM calls.
    #[must_use]
    pub fn has_llm(self) -> bool {
        matches!(self, Self::Medium | Self::High | Self::CloudAssisted)
    }

    /// Maximum LLM tier available on this profile.
    #[must_use]
    pub fn max_llm_tier(self) -> u8 {
        match self {
            Self::UltraLow => 0,
            Self::Low => 0,
            Self::Medium => 1,
            Self::High => 2,
            Self::CloudAssisted => 2,
        }
    }

    /// Recommended max active NPCs with memory processing.
    #[must_use]
    pub fn max_active_npcs(self) -> usize {
        match self {
            Self::UltraLow => 10,
            Self::Low => 30,
            Self::Medium => 50,
            Self::High => 100,
            Self::CloudAssisted => 80,
        }
    }

    /// Simple hardware detection heuristic.
    ///
    /// In a real implementation this would probe:
    /// - CPU core count via `num_cpus`
    /// - Available RAM via `sysinfo`
    /// - GPU presence via Vulkan / CUDA probing
    /// - Ollama availability via HTTP health check
    ///
    /// For now, returns Medium as a safe default.
    #[must_use]
    pub fn auto_detect() -> Self {
        let cpu_count = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);

        if cpu_count >= 12 {
            Self::High
        } else if cpu_count >= 6 {
            Self::Medium
        } else if cpu_count >= 4 {
            Self::Low
        } else {
            Self::UltraLow
        }
    }
}

impl Default for HardwareProfile {
    fn default() -> Self {
        Self::auto_detect()
    }
}

// ---------------------------------------------------------------------------
// Veloren Memory Configuration
// ---------------------------------------------------------------------------

/// Extended configuration for the Veloren integration layer.
#[derive(Debug, Clone)]
pub struct VelorenMemzConfig {
    /// Base memory system configuration.
    pub memory: MemoryConfig,
    /// Hardware profile.
    pub profile: HardwareProfile,
    /// Observation radius (world units) — how far NPCs perceive events.
    pub observation_radius: f32,
    /// Gossip radius (world units) — how far gossip reaches in taverns.
    pub gossip_radius: f32,
    /// How frequently (in game ticks) to run the decay pass.
    pub decay_interval_ticks: u64,
    /// How frequently (in game ticks) to check for reflections.
    pub reflection_interval_ticks: u64,
    /// How frequently (in game ticks) to enforce memory limits.
    pub limit_enforcement_interval_ticks: u64,
    /// How frequently (in game ticks) to decay reputation.
    pub reputation_decay_interval_ticks: u64,
    /// Maximum simultaneous LLM requests in flight.
    pub max_concurrent_llm_requests: usize,
    /// Whether to enable the bard composition system.
    pub enable_bard_system: bool,
    /// Whether to enable player memory injection.
    pub enable_player_injection: bool,
    /// Whether to log memory events for debugging.
    pub debug_logging: bool,
}

impl VelorenMemzConfig {
    /// Create a config tuned for the given hardware profile.
    #[must_use]
    pub fn for_profile(profile: HardwareProfile) -> Self {
        let mut config = Self::default();
        config.profile = profile;

        match profile {
            HardwareProfile::UltraLow => {
                config.memory.max_episodic_per_npc = 50;
                config.memory.max_semantic_per_npc = 20;
                config.memory.max_social_per_npc = 30;
                config.observation_radius = 20.0;
                config.gossip_radius = 10.0;
                config.decay_interval_ticks = 120; // Less frequent
                config.reflection_interval_ticks = 0; // Disabled
                config.max_concurrent_llm_requests = 0;
                config.enable_bard_system = false;
            }
            HardwareProfile::Low => {
                config.memory.max_episodic_per_npc = 100;
                config.memory.max_semantic_per_npc = 30;
                config.memory.max_social_per_npc = 50;
                config.observation_radius = 30.0;
                config.gossip_radius = 15.0;
                config.max_concurrent_llm_requests = 0;
                config.enable_bard_system = false;
            }
            HardwareProfile::Medium => {
                // Default values are tuned for Medium
            }
            HardwareProfile::High => {
                config.memory.max_episodic_per_npc = 300;
                config.memory.max_semantic_per_npc = 80;
                config.memory.max_social_per_npc = 150;
                config.observation_radius = 50.0;
                config.gossip_radius = 25.0;
                config.max_concurrent_llm_requests = 4;
            }
            HardwareProfile::CloudAssisted => {
                config.memory.max_episodic_per_npc = 250;
                config.memory.max_semantic_per_npc = 60;
                config.memory.max_social_per_npc = 120;
                config.observation_radius = 40.0;
                config.gossip_radius = 20.0;
                config.max_concurrent_llm_requests = 8; // Cloud can handle more
            }
        }

        config
    }
}

impl Default for VelorenMemzConfig {
    fn default() -> Self {
        Self {
            memory: MemoryConfig::default(),
            profile: HardwareProfile::Medium,
            observation_radius: 32.0,
            gossip_radius: 16.0,
            decay_interval_ticks: 60,
            reflection_interval_ticks: 5000,
            limit_enforcement_interval_ticks: 300,
            reputation_decay_interval_ticks: 10_000,
            max_concurrent_llm_requests: 2,
            enable_bard_system: true,
            enable_player_injection: true,
            debug_logging: false,
        }
    }
}

// ---------------------------------------------------------------------------
// Performance Budget Tracker
// ---------------------------------------------------------------------------

/// Tracks per-frame budget usage for the memory system.
///
/// Used to monitor that MEMZ stays within its 2ms per-frame budget.
#[derive(Debug, Clone, Default)]
pub struct PerformanceBudget {
    /// Observation time (μs).
    pub observation_us: u64,
    /// Decay time (μs).
    pub decay_us: u64,
    /// Retrieval time (μs).
    pub retrieval_us: u64,
    /// Gossip propagation time (μs).
    pub gossip_us: u64,
    /// Behavior modification time (μs).
    pub behavior_us: u64,
    /// Limit enforcement time (μs).
    pub eviction_us: u64,
    /// Number of active NPCs processed this frame.
    pub active_npcs: u32,
    /// Number of memory events processed this frame.
    pub events_processed: u32,
}

impl PerformanceBudget {
    /// Total time spent this frame (μs).
    #[must_use]
    pub fn total_us(&self) -> u64 {
        self.observation_us
            + self.decay_us
            + self.retrieval_us
            + self.gossip_us
            + self.behavior_us
            + self.eviction_us
    }

    /// Whether we're within the 2ms (2000μs) frame budget.
    #[must_use]
    pub fn within_budget(&self) -> bool {
        self.total_us() < 2000
    }

    /// Reset counters for a new frame.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hardware_profile_detection() {
        let profile = HardwareProfile::auto_detect();
        // Should return a valid profile on any system
        assert!(profile.max_active_npcs() > 0);
    }

    #[test]
    fn ultra_low_disables_llm() {
        let config = VelorenMemzConfig::for_profile(HardwareProfile::UltraLow);
        assert!(!config.profile.has_llm());
        assert!(!config.profile.has_embeddings());
        assert_eq!(config.max_concurrent_llm_requests, 0);
        assert!(!config.enable_bard_system);
    }

    #[test]
    fn medium_enables_llm_tier_1() {
        let config = VelorenMemzConfig::for_profile(HardwareProfile::Medium);
        assert!(config.profile.has_llm());
        assert!(config.profile.has_embeddings());
        assert_eq!(config.profile.max_llm_tier(), 1);
    }

    #[test]
    fn high_profile_larger_limits() {
        let high = VelorenMemzConfig::for_profile(HardwareProfile::High);
        let medium = VelorenMemzConfig::for_profile(HardwareProfile::Medium);

        assert!(high.memory.max_episodic_per_npc >= medium.memory.max_episodic_per_npc);
        assert!(high.observation_radius >= medium.observation_radius);
    }

    #[test]
    fn performance_budget_tracking() {
        let mut budget = PerformanceBudget::default();
        budget.observation_us = 100;
        budget.decay_us = 50;
        budget.retrieval_us = 500;
        budget.gossip_us = 300;
        budget.behavior_us = 200;
        budget.eviction_us = 100;

        assert_eq!(budget.total_us(), 1250);
        assert!(budget.within_budget());

        budget.retrieval_us = 1500;
        assert!(!budget.within_budget());
    }

    #[test]
    fn default_config_is_medium() {
        let config = VelorenMemzConfig::default();
        assert_eq!(config.profile, HardwareProfile::Medium);
        assert!(config.enable_bard_system);
        assert!(config.enable_player_injection);
    }
}
