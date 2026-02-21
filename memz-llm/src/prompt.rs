//! Prompt templates for MEMZ LLM operations (§12.3.1).
//!
//! Every prompt is a versioned, testable artifact.
//! In production, these would be loaded from TOML files.
//! This module provides the default built-in templates.

/// Dialogue prompt template for simple NPC interactions (Tier 1).
pub const DIALOGUE_SIMPLE_SYSTEM: &str = r"You are {npc_name}, a {npc_profession} in {settlement_name}.
Your personality: {personality_description}.
Your current emotional state: {pad_state}.

RULES:
- Stay in character. Never break the fourth wall.
- Reference memories naturally — don't list them.
- Keep responses under 3 sentences.
- If you don't remember the player, say so honestly.
- Your response must be valid JSON.";

pub const DIALOGUE_SIMPLE_USER: &str = r#"Context: {context_description}
Player action: {player_action}

Your relevant memories (ranked by importance):
{memories_formatted}

Your current opinion of this player: {overall_sentiment} (confidence: {confidence})

Respond as {npc_name} would. Return JSON:
{{"dialogue": "your response", "emotion_shift": <float -1.0 to 1.0>, "new_memory": "what you'll remember about this"}}"#;

/// Deep reflection prompt (Tier 2).
pub const REFLECTION_SYSTEM: &str = r"You are the inner mind of {npc_name}, a {npc_profession}.
You are reflecting on your recent experiences during a quiet moment.
Think deeply. Consider patterns. Form opinions. Wonder about things.
You are NOT speaking to anyone — this is your private thought.";

pub const REFLECTION_USER: &str = r#"Your recent episodic memories (last {time_window}):
{recent_episodic_formatted}

Your existing beliefs and knowledge:
{semantic_formatted}

Your personality traits: {personality_summary}

Based on these experiences, what do you think? What patterns do you notice?
What has changed in your view of the world or the people around you?

Return JSON:
{{"reflection": "your inner thought", "new_beliefs": ["belief1", ...], "questions": ["thing you wonder about", ...], "mood_shift": {{"pleasure": <float>, "arousal": <float>, "dominance": <float>}}}}"#;

/// Gossip generation prompt (Tier 1).
pub const GOSSIP_SYSTEM: &str = r"You are {npc_name}, a {npc_profession}.
You are chatting with {listener_name} about recent events.
Share information naturally — as gossip, not as a report.
Your personality affects how you tell stories: {personality_description}.";

pub const GOSSIP_USER: &str = r#"You want to tell {listener_name} about:
{memory_to_share}

How confident are you in this information? {confidence}
Did you witness this yourself or hear it from someone? {source_type}

Tell them about it in character. Return JSON:
{{"gossip_text": "what you say", "confidence": <float 0.0-1.0>, "embellished": <bool>}}"#;

/// Bard composition prompt (Tier 2).
pub const BARD_SYSTEM: &str = r"You are {bard_name}, a wandering bard in {settlement_name}.
Your style is {bard_style}: {style_description}.
Compose a short song or poem (4-8 lines) about the events described.
Use a consistent rhyme scheme (AABB or ABAB).
The song should be memorable and fun to share.";

pub const BARD_USER: &str = r#"The events to compose about:
{events_formatted}

The most dramatic moment: {dramatic_moment}
The main character of the song: {main_character}

Compose your song. Return JSON:
{{"title": "song title", "verses": ["line 1", "line 2", ...], "style": "{bard_style}"}}"#;

/// Memory injection validation prompt (Tier 1).
pub const INJECTION_VALIDATION_SYSTEM: &str = r"You are a content validator for a fantasy RPG game.
Your job is to determine if a player's backstory memory is:
1. Plausible for a fantasy character
2. Not game-breaking or meta-gaming
3. Safe and appropriate

You must be generous — creative backstories are encouraged.";

pub const INJECTION_VALIDATION_USER: &str = r#"Player submitted this backstory memory:
"{memory_content}"

Is this a plausible personal memory for a fantasy RPG character?
Return JSON:
{{"approved": <bool>, "reason": "why approved/rejected", "suggested_edit": "optional improved version or null"}}"#;

/// GBNF grammar for structured dialogue output.
pub const DIALOGUE_GRAMMAR: &str = r#"root   ::= "{" ws "\"dialogue\"" ws ":" ws string "," ws "\"emotion_shift\"" ws ":" ws number "," ws "\"new_memory\"" ws ":" ws string "}" ws
string ::= "\"" ([^"\\] | "\\" .)* "\""
number ::= "-"? [0-1] ("." [0-9]{1,2})?
ws     ::= [ \t\n]*"#;

/// GBNF grammar for structured reflection output.
pub const REFLECTION_GRAMMAR: &str = r#"root   ::= "{" ws "\"reflection\"" ws ":" ws string "," ws "\"new_beliefs\"" ws ":" ws array "," ws "\"questions\"" ws ":" ws array "," ws "\"mood_shift\"" ws ":" ws mood "}" ws
string ::= "\"" ([^"\\] | "\\" .)* "\""
array  ::= "[" ws (string ("," ws string)*)? ws "]"
mood   ::= "{" ws "\"pleasure\"" ws ":" ws float "," ws "\"arousal\"" ws ":" ws float "," ws "\"dominance\"" ws ":" ws float "}" ws
float  ::= "-"? [0-1] ("." [0-9]{1,2})?
ws     ::= [ \t\n]*"#;

/// GBNF grammar for gossip output.
pub const GOSSIP_GRAMMAR: &str = r#"root   ::= "{" ws "\"gossip_text\"" ws ":" ws string "," ws "\"confidence\"" ws ":" ws float "," ws "\"embellished\"" ws ":" ws bool "}" ws
string ::= "\"" ([^"\\] | "\\" .)* "\""
float  ::= "0" ("." [0-9]{1,2})? | "1" ("." "0"{1,2})?
bool   ::= "true" | "false"
ws     ::= [ \t\n]*"#;

/// Simple template interpolation for prompts.
///
/// Replaces `{key}` with the corresponding value.
#[must_use]
pub fn render_template(template: &str, vars: &[(&str, &str)]) -> String {
    let mut result = template.to_string();
    for (key, value) in vars {
        result = result.replace(&format!("{{{key}}}"), value);
    }
    result
}

// ---------------------------------------------------------------------------
// PromptEngine — Versioned TOML Template Loader (§12.3.1)
// ---------------------------------------------------------------------------

use serde::Deserialize;
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// Identifies a prompt template by purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PromptId {
    /// Simple NPC dialogue (Tier 1).
    DialogueSimple,
    /// Complex multi-turn NPC dialogue (Tier 2).
    DialogueComplex,
    /// NPC reflection on recent experiences (Tier 2).
    Reflection,
    /// Gossip generation between NPCs (Tier 1).
    GossipGeneration,
    /// Vivid retelling of an emotional memory (Tier 1).
    MemoryReplay,
    /// Bard song/poem composition (Tier 2).
    BardComposition,
    /// Player memory injection validation (Tier 1).
    InjectionValidation,
    /// Memory bank summarization (Tier 1).
    MemorySummary,
}

impl PromptId {
    /// Returns the TOML filename (without path) for this prompt.
    #[must_use]
    pub fn filename(self) -> &'static str {
        match self {
            Self::DialogueSimple => "dialogue_simple.toml",
            Self::DialogueComplex => "dialogue_complex.toml",
            Self::Reflection => "reflection.toml",
            Self::GossipGeneration => "gossip_generation.toml",
            Self::MemoryReplay => "memory_replay.toml",
            Self::BardComposition => "bard_composition.toml",
            Self::InjectionValidation => "injection_validation.toml",
            Self::MemorySummary => "memory_summary.toml",
        }
    }

    /// All prompt IDs.
    #[must_use]
    pub fn all() -> &'static [PromptId] {
        &[
            Self::DialogueSimple,
            Self::DialogueComplex,
            Self::Reflection,
            Self::GossipGeneration,
            Self::MemoryReplay,
            Self::BardComposition,
            Self::InjectionValidation,
            Self::MemorySummary,
        ]
    }
}

impl fmt::Display for PromptId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::DialogueSimple => "dialogue_simple",
            Self::DialogueComplex => "dialogue_complex",
            Self::Reflection => "reflection",
            Self::GossipGeneration => "gossip_generation",
            Self::MemoryReplay => "memory_replay",
            Self::BardComposition => "bard_composition",
            Self::InjectionValidation => "injection_validation",
            Self::MemorySummary => "memory_summary",
        };
        write!(f, "{name}")
    }
}

impl FromStr for PromptId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "dialogue_simple" => Ok(Self::DialogueSimple),
            "dialogue_complex" => Ok(Self::DialogueComplex),
            "reflection" => Ok(Self::Reflection),
            "gossip_generation" => Ok(Self::GossipGeneration),
            "memory_replay" => Ok(Self::MemoryReplay),
            "bard_composition" => Ok(Self::BardComposition),
            "injection_validation" => Ok(Self::InjectionValidation),
            "memory_summary" => Ok(Self::MemorySummary),
            _ => Err(format!("unknown prompt id: '{s}'")),
        }
    }
}

/// Metadata and templates parsed from a TOML prompt file.
#[derive(Debug, Clone, Deserialize)]
struct TomlPromptFile {
    prompt: TomlPromptData,
}

/// Inner `[prompt]` section of a TOML file.
#[derive(Debug, Clone, Deserialize)]
struct TomlPromptData {
    version: String,
    tier: u8,
    max_tokens: u32,
    temperature: f32,
    #[serde(default)]
    grammar: String,
    system: String,
    user: String,
}

/// A loaded, ready-to-render prompt template.
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    /// Prompt version string (e.g., "1.0").
    pub version: String,
    /// LLM tier required (1 = small local, 2 = large local/cloud).
    pub tier: u8,
    /// Maximum output tokens.
    pub max_tokens: u32,
    /// Sampling temperature.
    pub temperature: f32,
    /// GBNF grammar filename for structured output.
    pub grammar: String,
    /// System prompt template (contains `{key}` placeholders).
    pub system: String,
    /// User prompt template (contains `{key}` placeholders).
    pub user: String,
}

/// Engine that loads versioned TOML prompt templates and renders them.
///
/// # Example
///
/// ```no_run
/// use memz_llm::prompt::{PromptEngine, PromptId};
///
/// let engine = PromptEngine::from_directory("memz-llm/prompts/v1").unwrap();
/// let tpl = engine.get(PromptId::DialogueSimple).unwrap();
/// let system = memz_llm::prompt::render_template(
///     &tpl.system,
///     &[("npc_name", "Goran"), ("npc_profession", "Blacksmith")],
/// );
/// ```
#[derive(Debug, Clone)]
pub struct PromptEngine {
    templates: HashMap<PromptId, PromptTemplate>,
}

impl PromptEngine {
    /// Create a `PromptEngine` pre-loaded with the built-in constant templates.
    ///
    /// This uses the compiled-in templates (the `const` strings in this module)
    /// and does not require any files on disk.
    #[must_use]
    pub fn builtin() -> Self {
        let mut templates = HashMap::new();

        // Dialogue Simple
        templates.insert(PromptId::DialogueSimple, PromptTemplate {
            version: "builtin".into(),
            tier: 1,
            max_tokens: 150,
            temperature: 0.7,
            grammar: "dialogue_response.gbnf".into(),
            system: DIALOGUE_SIMPLE_SYSTEM.into(),
            user: DIALOGUE_SIMPLE_USER.into(),
        });

        // Reflection
        templates.insert(PromptId::Reflection, PromptTemplate {
            version: "builtin".into(),
            tier: 2,
            max_tokens: 300,
            temperature: 0.8,
            grammar: "reflection_output.gbnf".into(),
            system: REFLECTION_SYSTEM.into(),
            user: REFLECTION_USER.into(),
        });

        // Gossip
        templates.insert(PromptId::GossipGeneration, PromptTemplate {
            version: "builtin".into(),
            tier: 1,
            max_tokens: 150,
            temperature: 0.7,
            grammar: "gossip_output.gbnf".into(),
            system: GOSSIP_SYSTEM.into(),
            user: GOSSIP_USER.into(),
        });

        // Bard
        templates.insert(PromptId::BardComposition, PromptTemplate {
            version: "builtin".into(),
            tier: 2,
            max_tokens: 300,
            temperature: 0.9,
            grammar: "bard_poem.gbnf".into(),
            system: BARD_SYSTEM.into(),
            user: BARD_USER.into(),
        });

        // Injection Validation
        templates.insert(PromptId::InjectionValidation, PromptTemplate {
            version: "builtin".into(),
            tier: 1,
            max_tokens: 100,
            temperature: 0.3,
            grammar: String::new(),
            system: INJECTION_VALIDATION_SYSTEM.into(),
            user: INJECTION_VALIDATION_USER.into(),
        });

        Self { templates }
    }

    /// Load prompt templates from a directory of TOML files.
    ///
    /// Each TOML file must match a known [`PromptId`] filename.
    /// Unknown files are ignored.
    ///
    /// # Errors
    ///
    /// Returns an error if a TOML file exists but cannot be parsed.
    pub fn from_directory(dir: impl AsRef<Path>) -> Result<Self, String> {
        let dir = dir.as_ref();
        let mut templates = HashMap::new();

        for id in PromptId::all() {
            let path: PathBuf = dir.join(id.filename());
            if path.exists() {
                let content = std::fs::read_to_string(&path)
                    .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
                let parsed: TomlPromptFile = toml::from_str(&content)
                    .map_err(|e| format!("failed to parse {}: {e}", path.display()))?;

                let d = parsed.prompt;
                templates.insert(*id, PromptTemplate {
                    version: d.version,
                    tier: d.tier,
                    max_tokens: d.max_tokens,
                    temperature: d.temperature,
                    grammar: d.grammar,
                    system: d.system,
                    user: d.user,
                });
            }
        }

        if templates.is_empty() {
            return Err(format!(
                "no prompt templates found in directory: {}",
                dir.display()
            ));
        }

        Ok(Self { templates })
    }

    /// Get a loaded prompt template by ID.
    #[must_use]
    pub fn get(&self, id: PromptId) -> Option<&PromptTemplate> {
        self.templates.get(&id)
    }

    /// Render both system and user prompts for a given ID.
    ///
    /// Returns `(system_prompt, user_prompt)` with all `{key}` placeholders
    /// replaced. Jinja-style `{%- for %}` loops in TOML templates must be
    /// pre-expanded by the caller into the vars (e.g., pass the full
    /// formatted list as a single variable).
    ///
    /// # Errors
    ///
    /// Returns an error if the prompt ID is not loaded.
    pub fn render(
        &self,
        id: PromptId,
        vars: &[(&str, &str)],
    ) -> Result<(String, String), String> {
        let tpl = self.get(id).ok_or_else(|| {
            format!("prompt template '{id}' not loaded")
        })?;

        let system = render_template(&tpl.system, vars);
        let user = render_template(&tpl.user, vars);
        Ok((system, user))
    }

    /// Number of loaded templates.
    #[must_use]
    pub fn len(&self) -> usize {
        self.templates.len()
    }

    /// Whether no templates are loaded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }

    /// List all loaded prompt IDs.
    #[must_use]
    pub fn loaded_ids(&self) -> Vec<PromptId> {
        self.templates.keys().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_rendering_works() {
        let rendered = render_template(
            "Hello {name}, you are a {role}.",
            &[("name", "Goran"), ("role", "blacksmith")],
        );
        assert_eq!(rendered, "Hello Goran, you are a blacksmith.");
    }

    #[test]
    fn template_handles_missing_vars() {
        let rendered = render_template("Hello {name}, {unknown}.", &[("name", "Goran")]);
        assert_eq!(rendered, "Hello Goran, {unknown}.");
    }

    #[test]
    fn prompt_id_from_str_round_trip() {
        for id in PromptId::all() {
            let s = id.to_string();
            let parsed: PromptId = s.parse().expect("should parse");
            assert_eq!(*id, parsed);
        }
    }

    #[test]
    fn prompt_id_unknown_returns_err() {
        assert!("nonexistent".parse::<PromptId>().is_err());
    }

    #[test]
    fn builtin_engine_has_templates() {
        let engine = PromptEngine::builtin();
        assert!(!engine.is_empty());
        assert!(engine.get(PromptId::DialogueSimple).is_some());
        assert!(engine.get(PromptId::Reflection).is_some());
        assert!(engine.get(PromptId::GossipGeneration).is_some());
        assert!(engine.get(PromptId::BardComposition).is_some());
        assert!(engine.get(PromptId::InjectionValidation).is_some());
    }

    #[test]
    fn builtin_engine_renders() {
        let engine = PromptEngine::builtin();
        let (system, _user) = engine
            .render(
                PromptId::DialogueSimple,
                &[
                    ("npc_name", "Goran"),
                    ("npc_profession", "Blacksmith"),
                    ("settlement_name", "Ironhaven"),
                    ("personality_description", "gruff"),
                    ("pad_state", "P=0.4 A=0.1 D=0.3"),
                ],
            )
            .expect("render should succeed");
        assert!(system.contains("Goran"));
        assert!(system.contains("Blacksmith"));
        assert!(!system.contains("{npc_name}"));
    }

    #[test]
    fn from_directory_loads_toml_files() {
        // This test only runs if the prompts directory exists
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/prompts/v1");
        if std::path::Path::new(dir).exists() {
            let engine = PromptEngine::from_directory(dir).expect("should load");
            assert!(engine.len() >= 5, "should load at least 5 templates");
        }
    }

    #[test]
    fn from_directory_errors_on_empty() {
        let result = PromptEngine::from_directory("/tmp/nonexistent_memz_prompts_dir");
        assert!(result.is_err());
    }
}
