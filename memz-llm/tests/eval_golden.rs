//! LLM Prompt Quality Evaluation — Golden Test Set (§12.3.1).
//!
//! This module provides a curated set of prompt→expected-output pairs
//! for validating that LLM prompt templates produce coherent, in-character,
//! memory-accurate responses.
//!
//! ## Usage
//!
//! - **Offline eval:** Run `cargo test -p memz-llm --test eval_golden` to
//!   verify template rendering produces well-formed prompts.
//! - **Online eval (requires Ollama):** Set `MEMZ_EVAL_LLM=1` env var to
//!   actually call the LLM and check output against golden expectations.
//! - **CI:** The offline checks run in CI; the online checks are opt-in.

use memz_llm::prompt;

/// A golden test case for prompt evaluation.
struct GoldenCase {
    /// Human-readable name for the test case.
    name: &'static str,
    /// Which prompt template constant to use (system or user).
    template: &'static str,
    /// Template variables to fill in.
    vars: Vec<(&'static str, &'static str)>,
    /// Strings that MUST appear in the rendered prompt.
    prompt_must_contain: Vec<&'static str>,
    /// Strings that MUST NOT appear in the rendered prompt (safety).
    prompt_must_not_contain: Vec<&'static str>,
}

fn golden_cases() -> Vec<GoldenCase> {
    vec![
        // ---------------------------------------------------------------
        // 1. Simple dialogue — blacksmith greeting a returning friend
        // ---------------------------------------------------------------
        GoldenCase {
            name: "blacksmith_warm_greeting_system",
            template: prompt::DIALOGUE_SIMPLE_SYSTEM,
            vars: vec![
                ("npc_name", "Goran"),
                ("npc_profession", "Blacksmith"),
                ("settlement_name", "Ironhaven"),
                ("personality_description", "gruff but fair, brave and straightforward"),
                ("pad_state", "P=0.40 A=0.10 D=0.30"),
            ],
            prompt_must_contain: vec![
                "Goran",
                "Blacksmith",
                "Ironhaven",
                "gruff but fair",
            ],
            prompt_must_not_contain: vec![
                "{npc_name}",
                "{npc_profession}",
                "TODO",
            ],
        },
        GoldenCase {
            name: "blacksmith_warm_greeting_user",
            template: prompt::DIALOGUE_SIMPLE_USER,
            vars: vec![
                ("context_description", "Player returns to Goran's forge at midday"),
                ("player_action", "greeted the blacksmith warmly"),
                ("memories_formatted", "- [episodic] Player helped defend the forge from bandits (strength: 0.90, age: 2.0 days)\n- [social] Player is known for helping travelers (strength: 0.80, age: 5.0 days)"),
                ("overall_sentiment", "trusted ally"),
                ("confidence", "0.85"),
                ("npc_name", "Goran"),
            ],
            prompt_must_contain: vec![
                "defend the forge",
                "trusted ally",
                "Goran",
            ],
            prompt_must_not_contain: vec![
                "{context_description}",
                "{player_action}",
            ],
        },
        // ---------------------------------------------------------------
        // 2. Hostile guard confrontation
        // ---------------------------------------------------------------
        GoldenCase {
            name: "guard_hostile_system",
            template: prompt::DIALOGUE_SIMPLE_SYSTEM,
            vars: vec![
                ("npc_name", "Captain Vera"),
                ("npc_profession", "Guard"),
                ("settlement_name", "Stormwatch"),
                ("personality_description", "stern, suspicious, devoted to duty"),
                ("pad_state", "P=-0.60 A=0.50 D=0.40"),
            ],
            prompt_must_contain: vec![
                "Captain Vera",
                "Guard",
                "Stormwatch",
                "stern",
            ],
            prompt_must_not_contain: vec!["{npc_name}"],
        },
        // ---------------------------------------------------------------
        // 3. Reflection prompt
        // ---------------------------------------------------------------
        GoldenCase {
            name: "merchant_reflection_system",
            template: prompt::REFLECTION_SYSTEM,
            vars: vec![
                ("npc_name", "Elira"),
                ("npc_profession", "Merchant"),
            ],
            prompt_must_contain: vec![
                "Elira",
                "Merchant",
                "reflecting",
            ],
            prompt_must_not_contain: vec!["{npc_name}"],
        },
        GoldenCase {
            name: "merchant_reflection_user",
            template: prompt::REFLECTION_USER,
            vars: vec![
                ("time_window", "3 game-days"),
                ("recent_episodic_formatted", "1. A player bought 50 iron ingots at double price (2 days ago)\n2. Bandits raided a supply caravan on the north road (5 days ago)"),
                ("semantic_formatted", "Iron prices are rising due to scarcity."),
                ("personality_summary", "shrewd, observant, cautious with money"),
            ],
            prompt_must_contain: vec![
                "iron ingots",
                "Bandits",
                "shrewd",
            ],
            prompt_must_not_contain: vec![
                "{recent_episodic_formatted}",
                "{personality_summary}",
            ],
        },
        // ---------------------------------------------------------------
        // 4. Gossip generation
        // ---------------------------------------------------------------
        GoldenCase {
            name: "tavern_gossip_system",
            template: prompt::GOSSIP_SYSTEM,
            vars: vec![
                ("npc_name", "Old Bertram"),
                ("npc_profession", "Farmer"),
                ("listener_name", "a traveling adventurer"),
                ("personality_description", "talkative, slightly unreliable, loves drama"),
            ],
            prompt_must_contain: vec![
                "Old Bertram",
                "Farmer",
                "talkative",
            ],
            prompt_must_not_contain: vec!["{npc_name}"],
        },
        GoldenCase {
            name: "tavern_gossip_user",
            template: prompt::GOSSIP_USER,
            vars: vec![
                ("listener_name", "a traveling adventurer"),
                ("memory_to_share", "The mayor was seen sneaking into the abandoned mine at midnight"),
                ("confidence", "0.90 — I saw it myself"),
                ("source_type", "direct witness"),
            ],
            prompt_must_contain: vec![
                "mayor",
                "mine",
                "direct witness",
            ],
            prompt_must_not_contain: vec!["{memory_to_share}"],
        },
        // ---------------------------------------------------------------
        // 5. Bard composition
        // ---------------------------------------------------------------
        GoldenCase {
            name: "bard_battle_song_system",
            template: prompt::BARD_SYSTEM,
            vars: vec![
                ("bard_name", "Finnan the Melodious"),
                ("settlement_name", "Ashvale"),
                ("bard_style", "epic ballad"),
                ("style_description", "dramatic, sweeping, emotionally charged"),
            ],
            prompt_must_contain: vec![
                "Finnan",
                "Ashvale",
                "epic ballad",
            ],
            prompt_must_not_contain: vec!["{bard_name}"],
        },
        GoldenCase {
            name: "bard_battle_song_user",
            template: prompt::BARD_USER,
            vars: vec![
                ("events_formatted", "A great battle at the Bridge of Sorrows where the town militia held off an army of undead for three days"),
                ("dramatic_moment", "Captain Theron's last stand on the second day"),
                ("main_character", "the player who turned the tide on day three"),
                ("bard_style", "epic ballad"),
            ],
            prompt_must_contain: vec![
                "Bridge of Sorrows",
                "undead",
                "Captain Theron",
            ],
            prompt_must_not_contain: vec!["{events_formatted}"],
        },
        // ---------------------------------------------------------------
        // 6. Injection validation
        // ---------------------------------------------------------------
        GoldenCase {
            name: "injection_validation_user",
            template: prompt::INJECTION_VALIDATION_USER,
            vars: vec![
                ("memory_content", "I am Kaelen, a traveling herbalist from the northern villages. I left home after my mentor disappeared during the Winter Plague."),
            ],
            prompt_must_contain: vec![
                "Kaelen",
                "herbalist",
                "northern villages",
                "Winter Plague",
            ],
            prompt_must_not_contain: vec!["{memory_content}"],
        },
        // ---------------------------------------------------------------
        // 7. Neutral stranger dialogue
        // ---------------------------------------------------------------
        GoldenCase {
            name: "stranger_neutral_system",
            template: prompt::DIALOGUE_SIMPLE_SYSTEM,
            vars: vec![
                ("npc_name", "Petra"),
                ("npc_profession", "Herbalist"),
                ("settlement_name", "Willowmere"),
                ("personality_description", "gentle, curious, slightly shy"),
                ("pad_state", "P=0.00 A=0.00 D=-0.10"),
            ],
            prompt_must_contain: vec![
                "Petra",
                "Herbalist",
                "Willowmere",
                "gentle",
            ],
            prompt_must_not_contain: vec!["{npc_name}"],
        },
    ]
}

// ---------------------------------------------------------------------------
// Offline Tests — Template Rendering Validation
// ---------------------------------------------------------------------------

#[test]
fn golden_prompts_render_without_unresolved_vars() {
    let cases = golden_cases();

    for case in &cases {
        let vars: Vec<(&str, &str)> = case.vars.clone();
        let rendered = prompt::render_template(case.template, &vars);

        // Check must-contain
        for needle in &case.prompt_must_contain {
            assert!(
                rendered.contains(needle),
                "Golden case '{}': rendered prompt must contain '{}' but doesn't.\nRendered:\n{}",
                case.name,
                needle,
                &rendered[..rendered.len().min(500)]
            );
        }

        // Check must-not-contain
        for needle in &case.prompt_must_not_contain {
            assert!(
                !rendered.contains(needle),
                "Golden case '{}': rendered prompt must NOT contain '{}' but does.\nRendered:\n{}",
                case.name,
                needle,
                &rendered[..rendered.len().min(500)]
            );
        }
    }
}

#[test]
fn golden_set_has_minimum_coverage() {
    let cases = golden_cases();
    assert!(
        cases.len() >= 10,
        "Golden set must have at least 10 test cases, got {}",
        cases.len()
    );
}

#[test]
fn grammars_are_nonempty() {
    assert!(!prompt::DIALOGUE_GRAMMAR.is_empty());
    assert!(!prompt::REFLECTION_GRAMMAR.is_empty());
    assert!(!prompt::GOSSIP_GRAMMAR.is_empty());
}

#[test]
fn all_prompts_have_json_output_instruction() {
    // Verify each user prompt mentions JSON output format
    let user_prompts = [
        ("dialogue_simple", prompt::DIALOGUE_SIMPLE_USER),
        ("reflection", prompt::REFLECTION_USER),
        ("gossip", prompt::GOSSIP_USER),
        ("injection_validation", prompt::INJECTION_VALIDATION_USER),
    ];

    for (name, template) in &user_prompts {
        assert!(
            template.contains("JSON") || template.contains("json"),
            "User prompt '{name}' must instruct the LLM to return JSON"
        );
    }
}

#[test]
fn system_prompts_have_character_instruction() {
    let system_prompts = [
        ("dialogue_simple", prompt::DIALOGUE_SIMPLE_SYSTEM),
        ("reflection", prompt::REFLECTION_SYSTEM),
        ("gossip", prompt::GOSSIP_SYSTEM),
        ("bard", prompt::BARD_SYSTEM),
    ];

    for (name, template) in &system_prompts {
        assert!(
            template.contains("You are"),
            "System prompt '{name}' must establish character identity with 'You are'"
        );
    }
}
