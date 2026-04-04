use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    println!("=== Microphone Test ===\n");

    let host = cpal::default_host();
    
    // List all input devices
    println!("Available Input Devices:");
    println!("------------------------");
    let devices: Vec<_> = host.input_devices()?.collect();
    
    if devices.is_empty() {
        println!("❌ No input devices found!");
        println!("\nCheck:");
        println!("  1. Microphone is connected");
        println!("  2. System Settings → Sound → Input");
        println!("  3. System Settings → Privacy → Microphone (enable for Terminal)");
        return Ok(());
    }
    
    for (idx, device) in devices.iter().enumerate() {
        let name = device.name().unwrap_or_else(|_| "Unknown".to_string());
        print!("  {}. {}", idx + 1, name);
        
        if let Ok(config) = device.default_input_config() {
            println!(" ({} Hz, {} ch)", config.sample_rate().0, config.channels());
        } else {
            println!();
        }
    }
    
    println!("\nDefault Input Device:");
    println!("--------------------");
    let device = host.default_input_device()
        .ok_or_else(|| anyhow::anyhow!("No default input device"))?;
    
    let device_name = device.name()?;
    println!("  {}", device_name);
    
    if device_name.contains("Steam") {
        println!("\n⚠️  WARNING: Using Steam Streaming Microphone!");
        println!("⚠️  This is a virtual device and won't capture your voice.");
        println!("\n💡 Fix:");
        println!("   1. Open System Settings → Sound → Input");
        println!("   2. Select your actual microphone (e.g., 'MacBook Pro Microphone')");
        println!("   3. Run this test again");
        return Ok(());
    }
    
    let config = device.default_input_config()?;
    println!("  Sample rate: {} Hz", config.sample_rate().0);
    println!("  Channels: {}", config.channels());
    println!("  Format: {:?}", config.sample_format());
    
    // Test recording
    println!("\n=== Recording Test ===");
    println!("Recording for 3 seconds...");
    println!("Speak now: 'Hello, this is a test!'");
    println!();
    
    let audio_buffer = Arc::new(Mutex::new(Vec::new()));
    let buffer_clone = Arc::clone(&audio_buffer);
    
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => build_stream::<f32>(&device, &config.into(), buffer_clone)?,
        cpal::SampleFormat::I16 => build_stream::<i16>(&device, &config.into(), buffer_clone)?,
        cpal::SampleFormat::U16 => build_stream::<u16>(&device, &config.into(), buffer_clone)?,
        _ => return Err(anyhow::anyhow!("Unsupported sample format")),
    };
    
    stream.play()?;
    
    // Record for 3 seconds
    for i in 1..=3 {
        thread::sleep(Duration::from_secs(1));
        let samples = audio_buffer.lock().unwrap().len();
        println!("  {}s - {} samples captured", i, samples);
    }
    
    drop(stream);
    
    // Analyze captured audio
    let audio = audio_buffer.lock().unwrap();
    println!("\n=== Results ===");
    println!("Total samples: {}", audio.len());
    
    if audio.is_empty() {
        println!("❌ No audio captured!");
        println!("\nPossible issues:");
        println!("  1. Microphone permissions not granted");
        println!("     → System Settings → Privacy & Security → Microphone");
        println!("     → Enable for Terminal");
        println!("  2. Wrong microphone selected");
        println!("     → System Settings → Sound → Input");
        println!("  3. Microphone is muted or volume is zero");
        return Ok(());
    }
    
    let max_amplitude = audio.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
    let avg_amplitude = audio.iter().map(|s| s.abs()).sum::<f32>() / audio.len() as f32;
    
    println!("Max amplitude: {:.4}", max_amplitude);
    println!("Avg amplitude: {:.4}", avg_amplitude);
    
    if max_amplitude < 0.001 {
        println!("\n⚠️  Audio captured but appears SILENT!");
        println!("\nPossible issues:");
        println!("  1. Microphone volume too low");
        println!("     → System Settings → Sound → Input → Input volume");
        println!("  2. Not speaking loud enough");
        println!("  3. Microphone is muted");
        println!("  4. Wrong input device selected");
    } else if max_amplitude < 0.01 {
        println!("\n⚠️  Audio level is VERY LOW");
        println!("   Try speaking louder or increasing microphone volume");
    } else if max_amplitude < 0.1 {
        println!("\n✓ Audio level is OK (but could be louder)");
    } else {
        println!("\n✅ Audio level is GOOD!");
        println!("   Your microphone is working correctly!");
    }
    
    println!("\n=== Next Steps ===");
    if max_amplitude > 0.01 {
        println!("✅ Microphone is working!");
        println!("   You can now run: cargo run --example test_full --release");
    } else {
        println!("❌ Fix microphone issues above, then run this test again");
    }
    
    Ok(())
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    buffer: Arc<Mutex<Vec<f32>>>,
) -> anyhow::Result<cpal::Stream>
where
    T: cpal::Sample + cpal::SizedSample,
    f32: cpal::FromSample<T>,
{
    let channels = config.channels as usize;
    
    let stream = device.build_input_stream(
        config,
        move |data: &[T], _: &cpal::InputCallbackInfo| {
            let mut buf = buffer.lock().unwrap();
            
            // Convert to mono f32
            for frame in data.chunks(channels) {
                let sample: f32 = frame[0].to_sample();
                buf.push(sample);
            }
        },
        |err| eprintln!("Audio stream error: {}", err),
        None,
    )?;

    Ok(stream)
}
