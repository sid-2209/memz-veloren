//! Text-to-Speech test — synthesizes text with different NPC voice profiles.
//!
//! Usage:
//!   cargo run -p memz-voice --example test_tts

use memz_voice::tts::TextToSpeech;
use memz_voice::voice_profile::VoiceProfile;
use std::io::{self, Write};

fn main() -> anyhow::Result<()> {
    env_logger::init();

    println!("=== Text-to-Speech Test ===\n");

    let mut tts = TextToSpeech::with_defaults()?;

    let profiles = vec![
        ("Guard", VoiceProfile::from_npc("guard", 0.3, 0.2)),
        ("Merchant", VoiceProfile::from_npc("merchant", 0.8, 0.4)),
        ("Blacksmith", VoiceProfile::from_npc("blacksmith", 0.4, 0.3)),
    ];

    loop {
        print!("Enter text to synthesize (or 'quit' to exit): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let text = input.trim();

        if text.is_empty() { continue; }
        if text.eq_ignore_ascii_case("quit") { break; }

        for (name, profile) in &profiles {
            let start = std::time::Instant::now();
            let audio = tts.synthesize(text, profile)?;
            let elapsed = start.elapsed();
            println!(
                "  {} ({}, speed={:.2}): {} samples ({:.1}s) in {:.0}ms",
                name, profile.voice_id, profile.speed,
                audio.len(), audio.len() as f32 / 24000.0, elapsed.as_millis()
            );
        }
        println!();
    }

    Ok(())
}
