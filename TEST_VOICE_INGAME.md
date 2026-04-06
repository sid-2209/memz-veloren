# 🎮 Test Voice In-Game - Complete Guide

## What We've Built

✅ **Voice Components** - STT, TTS, LLM all working  
✅ **Veloren Integration Code** - `VoiceSystem` ready  
✅ **In-Game Simulation** - Test without modifying Veloren  
⏳ **Full Integration** - Connect to actual Veloren (next step)  

---

## Quick Test: Simulated In-Game Experience

This test simulates exactly how voice will work in Veloren, without needing to modify the game yet.

### Step 1: Connect AirPods

1. Open Bluetooth settings
2. Connect your AirPods
3. Set as input device:
   - System Settings → Sound → Input
   - Select "AirPods Pro" (or your model)
4. Set as output device:
   - System Settings → Sound → Output
   - Select "AirPods Pro"

### Step 2: Test AirPods Microphone

```bash
cd memz-voice
cargo run --example test_microphone --release
```

**Expected output:**
```
Available Input Devices:
  1. AirPods Pro (48000 Hz, 1 ch)
  2. MacBook Pro Microphone (48000 Hz, 2 ch)

Default Input Device:
  AirPods Pro

Recording for 3 seconds...
Speak now: 'Hello, this is a test!'

✅ Audio level is GOOD!
   Your microphone is working correctly!
```

### Step 3: Run In-Game Simulation

```bash
cd memz-veloren
cargo run --example test_voice_ingame --release
```

**What happens:**
1. You see a list of NPCs (Guard, Merchant, Blacksmith)
2. Select an NPC (1-3)
3. Press Enter to "press V key"
4. Speak into your AirPods for 5 seconds
5. NPC responds with voice through your AirPods!

**Example interaction:**
```
═══════════════════════════════════════════════════════════
🎮 You approach: Guard at Town Gate
═══════════════════════════════════════════════════════════

NPC Info:
  Name: Guard
  Profession: guard
  Location: town gate
  Mood: alert

🎮 [Press V to talk]

Press Enter to start recording: [Enter]
🎤 Recording... (speak now, will record for 5 seconds)

[You speak: "Hello, can you help me find the blacksmith?"]

⏳ Processing your voice input...

💬 Guard says:
   "The blacksmith? His forge is down the street, past the 
    market. You can't miss the smoke and the sound of hammering."

🔊 Playing NPC voice...
✅ Done!
```

---

## How It Works (Architecture)

```
┌─────────────────────────────────────────────────────────┐
│  YOU (with AirPods)                                     │
└────────────┬────────────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────────────┐
│  "Hello, can you help me find the blacksmith?"          │
│  (Spoken through AirPods microphone)                    │
└────────────┬────────────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────────────┐
│  VoiceSystem::start_recording()                         │
│  • Captures audio from AirPods                          │
│  • Records for 5 seconds                                │
└────────────┬────────────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────────────┐
│  Whisper STT                                            │
│  • Transcribes: "Hello, can you help me find the        │
│    blacksmith?"                                         │
└────────────┬────────────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────────────┐
│  NPC Context Building                                   │
│  • Name: Guard                                          │
│  • Profession: guard                                    │
│  • Location: town gate                                  │
│  • Mood: alert                                          │
│  • Personality: conscientious, low extraversion         │
└────────────┬────────────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────────────┐
│  Ollama LLM (llama3.2:1b)                               │
│  • Generates contextual response                        │
│  • "The blacksmith? His forge is down the street..."    │
└────────────┬────────────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────────────┐
│  macOS TTS                                              │
│  • Synthesizes voice                                    │
│  • Plays through AirPods                                │
└────────────┬────────────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────────────┐
│  YOU (hear NPC response through AirPods)                │
└─────────────────────────────────────────────────────────┘
```

---

## Testing Different NPCs

The simulation includes 3 NPC types with different personalities:

### 1. Guard (Alert, Conscientious)
- **Profession:** guard
- **Mood:** alert
- **Personality:** High conscientiousness, low extraversion
- **Expected responses:** Professional, brief, helpful but formal

**Try asking:**
- "What's your job here?"
- "Have you seen any trouble?"
- "Can you help me?"

### 2. Merchant (Friendly, Extraverted)
- **Profession:** merchant
- **Mood:** friendly
- **Personality:** High extraversion, high agreeableness
- **Expected responses:** Warm, talkative, eager to help

**Try asking:**
- "What are you selling?"
- "Do you have any good deals?"
- "Tell me about your wares"

### 3. Blacksmith (Busy, Focused)
- **Profession:** blacksmith
- **Mood:** busy
- **Personality:** High conscientiousness, low extraversion
- **Expected responses:** Direct, focused on work, practical

**Try asking:**
- "Can you repair my sword?"
- "What weapons do you make?"
- "How long have you been a blacksmith?"

---

## Troubleshooting

### "No audio captured"

**Problem:** Microphone not working

**Solutions:**
1. Check AirPods are connected and selected as input
2. System Settings → Privacy & Security → Microphone → Enable for Terminal
3. Increase microphone volume in Sound settings
4. Speak louder and closer to AirPods

### "Transcription: [BLANK_AUDIO]"

**Problem:** Audio captured but silent

**Solutions:**
1. Wrong microphone selected (check System Settings → Sound → Input)
2. AirPods microphone muted
3. Not speaking during recording
4. Background noise too loud

### "Ollama not responding"

**Problem:** LLM not generating responses

**Solutions:**
```bash
# Check Ollama is running
ollama list

# Restart if needed
killall ollama
ollama serve &

# Verify model
ollama pull llama3.2:1b
```

### "No audio output"

**Problem:** Can't hear NPC voice

**Solutions:**
1. Check AirPods are selected as output device
2. Increase volume
3. Test with: `say "Hello"`
4. Check AirPods battery

---

## Performance Metrics

| Component | Time | Notes |
|-----------|------|-------|
| Recording | 5000ms | Fixed duration |
| STT (Whisper) | ~500ms | Depends on audio length |
| LLM (Ollama) | 2-3s | Depends on response length |
| TTS (macOS) | ~200ms | Depends on text length |
| Audio playback | 2-3s | Depends on response length |
| **Total** | **~8-12s** | Acceptable for dialogue |

---

## Next Steps

### Phase 1: Full Veloren Integration (2-3 hours)

1. **Add Voice System to Veloren:**
   - Modify `veloren/voxygen/Cargo.toml`
   - Add `memz-veloren` dependency
   - Initialize `VoiceSystem` in game state

2. **Add Keybind:**
   - Add V key binding
   - Detect nearby NPCs
   - Start/stop recording on key press/release

3. **Add UI:**
   - Recording indicator (🎤)
   - Processing indicator (⏳)
   - Dialogue bubble above NPC
   - Chat log integration

4. **Test In-Game:**
   - Build Veloren
   - Find NPC in game
   - Press V and speak
   - Verify response

### Phase 2: MEMZ Integration (1-2 days)

1. **Connect Memory System:**
   - Initialize `MemoryBank` for each NPC
   - Store conversations as episodic memories
   - Retrieve relevant memories for context

2. **Personality-Driven Dialogue:**
   - Map Veloren personality → MEMZ
   - Influence LLM prompts
   - Adjust response style

3. **Emotional Responses:**
   - Track emotional state
   - Update NPC mood
   - Remember emotional context

### Phase 3: Advanced Features (1 week)

1. Voice activity detection
2. Multiple voice profiles
3. Spatial audio (3D positioning)
4. Conversation history UI
5. Memory injection interface

---

## Code Structure

```
memz-veloren/
├── src/
│   ├── voice_system.rs      ← Voice integration (NEW!)
│   ├── dialogue.rs           ← Dialogue generation
│   ├── bridge.rs             ← Veloren ↔ MEMZ bridge
│   └── lib.rs                ← Exports
├── examples/
│   └── test_voice_ingame.rs  ← In-game simulation (NEW!)
└── Cargo.toml                ← Dependencies

memz-voice/
├── src/
│   ├── stt.rs                ← Speech-to-text (Whisper)
│   ├── tts.rs                ← Text-to-speech (macOS)
│   ├── llm.rs                ← LLM dialogue (Ollama)
│   └── lib.rs                ← Voice pipeline
└── examples/
    ├── test_microphone.rs    ← Test microphone
    ├── test_full.rs          ← Test full pipeline
    └── ...
```

---

## Success Criteria

### ✅ Phase 1 Complete When:
- [x] Voice components working (STT, TTS, LLM)
- [x] AirPods microphone tested
- [x] In-game simulation working
- [ ] Full Veloren integration
- [ ] Can talk to NPCs in actual game

### ✅ Phase 2 Complete When:
- [ ] NPCs remember conversations
- [ ] Responses are contextual
- [ ] Personality affects dialogue
- [ ] Emotional state tracked

### ✅ Phase 3 Complete When:
- [ ] Voice activity detection
- [ ] Multiple voice profiles
- [ ] Spatial audio
- [ ] Full conversation history
- [ ] Memory injection UI

---

## Quick Commands

```bash
# Test AirPods microphone
cd memz-voice
cargo run --example test_microphone --release

# Test voice pipeline
cargo run --example test_full --release

# Test in-game simulation
cd memz-veloren
cargo run --example test_voice_ingame --release

# Build Veloren with voice (after integration)
cd veloren
cargo build --release

# Run Veloren
./target/release/veloren-voxygen
```

---

## 🎉 Ready to Test!

**Run the in-game simulation:**

```bash
cd memz-veloren
cargo run --example test_voice_ingame --release
```

1. Connect your AirPods
2. Select an NPC
3. Press Enter to start recording
4. Speak your question
5. Hear the NPC respond!

This is exactly how it will work in the actual game! 🎮🎙️
