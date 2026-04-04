use cpal::traits::{DeviceTrait, HostTrait};

fn main() -> anyhow::Result<()> {
    println!("=== Available Audio Devices ===\n");

    let host = cpal::default_host();
    
    println!("Input Devices:");
    println!("--------------");
    let mut input_count = 0;
    for (idx, device) in host.input_devices()?.enumerate() {
        let name = device.name().unwrap_or_else(|_| "Unknown".to_string());
        let default_config = device.default_input_config();
        
        print!("{}. {}", idx, name);
        
        if let Ok(config) = default_config {
            println!(" - {} Hz, {} channels", 
                config.sample_rate().0, 
                config.channels()
            );
        } else {
            println!(" - (no config available)");
        }
        input_count += 1;
    }
    
    if input_count == 0 {
        println!("  (No input devices found)");
    }
    
    println!("\nDefault Input Device:");
    println!("--------------------");
    if let Some(device) = host.default_input_device() {
        let name = device.name().unwrap_or_else(|_| "Unknown".to_string());
        println!("  {}", name);
        
        if let Ok(config) = device.default_input_config() {
            println!("  Sample rate: {} Hz", config.sample_rate().0);
            println!("  Channels: {}", config.channels());
            println!("  Format: {:?}", config.sample_format());
        }
    } else {
        println!("  (No default input device)");
    }
    
    println!("\n=== Microphone Test ===");
    println!("If your actual microphone is not listed above,");
    println!("check System Settings → Sound → Input");
    println!("\nTo use a specific device, we'll need to modify the code.");
    
    Ok(())
}
