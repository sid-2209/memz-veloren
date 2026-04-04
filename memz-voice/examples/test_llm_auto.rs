//! Automated LLM dialogue test — runs 3 test scenarios without user input.
//!
//! Tests different NPC professions, moods, and conversation contexts.
//!
//! Prerequisites: `ollama serve` && `ollama pull llama3.2:1b`
//!
//! Usage:
//!   cargo run -p memz-voice --example test_llm_auto

use memz_voice::llm::{DialogueLlm, LlmConfig, NpcContext};
use memz_voice::conversation::ConversationHistory;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    println!("=== Automated LLM Dialogue Test ===\n");

    let mut llm = DialogueLlm::new(LlmConfig {
        model_name: "llama3.2:1b".to_string(),
        max_tokens: 60,
        seed: Some(42),
        ..LlmConfig::default()
    })?;

    let test_cases: Vec<(NpcContext, &str)> = vec![
        (
            NpcContext {
                name: "Thorin".to_string(),
                profession: "Blacksmith".to_string(),
                location: "Ironforge".to_string(),
                faction: String::new(),
                personality: "Gruff but proud".to_string(),
                mood: "friendly".to_string(),
                knowledge: String::new(),
                player_sentiment: "potential customer".to_string(),
            },
            "Hello! Can you repair my sword?",
        ),
        (
            NpcContext {
                name: "Elder Sage".to_string(),
                profession: "Herbalist".to_string(),
                location: "Whispering Woods".to_string(),
                faction: "Druids Circle".to_string(),
                personality: "Wise, mysterious, observant".to_string(),
                mood: "contemplative".to_string(),
                knowledge: "Knows of rare herbs deep in the forest".to_string(),
                player_sentiment: "curious adventurer".to_string(),
            },
            "I'm looking for a cure for the forest blight. Can you help?",
        ),
        (
            NpcContext {
                name: "Captain Reyna".to_string(),
                profession: "Guard Captain".to_string(),
                location: "Willowvale".to_string(),
                faction: "Town Watch".to_string(),
                personality: "Stern, duty-bound".to_string(),
                mood: "suspicious".to_string(),
                knowledge: "Reports of bandits on the northern road".to_string(),
                player_sentiment: "unknown traveler".to_string(),
            },
            "I've come from the north. The roads are crawling with bandits.",
        ),
    ];

    for (i, (npc, player_input)) in test_cases.iter().enumerate() {
        println!("--- Test {} ---", i + 1);
        println!("NPC: {} ({}, {})", npc.name, npc.profession, npc.mood);
        println!("🎤 Player: \"{}\"", player_input);

        let start = std::time::Instant::now();
        match llm.generate_response(npc, player_input, None) {
            Ok(response) => {
                let elapsed = start.elapsed();
                println!("🤖 {} ({}ms): \"{}\"\n✅ Success\n", npc.name, elapsed.as_millis(), response);
            }
            Err(e) => println!("❌ Error: {}\n", e),
        }
    }

    println!("=== All tests complete ===");
    Ok(())
}
