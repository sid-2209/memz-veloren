//! LLM inference engine for NPC dialogue generation.
//!
//! Uses Ollama's HTTP API for streaming model inference. This approach
//! avoids the llama-cpp-2 crate's GGUF tensor duplication bug and
//! provides reliable, high-quality inference with any model.

use crate::conversation::ConversationHistory;
use crate::error::{Result, VoiceError};

use serde::{Deserialize, Serialize};

/// Configuration for the LLM engine.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    /// Ollama server URL (default: http://localhost:11434)
    pub ollama_url: String,
    /// Model name in ollama (e.g., "llama3.2:1b", "llama3.2:3b")
    pub model_name: String,
    /// Temperature for response generation (0.0-2.0).
    pub temperature: f32,
    /// Maximum tokens to generate per response.
    pub max_tokens: i32,
    /// Seed for reproducible generation (None = random).
    pub seed: Option<u32>,
    /// Context window size for the model.
    pub context_size: u32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            ollama_url: "http://localhost:11434".to_string(),
            model_name: "llama3.2:3b".to_string(), // 3b gives dramatically better dialogue vs 1b
            temperature: 0.8,  // Slight creativity for NPC personality variance
            max_tokens: 80,    // Keep responses tight — 1-2 sentences for voice
            seed: None,
            context_size: 2048,
        }
    }
}

/// Context about an NPC for building the system prompt.
#[derive(Debug, Clone)]
pub struct NpcContext {
    /// NPC's name (e.g., "Kira").
    pub name: String,
    /// NPC's profession/role (e.g., "Guard", "Merchant").
    pub profession: String,
    /// Name of the NPC's current location/town.
    pub location: String,
    /// NPC's faction or affiliation.
    pub faction: String,
    /// Personality traits as a formatted string.
    pub personality: String,
    /// Current mood/emotion descriptor.
    pub mood: String,
    /// Recent memories or knowledge.
    pub knowledge: String,
    /// How the NPC feels about the player.
    pub player_sentiment: String,
}

impl NpcContext {
    /// Build the system prompt for the LLM.
    pub fn to_system_prompt(&self) -> String {
        format!(
            "You are {name}, a {profession} living in {location}, in the world of Veloren.\n\
             \n\
             WORLD CONTEXT — Veloren is a low-fantasy voxel RPG world:\n\
             - A dangerous world of mountains, plains, forests, deserts and dungeons\n\
             - Settlements are small communities under constant threat from beasts and bandits\n\
             - The player is an Adventurer — a wandering fighter who explores and takes on threats\n\
             - Common creatures: wolves, goblins, ogres, undead, cultists, dangerous wildlife\n\
             - There are no modern inventions — only swords, bows, magic, and crafted goods\n\
             - Trade between settlements happens via merchants and caravans\n\
             - Local concerns: food, safety, crafting supplies, relationships, trade\n\
             \n\
             YOUR CHARACTER:\n\
             {faction}\
             Personality: {personality}\n\
             Current mood: {mood}\n\
             How you see this Adventurer: {player_sentiment}\n\
             {knowledge}\
             \n\
             SPEECH RULES (critical — follow exactly):\n\
             - Respond in 1-2 short sentences only — voice dialogue must be brief\n\
             - Speak naturally as {name} — use contractions, emotion, and personality\n\
             - React directly to what the Adventurer said — be specific, not generic\n\
             - Use vocabulary fitting a {profession} in a medieval fantasy village\n\
             - Never break character, mention AI, or use modern references\n\
             - No emojis, no markdown, no lists — plain conversational speech only\n\
             - If asked about yourself: mention your name, role, and a personal detail\n\
             - If asked about danger/quests: share local knowledge about threats or opportunities\n\
             - If asked about trade/goods: respond as your profession would",
            name = self.name,
            profession = self.profession,
            location = self.location,
            faction = if self.faction.is_empty() {
                String::new()
            } else {
                format!("Faction/Group: {}\n", self.faction)
            },
            personality = self.personality,
            mood = self.mood,
            player_sentiment = self.player_sentiment,
            knowledge = if self.knowledge.is_empty() {
                String::new()
            } else {
                format!("What you know and care about:\n{}\n", self.knowledge)
            },
        )
    }

    /// Build ollama chat messages from context, history, and current input.
    fn build_messages(
        &self,
        player_input: &str,
        history: Option<&ConversationHistory>,
    ) -> Vec<OllamaMessage> {
        let mut messages = vec![
            OllamaMessage {
                role: "system".to_string(),
                content: self.to_system_prompt(),
            },
        ];

        // Add conversation history
        if let Some(history) = history {
            for (player, npc) in history.to_prompt_messages() {
                messages.push(OllamaMessage {
                    role: "user".to_string(),
                    content: player.to_string(),
                });
                messages.push(OllamaMessage {
                    role: "assistant".to_string(),
                    content: npc.to_string(),
                });
            }
        }

        // Add current player input
        messages.push(OllamaMessage {
            role: "user".to_string(),
            content: player_input.to_string(),
        });

        messages
    }
}

/// Ollama API message format.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

/// Ollama chat request.
#[derive(Debug, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    options: OllamaOptions,
}

/// Ollama generation options.
#[derive(Debug, Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    seed: Option<u32>,
    num_ctx: u32,
}

/// Ollama chat response (non-streaming).
#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: OllamaMessage,
    #[serde(default)]
    done: bool,
}

/// Ollama streaming response chunk.
#[derive(Debug, Deserialize)]
struct OllamaStreamChunk {
    #[serde(default)]
    message: Option<OllamaMessage>,
    #[serde(default)]
    done: bool,
}

/// SOTA LLM inference engine using Ollama HTTP API.
///
/// Connects to a local Ollama server for model inference with streaming
/// token generation. Falls back to placeholder responses when Ollama
/// is unavailable.
pub struct DialogueLlm {
    config: LlmConfig,
    client: reqwest::blocking::Client,
    /// Whether ollama connection has been verified.
    ollama_available: Option<bool>,
}

impl DialogueLlm {
    /// Create a new LLM engine.
    pub fn new(config: LlmConfig) -> Result<Self> {
        log::info!(
            "Initializing LLM engine (ollama model: {})...",
            config.model_name
        );
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| VoiceError::LlmError(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            config,
            client,
            ollama_available: None,
        })
    }

    /// Create with default settings.
    pub fn with_defaults() -> Result<Self> {
        Self::new(LlmConfig::default())
    }

    /// Check if Ollama server is available.
    fn check_ollama(&mut self) -> bool {
        if let Some(available) = self.ollama_available {
            return available;
        }

        let url = format!("{}/api/tags", self.config.ollama_url);
        let available = self.client.get(&url).send().map_or(false, |resp| {
            if resp.status().is_success() {
                log::info!("Ollama server connected at {}", self.config.ollama_url);
                true
            } else {
                log::warn!("Ollama server returned status {}", resp.status());
                false
            }
        });

        self.ollama_available = Some(available);
        available
    }

    /// Generate a complete NPC response (blocking).
    pub fn generate_response(
        &mut self,
        npc_context: &NpcContext,
        player_input: &str,
        history: Option<&ConversationHistory>,
    ) -> Result<String> {
        let start_time = std::time::Instant::now();
        log::info!(
            "Generating response for {} to: \"{}\"",
            npc_context.name, player_input
        );

        let response = if self.check_ollama() {
            self.generate_with_ollama(npc_context, player_input, history)?
        } else {
            self.generate_placeholder_response(npc_context, player_input, history)
        };

        let elapsed = start_time.elapsed();
        log::info!(
            "LLM response generated in {:.0}ms: \"{}\"",
            elapsed.as_millis(), response
        );
        Ok(response)
    }

    /// Generate response using Ollama HTTP API (non-streaming).
    fn generate_with_ollama(
        &self,
        npc_context: &NpcContext,
        player_input: &str,
        history: Option<&ConversationHistory>,
    ) -> Result<String> {
        let messages = npc_context.build_messages(player_input, history);
        let url = format!("{}/api/chat", self.config.ollama_url);

        let request = OllamaChatRequest {
            model: self.config.model_name.clone(),
            messages,
            stream: false,
            options: OllamaOptions {
                temperature: self.config.temperature,
                num_predict: self.config.max_tokens,
                seed: self.config.seed,
                num_ctx: self.config.context_size,
            },
        };

        log::debug!("Sending chat request to {}", url);

        let resp = self.client
            .post(&url)
            .json(&request)
            .send()
            .map_err(|e| VoiceError::LlmError(format!("Ollama request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(VoiceError::LlmError(
                format!("Ollama returned {}: {}", status, body)
            ));
        }

        let chat_resp: OllamaChatResponse = resp
            .json()
            .map_err(|e| VoiceError::LlmError(format!("Failed to parse response: {}", e)))?;

        let response = chat_resp.message.content.trim().to_string();
        let response = truncate_at_sentence(&response, 300);
        Ok(response)
    }

    /// Generate response with streaming token output.
    ///
    /// Calls `on_token` for each accumulated token string. Returns `false` to cancel.
    pub fn generate_response_streaming<F>(
        &mut self,
        npc_context: &NpcContext,
        player_input: &str,
        history: Option<&ConversationHistory>,
        mut on_token: F,
    ) -> Result<String>
    where
        F: FnMut(&str) -> bool,
    {
        let start_time = std::time::Instant::now();
        log::info!(
            "Streaming response for {} to: \"{}\"",
            npc_context.name, player_input
        );

        if self.check_ollama() {
            let response = self.stream_with_ollama(npc_context, player_input, history, &mut on_token)?;
            let elapsed = start_time.elapsed();
            log::info!("LLM streaming completed in {:.0}ms", elapsed.as_millis());
            Ok(response)
        } else {
            // Placeholder streaming
            let full = self.generate_placeholder_response(npc_context, player_input, history);
            let mut accumulated = String::new();
            for word in full.split_whitespace() {
                if !accumulated.is_empty() {
                    accumulated.push(' ');
                }
                accumulated.push_str(word);
                if !on_token(&accumulated) {
                    break;
                }
            }
            Ok(accumulated)
        }
    }

    /// Stream tokens from Ollama HTTP API — true line-by-line streaming.
    ///
    /// Uses `BufReader` on the response body so `on_token` is called as each
    /// JSON line arrives from Ollama, not after the full body is buffered.
    /// This is what enables the sentence-level TTS pipeline in lib.rs to start
    /// synthesizing the first sentence while Ollama is still generating the rest.
    fn stream_with_ollama<F>(
        &self,
        npc_context: &NpcContext,
        player_input: &str,
        history: Option<&ConversationHistory>,
        on_token: &mut F,
    ) -> Result<String>
    where
        F: FnMut(&str) -> bool,
    {
        let messages = npc_context.build_messages(player_input, history);
        let url = format!("{}/api/chat", self.config.ollama_url);

        let request = OllamaChatRequest {
            model: self.config.model_name.clone(),
            messages,
            stream: true,
            options: OllamaOptions {
                temperature: self.config.temperature,
                num_predict: self.config.max_tokens,
                seed: self.config.seed,
                num_ctx: self.config.context_size,
            },
        };

        let resp: reqwest::blocking::Response = self.client
            .post(&url)
            .json(&request)
            .send()
            .map_err(|e| VoiceError::LlmError(format!("Ollama streaming request failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(VoiceError::LlmError(
                format!("Ollama returned {}: {}", status, body)
            ));
        }

        // True streaming: read each NDJSON line as it arrives from Ollama.
        // reqwest::blocking::Response implements std::io::Read, so BufReader
        // gives us line-by-line access without waiting for the full body.
        use std::io::BufRead;
        let reader = std::io::BufReader::new(resp);
        let mut output = String::new();

        for line_result in reader.lines() {
            let line = match line_result {
                Ok(l) => l,
                Err(e) => {
                    log::warn!("Streaming read error: {}", e);
                    break;
                }
            };

            let line = line.trim().to_string();
            if line.is_empty() {
                continue;
            }

            match serde_json::from_str::<OllamaStreamChunk>(&line) {
                Ok(chunk) => {
                    if let Some(msg) = chunk.message {
                        output.push_str(&msg.content);
                        if !on_token(&output) {
                            log::info!("Streaming cancelled by consumer");
                            break;
                        }
                    }
                    if chunk.done {
                        break;
                    }
                }
                Err(e) => {
                    log::warn!("Failed to parse stream chunk: {} (line: {})", e, &line[..line.len().min(100)]);
                }
            }
        }

        let response = truncate_at_sentence(output.trim(), 300);
        Ok(response)
    }

    /// Generate a contextual placeholder response (no model / ollama unavailable).
    fn generate_placeholder_response(
        &self,
        npc: &NpcContext,
        player_input: &str,
        history: Option<&ConversationHistory>,
    ) -> String {
        let input_lower = player_input.to_lowercase();
        let has_history = history.map_or(false, |h| !h.is_empty());

        if input_lower.contains("hello") || input_lower.contains("hi ") || input_lower.contains("hey") {
            match npc.profession.to_lowercase().as_str() {
                "guard" => format!(
                    "Greetings, traveler. Welcome to {}. Keep your weapons sheathed within the walls.",
                    npc.location
                ),
                "merchant" | "trader" => format!(
                    "Well met! I'm {}, finest {} in all of {}. Looking to trade?",
                    npc.name, npc.profession.to_lowercase(), npc.location
                ),
                "blacksmith" => format!(
                    "Aye, what do you need? I forge the strongest blades in {}.",
                    npc.location
                ),
                _ => format!(
                    "Hello there. I'm {}, a humble {} here in {}.",
                    npc.name, npc.profession.to_lowercase(), npc.location
                ),
            }
        } else if input_lower.contains("quest") || input_lower.contains("help") || input_lower.contains("task") {
            format!(
                "Hmm, let me think. There might be something you could help with around {}. \
                 Ask around, and keep your eyes open for trouble.",
                npc.location
            )
        } else if input_lower.contains("trade") || input_lower.contains("buy") || input_lower.contains("sell") {
            "I deal in quality goods, friend. Take a look at what I have, \
             and we can work out a fair price.".to_string()
        } else if input_lower.contains("danger") || input_lower.contains("monster") || input_lower.contains("enemy") {
            format!(
                "Be careful out there. The wilds beyond {} aren't safe. \
                 Creatures lurk in the forests and caves.",
                npc.location
            )
        } else if input_lower.contains("name") || input_lower.contains("who are you") {
            format!(
                "I am {}, a {} of {}. I've lived here for many seasons now.",
                npc.name, npc.profession.to_lowercase(), npc.location
            )
        } else if input_lower.contains("bye") || input_lower.contains("goodbye") || input_lower.contains("farewell") {
            "Safe travels, friend. May the road treat you well.".to_string()
        } else if has_history {
            "I see. Well, there's always more to learn about these lands. \
             Is there anything else you wish to know?".to_string()
        } else {
            format!(
                "That's an interesting thought. Life in {} keeps us all busy, \
                 but I'm happy to chat when I can.",
                npc.location
            )
        }
    }
}

/// Truncate text at the last complete sentence within the character limit.
fn truncate_at_sentence(text: &str, max_chars: usize) -> String {
    if text.len() <= max_chars {
        return text.to_string();
    }

    let truncated = &text[..max_chars];
    if let Some(pos) = truncated.rfind(|c| c == '.' || c == '!' || c == '?') {
        truncated[..=pos].to_string()
    } else {
        if let Some(pos) = truncated.rfind(' ') {
            format!("{}.", &truncated[..pos])
        } else {
            format!("{}.", truncated)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_prompt_generation() {
        let ctx = NpcContext {
            name: "Thorin".to_string(),
            profession: "Blacksmith".to_string(),
            location: "Ironforge".to_string(),
            faction: "Mountain Clan".to_string(),
            personality: "Gruff but kind-hearted".to_string(),
            mood: "content".to_string(),
            knowledge: String::new(),
            player_sentiment: "neutral stranger".to_string(),
        };
        let prompt = ctx.to_system_prompt();
        assert!(prompt.contains("Thorin"));
        assert!(prompt.contains("Blacksmith"));
        assert!(prompt.contains("1-3 sentences"));
    }

    #[test]
    fn test_placeholder_response() {
        let mut llm = DialogueLlm::with_defaults().unwrap();
        // Force placeholder mode by setting ollama as unavailable
        llm.ollama_available = Some(false);
        let ctx = NpcContext {
            name: "Kira".to_string(),
            profession: "Guard".to_string(),
            location: "Stonehaven".to_string(),
            faction: String::new(),
            personality: "Strict and dutiful".to_string(),
            mood: "alert".to_string(),
            knowledge: String::new(),
            player_sentiment: "unknown".to_string(),
        };
        let response = llm.generate_response(&ctx, "Hello!", None).unwrap();
        assert!(!response.is_empty());
        assert!(response.contains("Stonehaven"));
    }

    #[test]
    fn test_streaming_placeholder() {
        let mut llm = DialogueLlm::with_defaults().unwrap();
        llm.ollama_available = Some(false);
        let ctx = NpcContext {
            name: "Test".to_string(),
            profession: "Merchant".to_string(),
            location: "Town".to_string(),
            faction: String::new(),
            personality: "Friendly".to_string(),
            mood: "happy".to_string(),
            knowledge: String::new(),
            player_sentiment: "friendly".to_string(),
        };
        let mut token_count = 0;
        let result = llm.generate_response_streaming(&ctx, "Hi there!", None, |_| {
            token_count += 1;
            true
        });
        assert!(result.is_ok());
        assert!(token_count > 0);
    }

    #[test]
    fn test_truncate_at_sentence() {
        let text = "Hello world. This is a test. And another sentence here.";
        let truncated = truncate_at_sentence(text, 30);
        assert!(truncated.ends_with('.'));
        assert!(truncated.len() <= 30);
    }

    #[test]
    fn test_message_building() {
        let ctx = NpcContext {
            name: "Test".to_string(),
            profession: "Guard".to_string(),
            location: "Town".to_string(),
            faction: String::new(),
            personality: "Stern".to_string(),
            mood: "neutral".to_string(),
            knowledge: String::new(),
            player_sentiment: "neutral".to_string(),
        };
        let messages = ctx.build_messages("Hello!", None);
        assert_eq!(messages.len(), 2); // system + user
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "user");
        assert_eq!(messages[1].content, "Hello!");
    }

    #[test]
    fn test_message_building_with_history() {
        let ctx = NpcContext {
            name: "Test".to_string(),
            profession: "Guard".to_string(),
            location: "Town".to_string(),
            faction: String::new(),
            personality: "Stern".to_string(),
            mood: "neutral".to_string(),
            knowledge: String::new(),
            player_sentiment: "neutral".to_string(),
        };
        let mut history = ConversationHistory::new(1, 6);
        history.add_exchange("Previous question".to_string(), "Previous answer".to_string());

        let messages = ctx.build_messages("New question", Some(&history));
        assert_eq!(messages.len(), 4); // system + history(user+assistant) + user
        assert_eq!(messages[0].role, "system");
        assert_eq!(messages[1].role, "user");
        assert_eq!(messages[1].content, "Previous question");
        assert_eq!(messages[2].role, "assistant");
        assert_eq!(messages[2].content, "Previous answer");
        assert_eq!(messages[3].role, "user");
        assert_eq!(messages[3].content, "New question");
    }
}
