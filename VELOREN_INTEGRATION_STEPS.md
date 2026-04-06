# 🎮 Veloren Voice Integration - Step by Step

## ✅ Phase 1 Complete!

You successfully tested voice conversations with NPCs in the simulation!

---

## 🚀 Phase 2: Integrate into Veloren Game

### Overview

We'll add voice interaction to the actual Veloren game so you can:
1. Walk up to any NPC in-game
2. Press V key to start talking
3. Speak your question
4. Release V key
5. Hear the NPC respond with voice!

---

## Step 1: Check Veloren Structure

First, let's see what we're working with:

```bash
cd veloren
ls -la
```

We need to find:
- `voxygen/` - The game client (where we'll add voice)
- `server/` - The game server
- `common/` - Shared code

---

## Step 2: Add Dependencies

### File: `veloren/voxygen/Cargo.toml`

Add to `[dependencies]`:
```toml
memz-veloren = { path = "../../memz-veloren" }
```

---

## Step 3: Initialize Voice System

### File: `veloren/voxygen/src/session/mod.rs`

Add at the top:
```rust
use memz_veloren::VoiceSystem;
```

Add to the `SessionState` struct:
```rust
pub struct SessionState {
    // ... existing fields ...
    voice_system: Option<VoiceSystem>,
}
```

Initialize in `new()`:
```rust
impl SessionState {
    pub fn new(...) -> Self {
        // ... existing code ...
        
        let voice_system = VoiceSystem::new().ok();
        if voice_system.is_some() {
            log::info!("Voice system initialized");
        } else {
            log::warn!("Voice system failed to initialize");
        }
        
        Self {
            // ... existing fields ...
            voice_system,
        }
    }
}
```

---

## Step 4: Add Keybind

### File: `veloren/voxygen/src/settings.rs`

Add to `ControlSettings`:
```rust
pub struct ControlSettings {
    // ... existing fields ...
    pub voice_talk: KeyMouse,
}
```

Add default binding:
```rust
impl Default for ControlSettings {
    fn default() -> Self {
        Self {
            // ... existing bindings ...
            voice_talk: KeyMouse::Key(VirtualKeyCode::V),
        }
    }
}
```

---

## Step 5: Handle Voice Input

### File: `veloren/voxygen/src/session/mod.rs`

In the `tick()` or input handling method:

```rust
// Check for V key press
if self.key_state.pressed(self.settings.controls.voice_talk) {
    if let Some(voice_system) = &mut self.voice_system {
        // Find nearest NPC
        if let Some(npc_entity) = self.find_nearest_npc() {
            let npc_data = self.get_npc_data(npc_entity);
            
            // Start recording
            voice_system.start_recording(npc_entity.id()).ok();
            
            // Show UI indicator
            self.hud.show_recording_indicator();
        }
    }
}

// Check for V key release
if self.key_state.released(self.settings.controls.voice_talk) {
    if let Some(voice_system) = &mut self.voice_system {
        if voice_system.is_recording() {
            // Get NPC data
            let npc_data = self.get_active_npc_data();
            
            // Process voice and get response
            match voice_system.stop_and_respond(&npc_data, None) {
                Ok((text, audio)) => {
                    // Show dialogue in chat
                    self.hud.new_message(ChatType::Npc(npc_data.name.clone(), text.clone()));
                    
                    // Play audio
                    voice_system.play_audio(&audio).ok();
                }
                Err(e) => {
                    log::error!("Voice error: {}", e);
                }
            }
            
            // Hide UI indicator
            self.hud.hide_recording_indicator();
        }
    }
}
```

---

## Step 6: Add Helper Methods

```rust
impl SessionState {
    /// Find the nearest NPC within interaction range
    fn find_nearest_npc(&self) -> Option<Entity> {
        let player_pos = self.client.position()?;
        let max_distance = 10.0; // 10 meters
        
        self.client
            .state()
            .ecs()
            .entities()
            .join()
            .filter_map(|entity| {
                // Check if entity is an NPC
                let pos = self.client.position_of(entity)?;
                let distance = player_pos.distance(pos);
                
                if distance < max_distance && self.is_npc(entity) {
                    Some((entity, distance))
                } else {
                    None
                }
            })
            .min_by(|(_, d1), (_, d2)| d1.partial_cmp(d2).unwrap())
            .map(|(entity, _)| entity)
    }
    
    /// Check if entity is an NPC
    fn is_npc(&self, entity: Entity) -> bool {
        // Check for NPC components
        self.client.state().ecs().read_storage::<comp::Agent>().get(entity).is_some()
    }
    
    /// Get NPC data for voice dialogue
    fn get_npc_data(&self, entity: Entity) -> memz_veloren::NpcData {
        let ecs = self.client.state().ecs();
        
        // Get NPC name
        let name = ecs
            .read_storage::<comp::Stats>()
            .get(entity)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "Unknown".to_string());
        
        // Get NPC role/profession
        let profession = "guard"; // TODO: Get from NPC data
        
        // Get location
        let location = "town gate"; // TODO: Get from position
        
        // Get mood
        let mood = "alert"; // TODO: Get from NPC state
        
        memz_veloren::NpcData {
            name,
            profession: profession.to_string(),
            location: location.to_string(),
            mood: mood.to_string(),
            personality: memz_veloren::NpcPersonality::default(),
        }
    }
}
```

---

## Step 7: Add UI Indicators

### File: `veloren/voxygen/src/hud/mod.rs`

Add methods:
```rust
impl Hud {
    pub fn show_recording_indicator(&mut self) {
        // Show "🎤 Recording..." text
        self.recording_indicator_visible = true;
    }
    
    pub fn hide_recording_indicator(&mut self) {
        self.recording_indicator_visible = false;
    }
}
```

In the UI rendering code:
```rust
if self.recording_indicator_visible {
    Text::new("🎤 Recording...")
        .color(Color::Rgba(1.0, 0.0, 0.0, 1.0))
        .font_size(24)
        .top_left_with_margins(100.0, 20.0)
        .set(self.ids.recording_indicator, ui);
}
```

---

## Step 8: Build Veloren

```bash
cd veloren
cargo build --release
```

This will take 10-30 minutes depending on your system.

---

## Step 9: Test In-Game

1. Start Veloren:
```bash
./target/release/veloren-voxygen
```

2. Create/load a character

3. Find an NPC in a town

4. Walk close to the NPC (within 10 meters)

5. **Press and hold V key**

6. **Speak your question**

7. **Release V key**

8. Watch the NPC respond in chat and hear their voice!

---

## Simplified Quick Integration

If the above seems complex, here's a minimal integration:

### File: `veloren/voxygen/src/session/mod.rs`

```rust
// At the top
use memz_veloren::VoiceSystem;

// In SessionState
pub struct SessionState {
    voice_system: Option<VoiceSystem>,
    // ... existing fields
}

// In tick() or input handling
if input.key_pressed(VirtualKeyCode::V) {
    if let Some(voice) = &mut self.voice_system {
        voice.start_recording(0).ok();
        println!("🎤 Recording...");
    }
}

if input.key_released(VirtualKeyCode::V) {
    if let Some(voice) = &mut self.voice_system {
        let npc_data = memz_veloren::NpcData::test_guard();
        match voice.stop_and_respond(&npc_data, None) {
            Ok((text, audio)) => {
                println!("NPC: {}", text);
                voice.play_audio(&audio).ok();
            }
            Err(e) => println!("Error: {}", e),
        }
    }
}
```

---

## Alternative: Plugin Approach

If modifying Veloren directly is too complex, we can create a plugin:

### File: `veloren-voice-plugin/src/lib.rs`

```rust
use veloren_plugin_api::*;
use memz_veloren::VoiceSystem;

#[plugin_api]
pub struct VoicePlugin {
    voice: VoiceSystem,
}

impl Plugin for VoicePlugin {
    fn on_key_press(&mut self, key: Key) {
        if key == Key::V {
            self.voice.start_recording(0).ok();
        }
    }
    
    fn on_key_release(&mut self, key: Key) {
        if key == Key::V {
            let npc_data = get_nearest_npc_data();
            let (text, audio) = self.voice.stop_and_respond(&npc_data, None).unwrap();
            show_dialogue(text);
            play_audio(audio);
        }
    }
}
```

---

## Next Steps

1. **Choose integration approach:**
   - Direct integration (modify Veloren source)
   - Plugin approach (if Veloren supports plugins)
   - Standalone overlay (separate app that hooks into Veloren)

2. **Test with simple keybind first**

3. **Add NPC detection**

4. **Add UI indicators**

5. **Connect to MEMZ memory**

---

## Estimated Time

- **Minimal integration:** 1-2 hours
- **Full integration with UI:** 3-4 hours
- **MEMZ memory integration:** 1-2 days
- **Polish and testing:** 1 week

---

## Need Help?

The simulation proves everything works! Now it's just a matter of connecting it to Veloren's input system.

Would you like me to:
1. Create a detailed integration guide for Veloren's specific architecture?
2. Build a standalone overlay app that works alongside Veloren?
3. Create a minimal proof-of-concept integration?

Let me know which approach you prefer!
