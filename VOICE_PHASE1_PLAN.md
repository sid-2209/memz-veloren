# 🎙️ Voice NPC - Phase 1: Voice-Only Implementation

**Goal:** Get basic voice dialogue working WITHOUT MEMZ integration  
**Timeline:** 2-3 weeks  
**Scope:** Player speaks → NPC responds with voice (simple, generic responses)

---

## What We're Building (Phase 1)

```
Player Voice → Whisper STT → Simple Prompt → Llama LLM → Kokoro TTS → Audio Output
                                                              ↓
                                                      Dialogue Box (text)
```

**Simplified from full system:**
- ❌ No MEMZ memory retrieval
- ❌ No personality system
- ❌ No sentiment tracking
- ✅ Basic voice input/output
- ✅ Generic NPC responses
- ✅ Spatial audio
- ✅ Text fallback

---

## Phase 1 Milestones

### Milestone 1.1: Standalone Voice Echo (Week 1)
**Goal:** Prove voice I/O works on your Mac M4

**Tasks:**
1. Create `memz-voice` crate
2. Implement microphone capture (cpal)
3. Integrate Whisper STT
4. Integrate Kokoro TTS
5. Test: Speak → Transcribe → Synthesize → Play

**Deliverable:** CLI app that echoes your voice

---

### Milestone 1.2: Simple LLM Responses (Week 1-2)
**Goal:** Add basic NPC dialogue generation

**Tasks:**
1. Integrate Llama-3.2-1B
2. Create simple prompt template (no MEMZ context)
3. Test response quality
4. Optimize latency

**Deliverable:** CLI app with NPC personality responses

---

### Milestone 1.3: Veloren Integration (Week 2-3)
**Goal:** Voice dialogue working in-game

**Tasks:**
1. Add voice system to Veloren
2. Hook PTT keybind (V key)
3. Connect to spatial audio
4. Display text in dialogue box
5. Test with NPCs in singleplayer

**Deliverable:** In-game voice dialogue (generic responses)

---

## Simplified Architecture

### Components

1. **VoiceInput** - Microphone → Whisper → Text
2. **VoiceOutput** - Text → Kokoro → Audio
3. **SimpleDialogue** - Text → Llama → Response (no MEMZ)
4. **VelorenBridge** - Connect to game systems

### Simple Prompt Template (No MEMZ)

```
You are a {profession} NPC in a fantasy RPG.
You are {friendly/neutral/grumpy}.

Player said: "{player_input}"

Respond in character with 1-2 sentences.
```

**Example:**
```
You are a blacksmith NPC in a fantasy RPG.
You are friendly.

Player said: "Do you have any weapons for sale?"

Respond in character with 1-2 sentences.
```

**LLM Output:** "Aye, I've got some fine blades fresh from the forge! Take a look at what I have in stock."

---

## Implementation Steps

### Step 1: Create `memz-voice` Crate

```bash
cd /Users/siddhartha/Downloads/dev/Project\ Vyuh/memz
cargo new --lib memz-voice
cd memz-voice
```

### Step 2: Add Dependencies

Edit `memz-voice/Cargo.toml`:

```toml
[package]
name = "memz-voice"
version = "0.1.0"
edition = "2021"

[dependencies]
# Audio I/O
cpal = "0.15"
hound = "3.5"

# Speech-to-Text
whisper-rs = { version = "0.12", features = ["metal"] }

# LLM Inference
llama-cpp-rs = { version = "0.3", features = ["metal"] }

# TTS - We'll use Python binding initially for Kokoro
pyo3 = { version = "0.22", features = ["auto-initialize"] }

# Utilities
anyhow = "1.0"
thiserror = "1.0"
log = "0.4"
```

### Step 3: Implement Core Modules

**File Structure:**
```
memz-voice/
├── src/
│   ├── lib.rs           # Public API
│   ├── stt.rs           # Speech-to-Text (Whisper)
│   ├── tts.rs           # Text-to-Speech (Kokoro)
│   ├── llm.rs           # LLM Dialogue (Llama)
│   ├── simple_dialogue.rs  # Simple prompt templates (no MEMZ)
│   └── error.rs         # Error types
├── examples/
│   ├── test_stt.rs      # Test microphone → text
│   ├── test_tts.rs      # Test text → audio
│   ├── test_llm.rs      # Test LLM responses
│   └── test_full.rs     # Full voice dialogue loop
└── Cargo.toml
```

---

## Testing Strategy

### Test 1: Microphone Capture
```bash
cargo run --example test_stt
# Speak into mic, see transcription
```

### Test 2: TTS Synthesis
```bash
cargo run --example test_tts
# Type text, hear synthesized voice
```

### Test 3: LLM Responses
```bash
cargo run --example test_llm
# Type player input, see NPC response
```

### Test 4: Full Loop
```bash
cargo run --example test_full
# Speak → NPC responds with voice
```

---

## Simplified Veloren Integration

### Add to `memz-veloren/Cargo.toml`:
```toml
[dependencies]
memz-voice = { path = "../memz-voice" }
```

### Create `memz-veloren/src/simple_voice.rs`:

```rust
use memz_voice::{VoiceInput, VoiceOutput, SimpleDialogue};

pub struct SimpleVoiceSystem {
    voice_input: VoiceInput,
    voice_output: VoiceOutput,
    dialogue: SimpleDialogue,
    is_recording: bool,
}

impl SimpleVoiceSystem {
    pub fn new() -> Result<Self> {
        Ok(Self {
            voice_input: VoiceInput::new("models/whisper-tiny.en.bin")?,
            voice_output: VoiceOutput::new("models/Kokoro-82M")?,
            dialogue: SimpleDialogue::new("models/llama-3.2-1b-q4.gguf")?,
            is_recording: false,
        })
    }

    pub fn handle_ptt_press(&mut self) {
        self.voice_input.start_recording();
        self.is_recording = true;
    }

    pub fn handle_ptt_release(&mut self, npc_profession: &str) -> Result<(String, Vec<f32>)> {
        self.is_recording = false;
        
        // 1. Transcribe player speech
        let player_text = self.voice_input.stop_and_transcribe()?;
        
        // 2. Generate NPC response (simple prompt, no MEMZ)
        let npc_text = self.dialogue.generate_response(
            player_text,
            npc_profession,
            "friendly", // Default mood
        )?;
        
        // 3. Synthesize NPC voice
        let npc_audio = self.voice_output.synthesize(&npc_text)?;
        
        Ok((npc_text, npc_audio))
    }
}
```

---

## What's Deferred to Phase 2 (MEMZ Integration)

- ❌ Memory retrieval (episodic, semantic, emotional, social)
- ❌ Personality-driven responses (OCEAN traits)
- ❌ Sentiment tracking
- ❌ Context-aware greetings
- ❌ Gossip and social memory
- ❌ Player memory injection

**Phase 2 will add:**
```rust
// Instead of simple prompt:
let npc_text = self.dialogue.generate_response(player_text, npc_profession, "friendly");

// We'll use MEMZ context:
let context = memz_dialogue::assemble_dialogue_context(&memory_bank, &npc_personality);
let npc_text = self.dialogue.generate_response_with_context(player_text, context);
```

---

## Success Criteria (Phase 1)

- ✅ Player can hold V and speak
- ✅ Speech is transcribed accurately
- ✅ NPC generates relevant response
- ✅ NPC voice plays from correct 3D position
- ✅ Text appears in dialogue box
- ✅ Latency <500ms
- ✅ No crashes or memory leaks
- ✅ Works with multiple NPCs

---

## Next Steps

1. **Download Models** (see below)
2. **Create `memz-voice` crate** (I'll help you build it)
3. **Test standalone** (CLI examples)
4. **Integrate with Veloren** (simple voice system)
5. **Test in-game** (talk to NPCs)
6. **Iterate** (fix bugs, optimize)

Then Phase 2: Add MEMZ integration for context-aware dialogue!

---

## Model Downloads (Phase 1)

```bash
mkdir -p models
cd models

# Whisper-tiny (~50MB)
curl -L -o whisper-tiny.en.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin

# Llama-3.2-1B (~700MB)
curl -L -o llama-3.2-1b-q4.gguf \
  https://huggingface.co/bartowski/Llama-3.2-1B-Instruct-GGUF/resolve/main/Llama-3.2-1B-Instruct-Q4_K_M.gguf

# Kokoro TTS (~100MB)
git clone https://huggingface.co/hexgrad/Kokoro-82M
```

---

**Ready to start building!** Let's create the `memz-voice` crate first.
