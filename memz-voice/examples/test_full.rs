//! Full voice dialogue test — combines STT + LLM + TTS in interactive loop.
//!
//! Records your voice, transcribes it, generates NPC response via Ollama,
//! and synthesizes speech. The full pipeline!
//!
//! Prerequisites:
//!   - `models/whisper-tiny.en.bin` must exist
//!   - `ollama serve` && `ollama pull llama3.2:1b`
//!
//! Usage:
//!   cargo run -p memz-voice --example test_full

use memz_voice::llm::{DialogueLlm, LlmConfig, NpcContext};
use memz_voice::stt::{SpeechToText, SttConfig};
use memz_voice::tts::{TextToSpeech, TtsConfig};
use memz_voice::voice_profile::VoiceProfile;
use memz_voice::conversation::ConversationHistory;
use std::io::{self, Write};
use std::thread;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    println!("=== Full Voice Dialogue Test ===\n");

    let model_dir = std::env::var("MEMZ_MODELS_DIR")
        .unwrap_or_else(|_| "models".to_string());
    let whisper_path = format!("{}/whisper-tiny.en.bin", model_dir);

    if !std::path::Path::new(&whisper_path).exists() {
        eprintln!("Error: Whisper model not found at {}", whisper_path);
        eprintln!("Download: curl -L -o models/whisper-tiny.en.bin \\");
        eprintln!("  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin");
        return Ok(());
    }

    println!("Loading models...");
    let mut stt = SpeechToText::new(SttConfig {
        model_path: whisper_path,
        ..SttConfig::default()
    })?;

    let mut llm = DialogueLlm::new(LlmConfig {
        model_name: "llama3.2:1b".to_string(),
        max_tokens: 80,
        ..LlmConfig::default()
    })?;

    let mut tts = TextToSpeech::with_defaults()?;
    println!("All models loaded!\n");

    let npc = NpcContext {
        name: "Grimjaw".to_string(),
        profession: "Guard".to_string(),
        location: "Stonehaven Gate".to_string(),
        faction: "Town Watch".to_string(),
        personality: "Grumpy, suspicious, but ultimately fair".to_string(),
        mood: "grumpy".to_string(),
        knowledge: String::new(),
        player_sentiment: "unknown".to_string(),
    };
    let voice = VoiceProfile::from_npc("guard", 0.2, 0.3);
    let mut history = ConversationHistory::new(1, 6);

    println!("You approach {} the {} ({})", npc.name, npc.profession, npc.mood);
    println!("Press Enter to talk (records 5 seconds), or type 'quit' to exit\n");

    loop {
        print!("Press Enter to start talking: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if input.trim().eq_ignore_ascii_case("quit") { break; }

        println!("🎤 Recording... (speak now, 5 seconds)");
        stt.start_recording()?;
        thread::sleep(Duration::from_secs(5));

        println!("⏳ Transcribing...");
        let transcription = stt.stop_and_transcribe()?;
        println!("📝 You said: \"{}\"", transcription);

        if transcription.trim().is_empty() {
            println!("(No speech detected, try again)\n");
            continue;
        }

        println!("🤔 {} is thinking...", npc.name);
        let start = std::time::Instant::now();
        match llm.generate_response(&npc, &transcription, Some(&history)) {
            Ok(response) => {
                let elapsed = start.elapsed();
                println!("🤖 {} ({}ms): \"{}\"", npc.name, elapsed.as_millis(), response);

                println!("🗣️ Synthesizing voice...");
                let audio = tts.synthesize(&response, &voice)?;
                println!("🔊 Generated {:.1}s of audio ({} samples)", 
                    audio.len() as f32 / 24000.0, audio.len());

                history.add_exchange(transcription, response);
            }
            Err(e) => println!("❌ Error: {}", e),
        }
        println!();
    }

    println!("Grimjaw grunts and turns back to his post.");
    Ok(())
}
