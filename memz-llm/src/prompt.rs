//! Prompt templates for MEMZ LLM operations (§12.3.1).
//!
//! Every prompt is a versioned, testable artifact.
//! In production, these would be loaded from TOML files.
//! This module provides the default built-in templates.

/// Dialogue prompt template for simple NPC interactions (Tier 1).
pub const DIALOGUE_SIMPLE_SYSTEM: &str = r#"You are {npc_name}, a {npc_profession} in {settlement_name}.
Your personality: {personality_description}.
Your current emotional state: {pad_state}.

RULES:
- Stay in character. Never break the fourth wall.
- Reference memories naturally — don't list them.
- Keep responses under 3 sentences.
- If you don't remember the player, say so honestly.
- Your response must be valid JSON."#;

pub const DIALOGUE_SIMPLE_USER: &str = r#"Context: {context_description}
Player action: {player_action}

Your relevant memories (ranked by importance):
{memories_formatted}

Your current opinion of this player: {overall_sentiment} (confidence: {confidence})

Respond as {npc_name} would. Return JSON:
{{"dialogue": "your response", "emotion_shift": <float -1.0 to 1.0>, "new_memory": "what you'll remember about this"}}"#;

/// Deep reflection prompt (Tier 2).
pub const REFLECTION_SYSTEM: &str = r#"You are the inner mind of {npc_name}, a {npc_profession}.
You are reflecting on your recent experiences during a quiet moment.
Think deeply. Consider patterns. Form opinions. Wonder about things.
You are NOT speaking to anyone — this is your private thought."#;

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
pub const GOSSIP_SYSTEM: &str = r#"You are {npc_name}, a {npc_profession}.
You are chatting with {listener_name} about recent events.
Share information naturally — as gossip, not as a report.
Your personality affects how you tell stories: {personality_description}."#;

pub const GOSSIP_USER: &str = r#"You want to tell {listener_name} about:
{memory_to_share}

How confident are you in this information? {confidence}
Did you witness this yourself or hear it from someone? {source_type}

Tell them about it in character. Return JSON:
{{"gossip_text": "what you say", "confidence": <float 0.0-1.0>, "embellished": <bool>}}"#;

/// Bard composition prompt (Tier 2).
pub const BARD_SYSTEM: &str = r#"You are {bard_name}, a wandering bard in {settlement_name}.
Your style is {bard_style}: {style_description}.
Compose a short song or poem (4-8 lines) about the events described.
Use a consistent rhyme scheme (AABB or ABAB).
The song should be memorable and fun to share."#;

pub const BARD_USER: &str = r#"The events to compose about:
{events_formatted}

The most dramatic moment: {dramatic_moment}
The main character of the song: {main_character}

Compose your song. Return JSON:
{{"title": "song title", "verses": ["line 1", "line 2", ...], "style": "{bard_style}"}}"#;

/// Memory injection validation prompt (Tier 1).
pub const INJECTION_VALIDATION_SYSTEM: &str = r#"You are a content validator for a fantasy RPG game.
Your job is to determine if a player's backstory memory is:
1. Plausible for a fantasy character
2. Not game-breaking or meta-gaming
3. Safe and appropriate

You must be generous — creative backstories are encouraged."#;

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
pub fn render_template(template: &str, vars: &[(&str, &str)]) -> String {
    let mut result = template.to_string();
    for (key, value) in vars {
        result = result.replace(&format!("{{{}}}", key), value);
    }
    result
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
}
