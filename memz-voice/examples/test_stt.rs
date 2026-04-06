//! Speech-to-Text test — records audio from microphone and transcribes it.
//!
//! Prerequisites: `models/whisper-tiny.en.bin` must exist.
//!
//! Usage:
//!   cargo run -p memz-voice --example test_stt

use memz_voice::stt::{SpeechToText, SttConfig};
use std::io;
use std::thread;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    env_logger::init();

    println!("=== Speech-to-Text Test ===\n");

    let model_dir = std::env::var("MEMZ_MODELS_DIR")
        .unwrap_or_else(|_| "models".to_string());
    let model_path = format!("{}/whisper-tiny.en.bin", model_dir);

    if !std::path::Path::new(&model_path).exists() {
        eprintln!("Error: Whisper model not found at {}", model_path);
        eprintln!("Download it with:");
        eprintln!("  curl -L -o models/whisper-tiny.en.bin \\");
        eprintln!("    https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin");
        return Ok(());
    }

    println!("Loading Whisper model from: {}", model_path);
    let mut stt = SpeechToText::new(SttConfig {
        model_path: model_path.clone(),
        ..SttConfig::default()
    })?;
    println!("Model loaded!\n");

    println!("Press Enter to start recording...");
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    println!("🎤 Recording... (will record for 5 seconds)");
    stt.start_recording()?;

    thread::sleep(Duration::from_secs(5));

    println!("⏳ Transcribing...");
    let transcription = stt.stop_and_transcribe()?;

    println!("\n✅ Transcription:");
    println!("  \"{}\"", transcription);

    Ok(())
}
