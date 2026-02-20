//! LLM Client — unified interface for Ollama, OpenAI, and llama.cpp backends.

use std::time::{Duration, Instant};

use reqwest::Client;
use serde_json::json;
use tracing::{debug, warn};

use crate::error::LlmError;
use crate::types::{LlmRequest, LlmResponse, LlmTier};

/// Provider backend for LLM inference.
#[derive(Debug, Clone)]
pub enum LlmProvider {
    /// Ollama running locally (recommended).
    Ollama { base_url: String },
    /// OpenAI-compatible API (also works with Anthropic, Together, etc.).
    OpenAiCompatible { base_url: String, api_key: String },
    /// No LLM available — all calls return error, triggering rule-based fallback.
    None,
}

/// The main LLM client that routes requests to the appropriate backend.
pub struct LlmClient {
    provider: LlmProvider,
    http: Client,
    tier1_model: String,
    tier2_model: String,
    max_retries: u32,
}

impl LlmClient {
    /// Create a new LLM client.
    #[must_use]
    pub fn new(
        provider: LlmProvider,
        tier1_model: impl Into<String>,
        tier2_model: impl Into<String>,
        max_retries: u32,
    ) -> Self {
        Self {
            provider,
            http: Client::new(),
            tier1_model: tier1_model.into(),
            tier2_model: tier2_model.into(),
            max_retries,
        }
    }

    /// Create a client with no LLM backend (all calls fail → rule-based fallback).
    #[must_use]
    pub fn none() -> Self {
        Self {
            provider: LlmProvider::None,
            http: Client::new(),
            tier1_model: String::new(),
            tier2_model: String::new(),
            max_retries: 0,
        }
    }

    /// Generate a response from the LLM.
    ///
    /// Returns `Err` if the LLM is unavailable or all retries fail.
    /// The caller should fall back to rule-based generation on error.
    pub async fn generate(&self, request: &LlmRequest) -> Result<LlmResponse, LlmError> {
        match &self.provider {
            LlmProvider::None => {
                Err(LlmError::Unavailable("No LLM provider configured".into()))
            }
            LlmProvider::Ollama { base_url } => {
                self.generate_ollama(base_url, request).await
            }
            LlmProvider::OpenAiCompatible { base_url, api_key } => {
                self.generate_openai(base_url, api_key, request).await
            }
        }
    }

    /// Generate using Ollama's API.
    async fn generate_ollama(
        &self,
        base_url: &str,
        request: &LlmRequest,
    ) -> Result<LlmResponse, LlmError> {
        let model = match request.tier {
            LlmTier::SmallLocal => &self.tier1_model,
            LlmTier::LargeModel => &self.tier2_model,
            LlmTier::RuleBased => {
                return Err(LlmError::ConfigError(
                    "Rule-based tier does not use LLM".into(),
                ));
            }
        };

        let url = format!("{}/api/generate", base_url);
        let mut body = json!({
            "model": model,
            "prompt": format!("{}\n\n{}", request.system, request.user),
            "stream": false,
            "options": {
                "temperature": request.temperature,
                "num_predict": request.max_tokens,
            }
        });

        // Add GBNF grammar if specified (Ollama supports this as "grammar").
        if let Some(grammar) = &request.grammar {
            body["options"]["grammar"] = json!(grammar);
        }

        let mut last_error = String::new();
        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                debug!("Retrying LLM call (attempt {}/{})", attempt + 1, self.max_retries + 1);
            }

            let start = Instant::now();
            let result = self
                .http
                .post(&url)
                .json(&body)
                .timeout(Duration::from_millis(request.timeout_ms))
                .send()
                .await;

            let latency_ms = start.elapsed().as_millis() as u64;

            match result {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let json: serde_json::Value = resp
                            .json()
                            .await
                            .map_err(|e| LlmError::ParseError(e.to_string()))?;

                        let text = json["response"]
                            .as_str()
                            .unwrap_or("")
                            .to_string();

                        return Ok(LlmResponse {
                            text,
                            tokens_generated: json["eval_count"].as_u64().unwrap_or(0) as u32,
                            latency_ms,
                            model: model.clone(),
                        });
                    } else {
                        last_error = format!("HTTP {}: {}", resp.status(), resp.text().await.unwrap_or_default());
                        warn!("Ollama returned error: {}", last_error);
                    }
                }
                Err(e) => {
                    last_error = e.to_string();
                    if e.is_timeout() {
                        warn!("Ollama request timed out after {}ms", request.timeout_ms);
                    } else {
                        warn!("Ollama request failed: {}", last_error);
                    }
                }
            }
        }

        Err(LlmError::RetriesExhausted {
            attempts: self.max_retries + 1,
            last_error,
        })
    }

    /// Generate using OpenAI-compatible API.
    async fn generate_openai(
        &self,
        base_url: &str,
        api_key: &str,
        request: &LlmRequest,
    ) -> Result<LlmResponse, LlmError> {
        let model = match request.tier {
            LlmTier::SmallLocal => &self.tier1_model,
            LlmTier::LargeModel => &self.tier2_model,
            LlmTier::RuleBased => {
                return Err(LlmError::ConfigError(
                    "Rule-based tier does not use LLM".into(),
                ));
            }
        };

        let url = format!("{}/v1/chat/completions", base_url);
        let body = json!({
            "model": model,
            "messages": [
                { "role": "system", "content": request.system },
                { "role": "user", "content": request.user },
            ],
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
        });

        let mut last_error = String::new();
        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                debug!("Retrying OpenAI call (attempt {}/{})", attempt + 1, self.max_retries + 1);
            }

            let start = Instant::now();
            let result = self
                .http
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .json(&body)
                .timeout(Duration::from_millis(request.timeout_ms))
                .send()
                .await;

            let latency_ms = start.elapsed().as_millis() as u64;

            match result {
                Ok(resp) => {
                    if resp.status().is_success() {
                        let json: serde_json::Value = resp
                            .json()
                            .await
                            .map_err(|e| LlmError::ParseError(e.to_string()))?;

                        let text = json["choices"][0]["message"]["content"]
                            .as_str()
                            .unwrap_or("")
                            .to_string();

                        let tokens = json["usage"]["completion_tokens"]
                            .as_u64()
                            .unwrap_or(0) as u32;

                        return Ok(LlmResponse {
                            text,
                            tokens_generated: tokens,
                            latency_ms,
                            model: model.clone(),
                        });
                    } else {
                        last_error = format!("HTTP {}", resp.status());
                        warn!("OpenAI API returned error: {}", last_error);
                    }
                }
                Err(e) => {
                    last_error = e.to_string();
                    warn!("OpenAI API request failed: {}", last_error);
                }
            }
        }

        Err(LlmError::RetriesExhausted {
            attempts: self.max_retries + 1,
            last_error,
        })
    }

    /// Parse a raw LLM response text as structured JSON.
    ///
    /// Returns `Err` if the text is not valid JSON or doesn't match the expected type.
    pub fn parse_structured<T: serde::de::DeserializeOwned>(
        &self,
        response: &LlmResponse,
    ) -> Result<T, LlmError> {
        serde_json::from_str(&response.text)
            .map_err(|e| LlmError::ParseError(format!("JSON parse error: {} — raw text: '{}'", e, response.text)))
    }

    /// Check if the LLM client has a backend configured.
    #[must_use]
    pub fn is_available(&self) -> bool {
        !matches!(self.provider, LlmProvider::None)
    }
}
