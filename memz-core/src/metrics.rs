//! Runtime Metrics & Instrumentation (§23)
//!
//! Provides frame-budget monitoring, per-system timing, and Prometheus-compatible
//! metrics for the MEMZ memory system.
//!
//! All MEMZ subsystems emit `tracing` spans for profiling (Tracy-compatible).
//! This module adds lightweight counters and histograms that can be queried
//! at runtime or exported for server dashboards.
//!
//! Design: Lock-free where possible using `AtomicU64` counters.
//! Full histograms use `parking_lot::Mutex` for rare reads (dashboard export).

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use parking_lot::Mutex;

// ---------------------------------------------------------------------------
// Global Counters (lock-free)
// ---------------------------------------------------------------------------

/// Atomic counters for high-frequency events.
/// These are incremented in the hot path and read on dashboard export.
pub struct MemzCounters {
    /// Total episodic memories created since startup.
    pub episodic_created: AtomicU64,
    /// Total memories evicted since startup.
    pub memories_evicted: AtomicU64,
    /// Total gossip propagations since startup.
    pub gossip_propagations: AtomicU64,
    /// Total LLM calls by tier (indices 0, 1, 2).
    pub llm_calls_tier0: AtomicU64,
    /// Tier 1 LLM calls.
    pub llm_calls_tier1: AtomicU64,
    /// Tier 2 LLM calls.
    pub llm_calls_tier2: AtomicU64,
    /// LLM parse failures.
    pub llm_parse_failures: AtomicU64,
    /// Memory injection attempts (total).
    pub injection_attempts: AtomicU64,
    /// Memory injections accepted.
    pub injection_accepted: AtomicU64,
    /// Memory injections rejected.
    pub injection_rejected: AtomicU64,
    /// Save operations completed.
    pub saves_completed: AtomicU64,
    /// Decay passes completed.
    pub decay_passes: AtomicU64,
}

impl MemzCounters {
    /// Create a new set of zeroed counters.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            episodic_created: AtomicU64::new(0),
            memories_evicted: AtomicU64::new(0),
            gossip_propagations: AtomicU64::new(0),
            llm_calls_tier0: AtomicU64::new(0),
            llm_calls_tier1: AtomicU64::new(0),
            llm_calls_tier2: AtomicU64::new(0),
            llm_parse_failures: AtomicU64::new(0),
            injection_attempts: AtomicU64::new(0),
            injection_accepted: AtomicU64::new(0),
            injection_rejected: AtomicU64::new(0),
            saves_completed: AtomicU64::new(0),
            decay_passes: AtomicU64::new(0),
        }
    }

    /// Snapshot all counters for export.
    #[must_use]
    pub fn snapshot(&self) -> CounterSnapshot {
        CounterSnapshot {
            episodic_created: self.episodic_created.load(Ordering::Relaxed),
            memories_evicted: self.memories_evicted.load(Ordering::Relaxed),
            gossip_propagations: self.gossip_propagations.load(Ordering::Relaxed),
            llm_calls: [
                self.llm_calls_tier0.load(Ordering::Relaxed),
                self.llm_calls_tier1.load(Ordering::Relaxed),
                self.llm_calls_tier2.load(Ordering::Relaxed),
            ],
            llm_parse_failures: self.llm_parse_failures.load(Ordering::Relaxed),
            injection_attempts: self.injection_attempts.load(Ordering::Relaxed),
            injection_accepted: self.injection_accepted.load(Ordering::Relaxed),
            injection_rejected: self.injection_rejected.load(Ordering::Relaxed),
            saves_completed: self.saves_completed.load(Ordering::Relaxed),
            decay_passes: self.decay_passes.load(Ordering::Relaxed),
        }
    }
}

impl Default for MemzCounters {
    fn default() -> Self {
        Self::new()
    }
}

/// A snapshot of counter values at a point in time.
#[derive(Debug, Clone)]
pub struct CounterSnapshot {
    /// Total episodic memories created.
    pub episodic_created: u64,
    /// Total memories evicted.
    pub memories_evicted: u64,
    /// Total gossip propagations.
    pub gossip_propagations: u64,
    /// LLM calls by tier [tier0, tier1, tier2].
    pub llm_calls: [u64; 3],
    /// LLM parse failures.
    pub llm_parse_failures: u64,
    /// Total injection attempts.
    pub injection_attempts: u64,
    /// Accepted injections.
    pub injection_accepted: u64,
    /// Rejected injections.
    pub injection_rejected: u64,
    /// Completed save operations.
    pub saves_completed: u64,
    /// Completed decay passes.
    pub decay_passes: u64,
}

impl CounterSnapshot {
    /// Format as Prometheus-compatible text.
    #[must_use]
    pub fn to_prometheus(&self) -> String {
        format!(
            "# HELP memz_episodic_created_total Total episodic memories created\n\
             # TYPE memz_episodic_created_total counter\n\
             memz_episodic_created_total {}\n\
             # HELP memz_memories_evicted_total Total memories evicted\n\
             # TYPE memz_memories_evicted_total counter\n\
             memz_memories_evicted_total {}\n\
             # HELP memz_gossip_propagations_total Gossip events propagated\n\
             # TYPE memz_gossip_propagations_total counter\n\
             memz_gossip_propagations_total {}\n\
             # HELP memz_llm_calls_total LLM calls by tier\n\
             # TYPE memz_llm_calls_total counter\n\
             memz_llm_calls_total{{tier=\"0\"}} {}\n\
             memz_llm_calls_total{{tier=\"1\"}} {}\n\
             memz_llm_calls_total{{tier=\"2\"}} {}\n\
             # HELP memz_llm_parse_failures_total LLM parse failures\n\
             # TYPE memz_llm_parse_failures_total counter\n\
             memz_llm_parse_failures_total {}\n\
             # HELP memz_injection_attempts_total Memory injection attempts\n\
             # TYPE memz_injection_attempts_total counter\n\
             memz_injection_attempts_total {}\n\
             # HELP memz_injection_accepted_total Accepted injections\n\
             # TYPE memz_injection_accepted_total counter\n\
             memz_injection_accepted_total {}\n\
             # HELP memz_injection_rejected_total Rejected injections\n\
             # TYPE memz_injection_rejected_total counter\n\
             memz_injection_rejected_total {}\n\
             # HELP memz_saves_completed_total Save operations completed\n\
             # TYPE memz_saves_completed_total counter\n\
             memz_saves_completed_total {}\n\
             # HELP memz_decay_passes_total Decay passes completed\n\
             # TYPE memz_decay_passes_total counter\n\
             memz_decay_passes_total {}\n",
            self.episodic_created,
            self.memories_evicted,
            self.gossip_propagations,
            self.llm_calls[0],
            self.llm_calls[1],
            self.llm_calls[2],
            self.llm_parse_failures,
            self.injection_attempts,
            self.injection_accepted,
            self.injection_rejected,
            self.saves_completed,
            self.decay_passes,
        )
    }
}

// ---------------------------------------------------------------------------
// Frame Budget Monitor
// ---------------------------------------------------------------------------

/// Tracks per-frame time spent in MEMZ subsystems.
///
/// Usage:
/// ```rust,no_run
/// # use memz_core::metrics::FrameBudgetMonitor;
/// let mut monitor = FrameBudgetMonitor::new(2.0); // 2ms budget
/// let _guard = monitor.begin_frame();
/// // ... do memory work ...
/// drop(_guard);
/// assert!(monitor.last_frame_ms() < 2.0);
/// ```
pub struct FrameBudgetMonitor {
    /// Maximum allowed milliseconds per frame for MEMZ work.
    budget_ms: f64,
    /// Timing history (last N frames).
    history: Mutex<FrameHistory>,
}

/// Internal frame timing data.
struct FrameHistory {
    /// Ring buffer of recent frame timings (milliseconds).
    timings: Vec<f64>,
    /// Next write index.
    write_idx: usize,
    /// Number of frames recorded.
    count: u64,
    /// Whether the last frame exceeded the budget.
    last_over_budget: bool,
}

impl FrameBudgetMonitor {
    /// Create a new monitor with the given budget (milliseconds).
    #[must_use]
    pub fn new(budget_ms: f64) -> Self {
        Self {
            budget_ms,
            history: Mutex::new(FrameHistory {
                timings: vec![0.0; 256], // Track last 256 frames
                write_idx: 0,
                count: 0,
                last_over_budget: false,
            }),
        }
    }

    /// Begin timing a frame. Returns a guard that records elapsed time on drop.
    pub fn begin_frame(&self) -> FrameGuard<'_> {
        FrameGuard {
            monitor: self,
            start: Instant::now(),
        }
    }

    /// Record a frame timing manually (milliseconds).
    pub fn record(&self, ms: f64) {
        let mut h = self.history.lock();
        let idx = h.write_idx;
        let len = h.timings.len();
        h.timings[idx] = ms;
        h.write_idx = (idx + 1) % len;
        h.count += 1;
        h.last_over_budget = ms > self.budget_ms;
    }

    /// Get the last frame's timing (milliseconds).
    #[must_use]
    pub fn last_frame_ms(&self) -> f64 {
        let h = self.history.lock();
        if h.count == 0 {
            return 0.0;
        }
        let idx = if h.write_idx == 0 {
            h.timings.len() - 1
        } else {
            h.write_idx - 1
        };
        h.timings[idx]
    }

    /// Whether the last frame exceeded the budget.
    #[must_use]
    pub fn is_over_budget(&self) -> bool {
        self.history.lock().last_over_budget
    }

    /// Get P50, P95, P99 timings from the history buffer (milliseconds).
    #[must_use]
    pub fn percentiles(&self) -> FramePercentiles {
        let h = self.history.lock();
        let n = (h.count as usize).min(h.timings.len());
        if n == 0 {
            return FramePercentiles {
                p50: 0.0,
                p95: 0.0,
                p99: 0.0,
                max: 0.0,
                over_budget_ratio: 0.0,
            };
        }

        let mut sorted: Vec<f64> = if h.count as usize <= h.timings.len() {
            h.timings[..n].to_vec()
        } else {
            h.timings.clone()
        };
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let p50 = sorted[n / 2];
        let p95 = sorted[(n as f64 * 0.95) as usize];
        let p99 = sorted[(n as f64 * 0.99) as usize];
        let max = sorted[n - 1];
        let over_count = sorted.iter().filter(|&&t| t > self.budget_ms).count();

        FramePercentiles {
            p50,
            p95,
            p99,
            max,
            over_budget_ratio: over_count as f64 / n as f64,
        }
    }

    /// Total number of frames recorded.
    #[must_use]
    pub fn frame_count(&self) -> u64 {
        self.history.lock().count
    }

    /// The configured budget in milliseconds.
    #[must_use]
    pub fn budget_ms(&self) -> f64 {
        self.budget_ms
    }
}

/// RAII guard that records elapsed time when dropped.
pub struct FrameGuard<'a> {
    monitor: &'a FrameBudgetMonitor,
    start: Instant,
}

impl Drop for FrameGuard<'_> {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        let ms = elapsed.as_secs_f64() * 1000.0;
        self.monitor.record(ms);
    }
}

/// Percentile statistics for frame timings.
#[derive(Debug, Clone)]
pub struct FramePercentiles {
    /// 50th percentile (median) in milliseconds.
    pub p50: f64,
    /// 95th percentile in milliseconds.
    pub p95: f64,
    /// 99th percentile in milliseconds.
    pub p99: f64,
    /// Maximum observed timing.
    pub max: f64,
    /// Ratio of frames that exceeded the budget (0.0–1.0).
    pub over_budget_ratio: f64,
}

impl FramePercentiles {
    /// Format as a human-readable summary.
    #[must_use]
    pub fn summary(&self, budget_ms: f64) -> String {
        format!(
            "P50={:.2}ms  P95={:.2}ms  P99={:.2}ms  Max={:.2}ms  Budget={budget_ms:.1}ms  \
             Over-budget={:.1}%",
            self.p50,
            self.p95,
            self.p99,
            self.max,
            self.over_budget_ratio * 100.0,
        )
    }
}

// ---------------------------------------------------------------------------
// Tracing Span Names (constants for Tracy integration)
// ---------------------------------------------------------------------------

/// Span names used with `tracing::span!` for Tracy profiler integration.
pub mod spans {
    /// Top-level per-frame span.
    pub const MEMZ_FRAME: &str = "memz::frame";
    /// Memory creation span.
    pub const MEMORY_CREATE: &str = "memz::memory::create";
    /// Memory retrieval span.
    pub const MEMORY_RETRIEVE: &str = "memz::memory::retrieve";
    /// Memory decay pass.
    pub const DECAY_PASS: &str = "memz::decay";
    /// Eviction pass.
    pub const EVICTION_PASS: &str = "memz::eviction";
    /// Gossip propagation.
    pub const GOSSIP: &str = "memz::gossip";
    /// Reputation update.
    pub const REPUTATION: &str = "memz::reputation";
    /// LLM call (async).
    pub const LLM_CALL: &str = "memz::llm::call";
    /// Persistence save.
    pub const PERSIST_SAVE: &str = "memz::persist::save";
    /// Persistence load.
    pub const PERSIST_LOAD: &str = "memz::persist::load";
    /// Reflection generation.
    pub const REFLECTION: &str = "memz::reflection";
    /// Observation pipeline.
    pub const OBSERVATION: &str = "memz::observation";
    /// Consolidation pass.
    pub const CONSOLIDATION: &str = "memz::consolidation";
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counters_default_zero() {
        let c = MemzCounters::new();
        let snap = c.snapshot();
        assert_eq!(snap.episodic_created, 0);
        assert_eq!(snap.memories_evicted, 0);
        assert_eq!(snap.llm_calls, [0, 0, 0]);
    }

    #[test]
    fn counters_increment_and_snapshot() {
        let c = MemzCounters::new();
        c.episodic_created.fetch_add(5, Ordering::Relaxed);
        c.gossip_propagations.fetch_add(3, Ordering::Relaxed);
        c.llm_calls_tier1.fetch_add(1, Ordering::Relaxed);
        c.injection_attempts.fetch_add(10, Ordering::Relaxed);
        c.injection_accepted.fetch_add(7, Ordering::Relaxed);
        c.injection_rejected.fetch_add(3, Ordering::Relaxed);

        let snap = c.snapshot();
        assert_eq!(snap.episodic_created, 5);
        assert_eq!(snap.gossip_propagations, 3);
        assert_eq!(snap.llm_calls, [0, 1, 0]);
        assert_eq!(snap.injection_attempts, 10);
        assert_eq!(snap.injection_accepted, 7);
        assert_eq!(snap.injection_rejected, 3);
    }

    #[test]
    fn prometheus_format_valid() {
        let c = MemzCounters::new();
        c.episodic_created.fetch_add(42, Ordering::Relaxed);
        let prom = c.snapshot().to_prometheus();
        assert!(prom.contains("memz_episodic_created_total 42"));
        assert!(prom.contains("# TYPE"));
        assert!(prom.contains("# HELP"));
    }

    #[test]
    fn frame_budget_monitor_records() {
        let monitor = FrameBudgetMonitor::new(2.0);
        assert_eq!(monitor.frame_count(), 0);

        monitor.record(0.5);
        monitor.record(1.0);
        monitor.record(1.5);

        assert_eq!(monitor.frame_count(), 3);
        assert!((monitor.last_frame_ms() - 1.5).abs() < 0.001);
        assert!(!monitor.is_over_budget());
    }

    #[test]
    fn frame_budget_detects_over_budget() {
        let monitor = FrameBudgetMonitor::new(2.0);
        monitor.record(3.0); // Over budget!
        assert!(monitor.is_over_budget());
    }

    #[test]
    fn frame_guard_records_timing() {
        let monitor = FrameBudgetMonitor::new(100.0);
        {
            let _guard = monitor.begin_frame();
            // Do some trivial work
            let mut _sum = 0u64;
            for i in 0..1000 {
                _sum += i;
            }
        }
        assert_eq!(monitor.frame_count(), 1);
        assert!(monitor.last_frame_ms() < 100.0); // Should be well under 100ms
    }

    #[test]
    fn percentiles_with_data() {
        let monitor = FrameBudgetMonitor::new(2.0);
        for i in 0..100 {
            monitor.record(i as f64 * 0.02); // 0.0 to 1.98ms
        }

        let pct = monitor.percentiles();
        assert!(pct.p50 > 0.0);
        assert!(pct.p95 >= pct.p50);
        assert!(pct.p99 >= pct.p95);
        assert!((pct.over_budget_ratio - 0.0).abs() < 0.01); // All under 2ms
    }

    #[test]
    fn percentiles_summary_format() {
        let monitor = FrameBudgetMonitor::new(2.0);
        monitor.record(0.5);
        monitor.record(1.0);
        monitor.record(1.5);

        let pct = monitor.percentiles();
        let summary = pct.summary(2.0);
        assert!(summary.contains("P50="));
        assert!(summary.contains("P95="));
        assert!(summary.contains("Budget=2.0ms"));
    }

    #[test]
    fn span_names_are_not_empty() {
        assert!(!spans::MEMZ_FRAME.is_empty());
        assert!(!spans::MEMORY_CREATE.is_empty());
        assert!(!spans::MEMORY_RETRIEVE.is_empty());
        assert!(!spans::DECAY_PASS.is_empty());
        assert!(!spans::LLM_CALL.is_empty());
    }
}
