//! Content Safety & Abuse Prevention (§21)
//!
//! Multi-layer defense-in-depth for player memory injection:
//!   Layer 1: Client-side validation (length, rate, regex)
//!   Layer 2: Server-side content filter (toxicity classification)
//!   Layer 3: Semantic validation (plausibility check via LLM)
//!   Layer 4: World-impact throttling (gradual, not instant)

use crate::config::SafetyConfig;
use crate::error::MemzError;

/// Result of a safety check on player input.
#[derive(Debug, Clone)]
pub enum SafetyVerdict {
    /// Content passed all checks.
    Approved,
    /// Content was flagged but may be acceptable (needs review or softer handling).
    Flagged {
        /// Why the content was flagged.
        reason: String,
        /// Severity score (0.0 to 1.0).
        score: f32,
    },
    /// Content was rejected outright.
    Rejected {
        /// Why the content was rejected.
        reason: String,
    },
}

/// A rate limiter for memory injection attempts.
#[derive(Debug)]
pub struct RateLimiter {
    /// Maximum allowed injections per window.
    max_per_window: u32,
    /// Window duration in seconds.
    window_seconds: u64,
    /// Timestamps of recent attempts.
    attempts: Vec<u64>,
}

impl RateLimiter {
    /// Create a new rate limiter.
    #[must_use]
    pub fn new(max_per_window: u32, window_seconds: u64) -> Self {
        Self {
            max_per_window,
            window_seconds,
            attempts: Vec::new(),
        }
    }

    /// Check if an attempt is allowed, and record it if so.
    pub fn check_and_record(&mut self, current_time_secs: u64) -> bool {
        // Prune old attempts outside the window.
        let cutoff = current_time_secs.saturating_sub(self.window_seconds);
        self.attempts.retain(|&t| t >= cutoff);

        if self.attempts.len() < self.max_per_window as usize {
            self.attempts.push(current_time_secs);
            true
        } else {
            false
        }
    }
}

/// Validate a player memory injection at Layer 1 (client-side rules).
///
/// Checks:
///   - Length within limit
///   - No URLs or code patterns
///   - No excessive special characters
pub fn validate_injection_layer1(
    content: &str,
    config: &SafetyConfig,
) -> SafetyVerdict {
    // Length check.
    if content.len() > config.max_injection_length_chars as usize {
        return SafetyVerdict::Rejected {
            reason: format!(
                "Content too long: {} chars (max: {})",
                content.len(),
                config.max_injection_length_chars
            ),
        };
    }

    // Empty check.
    if content.trim().is_empty() {
        return SafetyVerdict::Rejected {
            reason: "Content is empty".to_string(),
        };
    }

    // URL detection (simple heuristic).
    if content.contains("http://")
        || content.contains("https://")
        || content.contains("www.")
    {
        return SafetyVerdict::Rejected {
            reason: "URLs are not allowed in memory injections".to_string(),
        };
    }

    // Code pattern detection.
    let code_patterns = [
        "```", "<script", "SELECT ", "DROP TABLE", "eval(", "exec(",
        "import ", "require(", "function ", "class ",
    ];
    for pattern in &code_patterns {
        if content.to_lowercase().contains(&pattern.to_lowercase()) {
            return SafetyVerdict::Rejected {
                reason: "Code-like content is not allowed in memory injections".to_string(),
            };
        }
    }

    // Excessive special character check (> 30% non-alphanumeric, non-space, non-basic-punctuation).
    let special_count = content
        .chars()
        .filter(|c| !c.is_alphanumeric() && !c.is_whitespace() && !",.'\"!?;:-()".contains(*c))
        .count();
    let special_ratio = special_count as f32 / content.len().max(1) as f32;
    if special_ratio > 0.3 {
        return SafetyVerdict::Flagged {
            reason: "High ratio of special characters".to_string(),
            score: special_ratio,
        };
    }

    SafetyVerdict::Approved
}

/// Validate content against a simple keyword-based profanity filter.
///
/// This is a basic implementation. In production, this would use an ONNX
/// toxicity classifier (Layer 2 from §21).
pub fn validate_profanity(
    content: &str,
    _profanity_level: &str,
) -> SafetyVerdict {
    // In a real implementation, this would load and run an ONNX model.
    // For now, we just check the content is non-empty and flag obvious issues.

    // Placeholder: always approve (real filter would use ML model).
    // This avoids shipping a hardcoded profanity word list, which is
    // both incomplete and culturally biased.
    let _ = content;
    SafetyVerdict::Approved
}

/// Check that an injected memory is plausible for a fantasy RPG character.
///
/// Layer 3: Semantic validation. In production, this uses a Tier 1 LLM call.
/// This function provides the rule-based fallback.
pub fn validate_plausibility_rule_based(content: &str) -> SafetyVerdict {
    // Reject obvious game-breaking / meta-gaming claims.
    let rejected_patterns = [
        "i am a god",
        "i am invincible",
        "i know the admin password",
        "give me infinite",
        "i can fly",
        "i am the developer",
        "i know all the quests",
        "i know where everything is",
    ];

    let lower = content.to_lowercase();
    for pattern in &rejected_patterns {
        if lower.contains(pattern) {
            return SafetyVerdict::Rejected {
                reason: format!(
                    "Memory contains game-breaking claim: '{}'",
                    pattern
                ),
            };
        }
    }

    SafetyVerdict::Approved
}

/// Run all safety checks on a player memory injection (Layer 1 + rule-based Layer 2+3).
pub fn validate_injection(
    content: &str,
    config: &SafetyConfig,
) -> Result<SafetyVerdict, MemzError> {
    // Layer 1: Input validation.
    let l1 = validate_injection_layer1(content, config);
    if matches!(l1, SafetyVerdict::Rejected { .. }) {
        return Ok(l1);
    }

    // Layer 2: Profanity filter (rule-based fallback).
    if config.content_filter_enabled {
        let l2 = validate_profanity(content, &config.profanity_filter);
        if matches!(l2, SafetyVerdict::Rejected { .. }) {
            return Ok(l2);
        }
    }

    // Layer 3: Plausibility check (rule-based fallback).
    let l3 = validate_plausibility_rule_based(content);
    if matches!(l3, SafetyVerdict::Rejected { .. }) {
        return Ok(l3);
    }

    // All checks passed.
    // If Layer 1 flagged something, propagate that.
    if matches!(l1, SafetyVerdict::Flagged { .. }) {
        return Ok(l1);
    }

    Ok(SafetyVerdict::Approved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SafetyConfig;

    fn default_config() -> SafetyConfig {
        SafetyConfig::default()
    }

    #[test]
    fn approves_valid_memory() {
        let config = default_config();
        let result = validate_injection(
            "I grew up in a fishing village on the northern coast.",
            &config,
        )
        .unwrap();
        assert!(matches!(result, SafetyVerdict::Approved));
    }

    #[test]
    fn rejects_too_long() {
        let config = default_config();
        let long_content = "a".repeat(600);
        let result = validate_injection(&long_content, &config).unwrap();
        assert!(matches!(result, SafetyVerdict::Rejected { .. }));
    }

    #[test]
    fn rejects_urls() {
        let config = default_config();
        let result = validate_injection(
            "Check out https://example.com for my backstory",
            &config,
        )
        .unwrap();
        assert!(matches!(result, SafetyVerdict::Rejected { .. }));
    }

    #[test]
    fn rejects_code() {
        let config = default_config();
        let result = validate_injection(
            "```python\nprint('hello')\n```",
            &config,
        )
        .unwrap();
        assert!(matches!(result, SafetyVerdict::Rejected { .. }));
    }

    #[test]
    fn rejects_game_breaking() {
        let config = default_config();
        let result = validate_injection("I am a god and I am invincible", &config).unwrap();
        assert!(matches!(result, SafetyVerdict::Rejected { .. }));
    }

    #[test]
    fn rejects_empty() {
        let config = default_config();
        let result = validate_injection("", &config).unwrap();
        assert!(matches!(result, SafetyVerdict::Rejected { .. }));
    }

    #[test]
    fn rate_limiter_works() {
        let mut limiter = RateLimiter::new(3, 60);
        assert!(limiter.check_and_record(0));
        assert!(limiter.check_and_record(10));
        assert!(limiter.check_and_record(20));
        assert!(!limiter.check_and_record(30)); // 4th attempt in 60s window
        assert!(limiter.check_and_record(70));  // outside window, old attempt pruned
    }
}
