/// Simulates in-game voice interaction with NPCs
/// This demonstrates how the voice system works in Veloren

use memz_veloren::{VoiceSystem, VelorenNpcData, VoiceGameEvent};
use std::io::{self, Write};

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║     Veloren Voice NPC — In-Game Simulation Test           ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!();

    // Initialize voice system
    println!("Initializing voice system...");
    let mut voice_system = VoiceSystem::new();
    println!("✅ Voice system ready!\n");

    // Simulate different NPCs
    let npcs = vec![
        VelorenNpcData {
            entity_id: 1,
            name: "Captain Aldric".to_string(),
            profession: "Guard".to_string(),
            location: "Willowvale Gate".to_string(),
            faction: "Town Watch".to_string(),
            personality_description: "Stern, dutiful, protective".to_string(),
            mood: "alert but calm".to_string(),
            player_sentiment: "cautious toward stranger".to_string(),
            extraversion: 0.3,
            neuroticism: 0.2,
            position: [100.0, 200.0, 50.0],
        },
        VelorenNpcData {
            entity_id: 2,
            name: "Mira".to_string(),
            profession: "Merchant".to_string(),
            location: "Willowvale Market".to_string(),
            faction: "Traders Guild".to_string(),
            personality_description: "Friendly, shrewd, talkative".to_string(),
            mood: "cheerful".to_string(),
            player_sentiment: "potential customer".to_string(),
            extraversion: 0.8,
            neuroticism: 0.4,
            position: [150.0, 210.0, 50.0],
        },
        VelorenNpcData {
            entity_id: 3,
            name: "Thorin".to_string(),
            profession: "Blacksmith".to_string(),
            location: "Ironforge".to_string(),
            faction: "Mountain Clan".to_string(),
            personality_description: "Gruff but kind-hearted".to_string(),
            mood: "content".to_string(),
            player_sentiment: "neutral".to_string(),
            extraversion: 0.4,
            neuroticism: 0.3,
            position: [120.0, 190.0, 50.0],
        },
    ];

    println!("Available NPCs:");
    for (i, npc) in npcs.iter().enumerate() {
        println!("  {}. {} the {} ({})", i + 1, npc.name, npc.profession, npc.mood);
    }
    println!();

    loop {
        print!("Select NPC (1-3) or 'quit' to exit: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.eq_ignore_ascii_case("quit") { break; }

        let npc_idx: usize = match input.parse::<usize>() {
            Ok(n) if n >= 1 && n <= 3 => n - 1,
            _ => {
                println!("Invalid selection. Try again.\n");
                continue;
            }
        };

        let npc = &npcs[npc_idx];
        println!("\n═══════════════════════════════════════════════════════════");
        println!("🎮 You approach: {} the {}", npc.name, npc.profession);
        println!("═══════════════════════════════════════════════════════════");
        println!("  Location: {} | Mood: {} | Faction: {}", npc.location, npc.mood, npc.faction);
        println!();

        // Start voice interaction (simulates V key press)
        println!("🎮 [Holding V to talk...]");
        if let Err(e) = voice_system.start_interaction(npc) {
            println!("❌ Start error: {}", e);
            continue;
        }

        // Simulate recording for 5 seconds
        println!("🎤 Recording... (speak now, 5 seconds)");
        std::thread::sleep(std::time::Duration::from_secs(5));

        // Stop recording (simulates V key release)
        println!("⏳ Processing your voice input...");
        if let Err(e) = voice_system.stop_recording() {
            println!("❌ Stop error: {}", e);
            continue;
        }

        // Poll for events (simulates per-frame update loop)
        let poll_start = std::time::Instant::now();
        loop {
            let events = voice_system.update();
            for event in &events {
                match event {
                    VoiceGameEvent::PlayerTranscription(text) => {
                        println!("\n📝 You said: \"{}\"", text);
                    }
                    VoiceGameEvent::NpcResponseText { text, .. } => {
                        println!("💬 {} says: \"{}\"", npc.name, text);
                    }
                    VoiceGameEvent::NpcAudioChunk { audio, .. } => {
                        println!("🔊 Audio chunk: {} samples ({:.1}s)",
                            audio.len(), audio.len() as f32 / 24000.0);
                    }
                    VoiceGameEvent::NpcAudioComplete { .. } => {
                        println!("✅ Audio complete!");
                    }
                    VoiceGameEvent::InteractionComplete => {
                        println!("✅ Interaction complete!\n");
                    }
                    VoiceGameEvent::Error(msg) => {
                        println!("❌ Error: {}", msg);
                    }
                }
            }

            if !voice_system.is_active() || poll_start.elapsed().as_secs() > 30 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    println!("\n👋 Voice interaction test complete!");
    Ok(())
}
