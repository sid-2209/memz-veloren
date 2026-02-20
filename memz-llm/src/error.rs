//! LLM error types.

use thiserror::Error;

/// Errors that can occur during LLM operations.
#[derive(Debug, Error)]
pub enum LlmError {
    /// HTTP request failed.
    #[error("LLM request failed: {0}")]
    RequestFailed(String),

    /// LLM response was not valid JSON.
    #[error("Failed to parse LLM response as JSON: {0}")]
    ParseError(String),

    /// LLM response did not match expected schema.
    #[error("LLM output schema validation failed: {0}")]
    SchemaValidation(String),

    /// Request timed out.
    #[error("LLM request timed out after {0}ms")]
    Timeout(u64),

    /// LLM provider is unavailable.
    #[error("LLM provider unavailable: {0}")]
    Unavailable(String),

    /// All retry attempts exhausted.
    #[error("All LLM retry attempts exhausted after {attempts} tries: {last_error}")]
    RetriesExhausted {
        attempts: u32,
        last_error: String,
    },

    /// Configuration error.
    #[error("LLM configuration error: {0}")]
    ConfigError(String),
}

impl From<reqwest::Error> for LlmError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            LlmError::Timeout(0)
        } else if err.is_connect() {
            LlmError::Unavailable(err.to_string())
        } else {
            LlmError::RequestFailed(err.to_string())
        }
    }
}
