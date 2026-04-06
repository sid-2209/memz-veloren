//! End-to-end voice pipeline test with real models.
//!
//! Tests the complete flow: LLM inference via Ollama HTTP API,
//! TTS synthesis (placeholder), and voice pipeline orchestration.
//!
//! Prerequisites:
//!   - Ollama must be running: `ollama serve`
//!   - Model must be pulled: `ollama pull llama3.2:1b`
//!
//! Usage:
//!   cargo run -p memz-voice --example test_e2e_voice

use memz_voice::llm::{DialogueLlm, LlmConfig, NpcContext};
use memz_voice::tts::TextToSpeech;
use memz_voice::voice_profile::VoiceProfile;
use memz_voice::conversation::ConversationHistory;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("╔══════════════════════════════════════════════════╗");
    println!("║   SOTA Voice NPC Pipeline — End-to-End Test     ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!();

    // ─── Model paths ───────────────────────────────────────────
    let model_dir = std::env::var("MEMZ_MODELS_DIR")
        .unwrap_or_else(|_| "models".to_string());

    let whisper_model_path = format!("{}/whisper-tiny.en.bin", model_dir);
    let ollama_model = std::env::var("OLLAMA_MODEL")
        .unwrap_or_else(|_| "llama3.2:1b".to_string());

    println!("📦 Configuration:");
    println!("   LLM:     Ollama → {}", ollama_model);
    println!("   Whisper:  {}", whisper_model_path);
    println!();

    // ─── Test 1: LLM Inference via Ollama ─────────────────────
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🧠 Test 1: LLM Inference via Ollama ({})", ollama_model);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let llm_config = LlmConfig {
        ollama_url: "http://localhost:11434".to_string(),
        model_name: ollama_model.clone(),
        temperature: 0.7,
        max_tokens: 60,
        seed: Some(42),
        context_size: 2048,
        keep_alive: "30m".to_string(),
    };

    let mut llm = match DialogueLlm::new(llm_config) {
        Ok(llm) => llm,
        Err(e) => {
            eprintln!("❌ Failed to create LLM: {}", e);
            return;
        }
    };

    // Set up NPC context
    let guard_npc = NpcContext {
        name: "Captain Aldric".to_string(),
        profession: "Guard Captain".to_string(),
        location: "Willowvale".to_string(),
        faction: "Town Watch".to_string(),
        personality: "Stern, dutiful, protective, slightly suspicious of strangers".to_string(),
        mood: "alert but calm".to_string(),
        knowledge: String::new(),
        player_sentiment: "cautious toward unknown traveler".to_string(),
    };

    let test_inputs = vec![
        "Hello Captain! What's going on in this town?",
        "Are there any dangers nearby?",
        "Can I help with anything?",
    ];

    let mut history = ConversationHistory::new(1, 6);

    for input in &test_inputs {
        println!();
        println!("🎤 Player: \"{}\"", input);

        let start = std::time::Instant::now();
        match llm.generate_response(&guard_npc, input, Some(&history)) {
            Ok(response) => {
                let elapsed = start.elapsed();
                println!("🤖 {} ({}ms): \"{}\"", guard_npc.name, elapsed.as_millis(), response);
                history.add_exchange(input.to_string(), response);
            }
            Err(e) => {
                println!("❌ LLM error: {}", e);
            }
        }
    }

    // ─── Test 2: Streaming LLM ─────────────────────────────────
    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🔄 Test 2: Streaming LLM Token Generation");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let merchant_npc = NpcContext {
        name: "Mira".to_string(),
        profession: "Merchant".to_string(),
        location: "Willowvale Market".to_string(),
        faction: "Traders Guild".to_string(),
        personality: "Friendly, shrewd, talkative, loves to gossip".to_string(),
        mood: "cheerful".to_string(),
        knowledge: String::new(),
        player_sentiment: "potential customer".to_string(),
    };

    println!();
    println!("🎤 Player: \"What do you have for sale today?\"");
    print!("🤖 Mira (streaming): ");

    let mut token_count = 0;
    let start = std::time::Instant::now();
    match llm.generate_response_streaming(
        &merchant_npc,
        "What do you have for sale today?",
        None,
        |_text| {
            token_count += 1;
            // Print dots to show streaming progress
            if token_count % 5 == 0 {
                print!(".");
                let _ = std::io::Write::flush(&mut std::io::stdout());
            }
            true
        },
    ) {
        Ok(response) => {
            let elapsed = start.elapsed();
            println!();
            println!("   Full response: \"{}\"", response);
            println!("   Tokens: {}, Time: {}ms", token_count, elapsed.as_millis());
        }
        Err(e) => {
            println!();
            println!("   ❌ Streaming error: {}", e);
        }
    }

    // ─── Test 3: TTS Synthesis ─────────────────────────────────
    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🗣️ Test 3: TTS Synthesis (Placeholder)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let mut tts = TextToSpeech::with_defaults().unwrap();

    let profiles = vec![
        ("Guard", VoiceProfile::from_npc("guard", 0.3, 0.2)),
        ("Merchant", VoiceProfile::from_npc("merchant", 0.8, 0.4)),
        ("Blacksmith", VoiceProfile::from_npc("blacksmith", 0.4, 0.3)),
        ("Herbalist", VoiceProfile::from_npc("herbalist", 0.6, 0.2)),
    ];

    for (name, profile) in &profiles {
        let text = "Welcome to our humble village, traveler.";
        let start = std::time::Instant::now();
        let audio = tts.synthesize(text, profile).unwrap();
        let elapsed = start.elapsed();
        println!(
            "   {} ({}, speed={:.2}, pitch={:.1}): {} samples ({:.1}s audio) in {:.0}ms",
            name, profile.voice_id, profile.speed, profile.pitch_shift,
            audio.len(), audio.len() as f32 / 24000.0, elapsed.as_millis()
        );
    }

    // ─── Test 4: Streaming TTS ─────────────────────────────────
    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("🔊 Test 4: Streaming TTS (Sentence-by-Sentence)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let text = "Greetings traveler! I haven't seen you around before. \
                Are you looking for supplies? We have the finest wares in the region.";

    let profile = VoiceProfile::from_npc("merchant", 0.8, 0.4);
    let mut chunk_count = 0;
    let mut total_samples = 0;

    let start = std::time::Instant::now();
    tts.synthesize_streaming(text, &profile, |audio| {
        chunk_count += 1;
        total_samples += audio.len();
        println!(
            "   Chunk {}: {} samples ({:.1}s audio)",
            chunk_count, audio.len(), audio.len() as f32 / 24000.0
        );
        true
    }).unwrap();
    let elapsed = start.elapsed();
    println!(
        "   Total: {} chunks, {} samples ({:.1}s audio) in {:.0}ms",
        chunk_count, total_samples, total_samples as f32 / 24000.0, elapsed.as_millis()
    );

    // ─── Summary ───────────────────────────────────────────────
    println!();
    println!("╔══════════════════════════════════════════════════╗");
    println!("║              ALL TESTS COMPLETE ✅               ║");
    println!("╚══════════════════════════════════════════════════╝");
    println!();
    println!("Configuration used:");
    println!("   LLM:  Ollama → {} (via HTTP API)", ollama_model);
    println!("   STT:  {} (exists: {})", whisper_model_path, std::path::Path::new(&whisper_model_path).exists());
    println!();
}
