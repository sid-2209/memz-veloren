# 🎮 Veloren In-Game Voice Integration Plan

## Current Status

✅ **Voice Components Working** - STT, TTS, LLM all functional  
✅ **Terminal Tests Working** - Can test voice pipeline standalone  
✅ **Veloren Integration Code Exists** - `memz-veloren` crate ready  
⏳ **Need to Connect** - Voice system → Veloren NPCs  

---

## Architecture: Voice in Veloren

```
┌─────────────────────────────────────────────────────────────┐
│                    VELOREN GAME                             │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Player presses V key near NPC                       │  │
│  └────────────────────┬─────────────────────────────────┘  │
│                       │                                     │
│                       ▼                                     │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Voice System (memz-voice)                           │  │
│  │  • Start recording from AirPods                      │  │
│  │  • Capture 5 seconds of audio                        │  │
│  └────────────────────┬─────────────────────────────────┘  │
│                       │                                     │
│                       ▼                                     │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Speech-to-Text (Whisper)                            │  │
│  │  • Transcribe player speech                          │  │
│  │  • "Hello, can you help me find the blacksmith?"     │  │
│  └────────────────────┬─────────────────────────────────┘  │
│                       │                                     │
│                       ▼                                     │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  MEMZ Memory System                                   │  │
│  │  • Retrieve NPC memories about player                │  │
│  │  • Get NPC personality, profession, mood             │  │
│  │  • Build context for LLM                             │  │
│  └────────────────────┬─────────────────────────────────┘  │
│                       │                                     │
│                       ▼                                     │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  LLM Dialogue Generation (Ollama)                    │  │
│  │  • Generate contextual NPC response                  │  │
│  │  • "Ah, the blacksmith! His forge is down the       │  │
│  │    street. I remember you helped defend the town    │  │
│  │    last week - he'll give you a good price!"        │  │
│  └────────────────────┬─────────────────────────────────┘  │
│                       │                                     │
│                       ▼                                     │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Text-to-Speech (macOS say)                          │  │
│  │  • Synthesize NPC voice                              │  │
│  │  • Play through AirPods                              │  │
│  └────────────────────┬─────────────────────────────────┘  │
│                       │                                     │
│                       ▼                                     │
│  ┌──────────────────────────────────────────────────────┐  │
│  │  Display in Game                                      │  │
│  │  • Show dialogue bubble above NPC                    │  │
│  │  • Add to chat log                                   │  │
│  │  • Store in MEMZ as episodic memory                  │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

---

## Integration Points

### 1. Input Handling (Keybind)

**File:** `veloren/voxygen/src/session/mod.rs` or `veloren/voxygen/src/hud/mod.rs`

Add keybind for voice input:

```rust
// In input handling
if input.key_pressed(VirtualKeyCode::V) && near_npc() {
    // Start voice recording
    voice_system.start_recording();
    show_recording_indicator();
}

if input.key_released(VirtualKeyCode::V) {
    // Stop recording and process
    let npc_id = get_nearest_npc();
    voice_system.stop_and_respond(npc_id);
}
```

### 2. Voice System Component

**File:** `memz-veloren/src/voice_system.rs` (new)

```rust
pub struct VoiceSystem {
    voice: VoiceDialogue,
    active_npc: Option<NpcId>,
    is_recording: bool,
}

impl VoiceSystem {
    pub fn start_recording(&mut self, npc_id: NpcId) {
        self.active_npc = Some(npc_id);
        self.voice.start_recording().ok();
        self.is_recording = true;
    }

    pub fn stop_and_respond(&mut self, memory_bank: &MemoryBank, npc_data: &NpcData) -> Result<(String, Vec<f32>)> {
        self.is_recording = false;
        
        // Build context from MEMZ
        let context = build_npc_context(memory_bank, npc_data);
        
        // Process voice input
        let (npc_text, npc_audio) = self.voice.stop_and_respond(
            &npc_data.profession,
            &npc_data.mood,
        )?;
        
        Ok((npc_text, npc_audio))
    }
}
```

### 3. NPC Context Building

**File:** `memz-veloren/src/dialogue.rs`

```rust
fn build_npc_context(memory_bank: &MemoryBank, npc_data: &NpcData) -> String {
    let mut context = format!(
        "You are {}, a {} in the town of {}. You are {}.\n\n",
        npc_data.name,
        npc_data.profession,
        npc_data.home_town,
        npc_data.mood
    );
    
    // Add recent memories about the player
    let memories = memory_bank.retrieve_recent(player_id, limit: 5);
    if !memories.is_empty() {
        context.push_str("Recent memories:\n");
        for memory in memories {
            context.push_str(&format!("- {}\n", memory.summary()));
        }
    }
    
    // Add personality traits
    context.push_str(&format!(
        "\nPersonality: openness={}, conscientiousness={}, extraversion={}\n",
        npc_data.personality.openness,
        npc_data.personality.conscientiousness,
        npc_data.personality.extraversion
    ));
    
    context
}
```

### 4. UI Integration

**File:** `veloren/voxygen/src/hud/mod.rs`

Add visual indicators:

```rust
// Recording indicator
if voice_system.is_recording() {
    ui.text("🎤 Recording...")
        .color(Color::Rgba(1.0, 0.0, 0.0, 1.0))
        .font_size(24)
        .top_left_with_margins(100.0, 20.0)
        .set(state.ids.recording_indicator, ui);
}

// Processing indicator
if voice_system.is_processing() {
    ui.text("⏳ Processing...")
        .color(Color::Rgba(1.0, 1.0, 0.0, 1.0))
        .font_size(24)
        .top_left_with_margins(100.0, 20.0)
        .set(state.ids.processing_indicator, ui);
}

// NPC dialogue bubble
if let Some((npc_id, text)) = voice_system.get_active_dialogue() {
    show_dialogue_bubble(npc_id, text, ui);
}
```

### 5. Audio Output

**File:** `memz-veloren/src/voice_system.rs`

```rust
pub fn play_npc_voice(&self, audio: &[f32], npc_position: Vec3<f32>) {
    // Option 1: Play through game's audio system
    audio_frontend.emit_sfx(audio, npc_position);
    
    // Option 2: Play directly (simpler for Phase 1)
    self.voice.play_audio(audio).ok();
}
```

---

## Implementation Steps

### Phase 1: Basic Integration (Today - 2 hours)

#### Step 1: Add Voice System to Veloren (30 min)

1. Add `memz-voice` dependency to `veloren/voxygen/Cargo.toml`:
```toml
[dependencies]
memz-voice = { path = "../memz-voice" }
```

2. Create `VoiceSystem` in `memz-veloren/src/voice_system.rs`

3. Initialize in game state:
```rust
pub struct SessionState {
    // ... existing fields
    voice_system: Option<VoiceSystem>,
}
```

#### Step 2: Add Keybind (15 min)

1. Add V key binding in `veloren/voxygen/src/settings.rs`
2. Handle key press/release in input system
3. Check if player is near NPC

#### Step 3: Connect to NPC Data (30 min)

1. Get nearest NPC when V is pressed
2. Fetch NPC data (profession, personality, mood)
3. Build context string for LLM

#### Step 4: Display Response (15 min)

1. Show dialogue bubble above NPC
2. Add to chat log
3. Play audio through speakers/AirPods

#### Step 5: Test In-Game (30 min)

1. Build Veloren with voice integration
2. Start game, find an NPC
3. Press V, speak, release V
4. Verify NPC responds with voice

### Phase 2: MEMZ Integration (Tomorrow - 3 hours)

#### Step 1: Connect Memory System

1. Initialize `MemoryBank` for each NPC
2. Store conversation in episodic memory
3. Retrieve relevant memories for context

#### Step 2: Personality-Driven Responses

1. Map Veloren personality → MEMZ personality
2. Use personality to influence LLM prompt
3. Adjust response style based on traits

#### Step 3: Emotional Responses

1. Track emotional state during conversation
2. Adjust NPC mood based on interaction
3. Remember emotional context for future

### Phase 3: Advanced Features (Next Week)

1. Voice activity detection (auto start/stop)
2. Multiple voice profiles per NPC type
3. Spatial audio (3D positioning)
4. Conversation history UI
5. Memory injection interface

---

## File Structure

```
veloren/
├── voxygen/
│   ├── src/
│   │   ├── session/
│   │   │   └── mod.rs          ← Add voice input handling
│   │   ├── hud/
│   │   │   └── mod.rs          ← Add UI indicators
│   │   └── audio/
│   │       └── sfx.rs          ← Add voice audio playback
│   └── Cargo.toml              ← Add memz-voice dependency
│
├── memz-veloren/
│   ├── src/
│   │   ├── voice_system.rs     ← NEW: Voice system integration
│   │   ├── dialogue.rs         ← Update: Add voice dialogue
│   │   ├── bridge.rs           ← Update: Bridge to voice
│   │   └── lib.rs              ← Export voice system
│   └── Cargo.toml              ← Add memz-voice dependency
│
└── memz-voice/
    └── src/
        └── lib.rs              ← Already complete!
```

---

## Testing Plan

### Test 1: Microphone with AirPods

```bash
# First, test AirPods microphone
cd memz-voice
cargo run --example test_microphone --release
```

**Expected:**
- AirPods listed as input device
- Audio level shows when speaking
- "Audio level is GOOD!"

### Test 2: Voice Pipeline with AirPods

```bash
cargo run --example test_full --release
```

**Expected:**
- Speak through AirPods
- Transcription appears correctly
- NPC response plays through AirPods

### Test 3: In-Game Voice (After Integration)

1. Build Veloren: `cargo build --release`
2. Start game: `./target/release/veloren-voxygen`
3. Find an NPC in a town
4. Press and hold V
5. Speak: "Hello, can you help me?"
6. Release V
7. See dialogue bubble and hear response

---

## Quick Start: In-Game Testing

### Option 1: Minimal Integration (Fastest - 1 hour)

Create a simple plugin that:
1. Detects V key press
2. Calls voice system
3. Shows response in chat
4. No UI changes needed

### Option 2: Full Integration (Recommended - 2 hours)

Full integration with:
1. Keybind system
2. UI indicators
3. Dialogue bubbles
4. Audio playback
5. MEMZ memory storage

---

## Code Snippets

### Minimal Integration Example

```rust
// In veloren/voxygen/src/session/mod.rs

use memz_voice::VoiceDialogue;

pub struct SessionState {
    voice: Option<VoiceDialogue>,
}

impl SessionState {
    fn handle_voice_input(&mut self, npc_id: NpcId) {
        if let Some(voice) = &mut self.voice {
            // Start recording
            voice.start_recording().ok();
            
            // Wait 5 seconds (or until key release)
            std::thread::sleep(Duration::from_secs(5));
            
            // Get NPC data
            let npc = self.get_npc(npc_id);
            
            // Process and respond
            match voice.stop_and_respond(&npc.profession, &npc.mood) {
                Ok((text, audio)) => {
                    // Show in chat
                    self.chat.add_message(format!("{}: {}", npc.name, text));
                    
                    // Play audio
                    voice.play_audio(&audio).ok();
                }
                Err(e) => {
                    self.chat.add_message(format!("Voice error: {}", e));
                }
            }
        }
    }
}
```

---

## AirPods Configuration

### macOS Settings

1. **Connect AirPods:**
   - Open Bluetooth settings
   - Connect AirPods

2. **Set as Input Device:**
   - System Settings → Sound → Input
   - Select "AirPods Pro" (or your model)
   - Adjust input volume

3. **Set as Output Device:**
   - System Settings → Sound → Output
   - Select "AirPods Pro"
   - Adjust output volume

4. **Test:**
```bash
# Test microphone
say "Testing AirPods microphone"

# Test recording
cd memz-voice
cargo run --example test_microphone --release
```

---

## Performance Considerations

### Latency Budget

| Component | Target | Notes |
|-----------|--------|-------|
| Key press → Recording | < 100ms | Instant feedback |
| Recording (5s) | 5000ms | Fixed duration |
| STT (Whisper) | < 1000ms | Depends on audio length |
| MEMZ retrieval | < 100ms | Cached memories |
| LLM generation | 2000-3000ms | Ollama inference |
| TTS synthesis | < 500ms | macOS say |
| Audio playback | 2000-3000ms | Depends on response length |
| **Total** | **~10-12s** | Acceptable for turn-based dialogue |

### Optimization Strategies

1. **Async Processing:**
   - Don't block game loop
   - Process voice in background thread
   - Show "processing" indicator

2. **Caching:**
   - Cache NPC memory banks
   - Cache common responses
   - Preload voice models

3. **Streaming:**
   - Stream TTS audio as it's generated
   - Start playback before full generation
   - Reduce perceived latency

---

## Next Steps

### Immediate (Today)

1. **Test AirPods:**
   ```bash
   cd memz-voice
   cargo run --example test_microphone --release
   ```

2. **Test Voice Pipeline:**
   ```bash
   cargo run --example test_full --release
   ```

3. **Verify Everything Works:**
   - Speak through AirPods
   - See correct transcription
   - Hear NPC response

### Short-term (This Week)

1. **Create Voice System Integration:**
   - Add `voice_system.rs` to `memz-veloren`
   - Connect to Veloren input system
   - Add UI indicators

2. **Test In-Game:**
   - Build Veloren with voice
   - Find NPC in game
   - Test voice conversation

3. **Iterate:**
   - Fix microphone issues
   - Improve response quality
   - Add MEMZ context

---

## Success Criteria

✅ **Phase 1 Complete When:**
- Can press V near NPC in-game
- Speak through AirPods
- NPC responds with voice
- Response plays through AirPods
- Dialogue shows in game

✅ **Phase 2 Complete When:**
- NPC remembers previous conversations
- Responses are contextual and intelligent
- Personality affects dialogue style
- Emotional state influences responses

✅ **Phase 3 Complete When:**
- Voice activity detection works
- Multiple NPCs have unique voices
- Spatial audio positioning
- Full conversation history
- Memory injection interface

---

## Let's Start!

**First, test your AirPods:**

```bash
cd memz-voice
cargo run --example test_microphone --release
```

Then we'll integrate into Veloren! 🎮🎙️
