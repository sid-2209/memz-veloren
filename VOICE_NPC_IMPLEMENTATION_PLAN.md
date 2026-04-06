# 🎙️ Voice-Based NPC Dialogue System for Veloren + MEMZ

**Project Goal:** Enable real-time voice conversations with Veloren's 1,976 NPCs using speech-to-text (STT) and text-to-speech (TTS), integrated with MEMZ memory system for context-aware, personalized dialogue.

**Date:** March 22, 2026  
**Target Platform:** Mac Mini M4 (Apple Silicon), cross-platform compatible  
**Inspiration:** AI Voice Infrastructure project (project_overview copy.md, model_comparison.md)

---

## Executive Summary

This document outlines a comprehensive plan to add voice-based NPC interaction to Veloren, allowing players to:
1. **Speak to NPCs** using their microphone (voice input)
2. **Hear NPCs respond** with synthesized voice (voice output)
3. **See dialogue** in the existing message box (visual fallback)
4. **Experience context-aware conversations** powered by MEMZ memory system

The system will be **local-first**, **low-latency** (<500ms), and **lightweight** (<1GB total footprint) to ensure smooth gameplay.

---

## 1. Architecture Overview

### 1.1 High-Level Pipeline

```
Player Voice Input → VAD → STT → MEMZ Context → LLM → TTS → Audio Output
                                      ↓
                              Dialogue Message Box (existing)
```

### 1.2 Component Breakdown

| Component | Technology | Size | Latency | Purpose |
|-----------|-----------|------|---------|---------|
| **VAD** | Silero VAD | ~1MB | <10ms | Detect when player is speaking |
| **STT** | Whisper-tiny.en / Moonshine | ~50MB | 50-150ms | Convert speech to text |
| **Context** | MEMZ Memory System | N/A | <50ms | Retrieve NPC memories, personality |
| **LLM** | Llama-3.2-1B-Instruct | ~700MB | 100-200ms | Generate NPC response |
| **TTS** | Kokoro TTS (82M) | ~100MB | 50-100ms | Convert text to speech |
| **Audio** | Veloren's rodio system | N/A | <10ms | Play spatial 3D audio |

**Total Footprint:** ~850MB  
**Total Latency:** 250-510ms (sub-second, feels real-time)

---

## 2. Inspiration Analysis

### 2.1 Key Insights from "AI Voice Infrastructure" Project

The inspiration documents reveal critical design principles:


#### A. Edge-First Constraint (Local Processing)
- **Problem:** Cloud-based APIs add latency (200-500ms network round-trip) and require internet
- **Solution:** Run all AI models locally on the player's machine
- **Benefit:** Zero network latency, works offline, no API costs, privacy-preserving

#### B. Sub-1GB Model Constraint (Game Download Size)
- **Problem:** Adding 4-5GB of AI models to a game is unacceptable for downloads
- **Solution:** Use ultra-lightweight models (Whisper-tiny, Llama-1B, Kokoro-82M)
- **Benefit:** Total AI footprint <1GB, acceptable for game distribution

#### C. Gaming-Specific Fine-Tuning
- **Problem:** Generic LLMs don't understand gaming slang, fast speech, or fragmented sentences
- **Solution:** Fine-tune models on Twitch chat, esports comms, gaming datasets
- **Benefit:** NPCs understand "aggro", "gank", "one-shot", etc. naturally

#### D. Semantic Interruption Handling (Full-Duplex)
- **Problem:** Traditional systems are half-duplex (NPC talks, you wait)
- **Solution:** NPC listens while speaking, detects real interruptions vs. background noise
- **Benefit:** Natural conversation flow, feels like talking to a real person

#### E. Spatial Audio Integration
- **Problem:** Voice should come from NPC's position in 3D space
- **Solution:** Feed TTS output into game's spatial audio system (Veloren's rodio)
- **Benefit:** Immersive, directional audio with distance attenuation

#### F. Lore Guardrails (RAG)
- **Problem:** LLMs hallucinate facts about the game world
- **Solution:** Inject NPC's MEMZ memories + world lore into LLM context
- **Benefit:** NPCs stay in-character, don't invent false information

---

### 2.2 Comparison: BUD-E Whisper vs. Lightweight Edge Models

The model comparison document highlights why we should NOT use BUD-E Whisper:

| Aspect | BUD-E Whisper Pipeline | Our Lightweight Pipeline |
|--------|------------------------|--------------------------|
| **Latency** | 400-800ms (text conversion bottleneck) | 250-350ms (optimized) |
| **Size** | ~4.5GB total | ~850MB total |
| **Mobile** | Not viable (too large) | Excellent (runs on M4) |
| **Emotion** | Text tags `[Sighs heavily]` | Prosodic analysis (acoustic) |
| **Interruptions** | Half-duplex (clunky) | Full-duplex (seamless) |

**Verdict:** Use micro-cascaded pipeline (Whisper-tiny + Llama-1B + Kokoro) for best balance of quality, size, and latency.

---

## 3. Current Veloren Systems Analysis

### 3.1 Existing Dialogue System

**Location:** `veloren/rtsim/src/data/npc.rs`, `veloren/client/src/lib.rs`

**Current Flow:**
1. Player presses **E** near NPC
2. Client calls `npc_interact(npc_entity)`
3. NPC starts `DialogueSession` with `dialogue_start(target)`
4. NPC sends `Dialogue` messages with `DialogueKind`:
   - `Start` - Begin conversation
   - `Question { tag, msg, responses }` - Ask with multiple choice
   - `Response { tag, response }` - Player's answer
   - `Statement { msg, given_item }` - NPC statement
   - `End` - End conversation
5. Client displays dialogue in HUD message box

**Key Insight:** Veloren already has a robust dialogue system! We just need to add voice I/O layers.

---

### 3.2 Existing Audio System

**Location:** `veloren/voxygen/src/audio/mod.rs`

**Current Capabilities:**
- **Spatial 3D Audio:** `set_listener_pos(pos, ori)` for player position
- **SFX Channels:** 64 channels for sound effects
- **Ambience:** Background sounds (rain, wind, etc.)
- **Music:** Background music system
- **Subtitles:** `emit_subtitle(text)` for accessibility
- **Volume Control:** Master, music, SFX, ambience volumes

**Audio Backend:** `rodio` (Rust audio library) with `kira` for advanced features

**Key Insight:** Veloren's audio system is perfect for spatial NPC voices! We can use `emit_sfx()` with 3D positioning.

---

### 3.3 MEMZ Integration Points

**Location:** `memz-veloren/src/dialogue.rs`

**Current MEMZ Dialogue Functions:**
- `generate_greeting(bank, npc_personality)` - Context-aware greetings
- `generate_response_rule_based(bank, player_input)` - Memory-based responses
- `generate_sentiment_response(bank, npc_personality)` - Emotional responses
- `generate_gossip_text(bank, npc_personality)` - Social memory gossip
- `assemble_dialogue_context(bank, npc_personality)` - Full context for LLM

**Key Insight:** MEMZ already provides rich context for NPC dialogue! We just need to connect it to voice I/O.

---

## 4. Technical Implementation Plan

### Phase 1: Foundation (Voice Input/Output)

**Goal:** Get basic voice I/O working without LLM integration

#### 4.1 Add Voice Input (STT)

**New Crate:** `memz-voice` (Rust)

**Dependencies:**
```toml
[dependencies]
whisper-rs = "0.12"  # Rust bindings for Whisper.cpp
cpal = "0.15"        # Cross-platform audio I/O
hound = "3.5"        # WAV file handling
```

**Components:**
1. **Microphone Capture:**
   - Use `cpal` to capture audio from default microphone
   - Buffer audio in 16kHz mono PCM format (Whisper requirement)
   - Implement push-to-talk (PTT) keybind (e.g., V key)

2. **Voice Activity Detection (VAD):**
   - Use Silero VAD (tiny ~1MB model)
   - Detect when player starts/stops speaking
   - Trim silence from audio buffer

3. **Speech-to-Text (STT):**
   - Use `whisper-rs` with `tiny.en` model (~50MB)
   - Convert audio buffer to text
   - Return transcribed text to game

**API:**
```rust
pub struct VoiceInput {
    whisper_ctx: WhisperContext,
    audio_buffer: Vec<f32>,
    is_recording: bool,
}

impl VoiceInput {
    pub fn new() -> Result<Self>;
    pub fn start_recording(&mut self);
    pub fn stop_recording(&mut self) -> Result<String>; // Returns transcribed text
}
```


#### 4.2 Add Voice Output (TTS)

**Dependencies:**
```toml
[dependencies]
# Option A: Kokoro TTS (recommended)
kokoro-tts = "0.1"  # Lightweight TTS (82M params)

# Option B: Piper TTS (alternative)
piper-rs = "0.2"    # Fast neural TTS
```

**Components:**
1. **Text-to-Speech Engine:**
   - Use Kokoro TTS (82M parameters, ~100MB)
   - Generate audio from NPC response text
   - Support multiple voices (male/female, different ages)

2. **Audio Playback:**
   - Convert TTS output to PCM audio
   - Feed into Veloren's `AudioFrontend::emit_sfx()`
   - Apply 3D spatial positioning based on NPC location

**API:**
```rust
pub struct VoiceOutput {
    tts_engine: KokoroTTS,
    voice_profiles: HashMap<NpcId, VoiceProfile>,
}

impl VoiceOutput {
    pub fn new() -> Result<Self>;
    pub fn synthesize(&self, text: &str, voice: VoiceProfile) -> Result<Vec<f32>>;
    pub fn play_at_position(&self, audio: Vec<f32>, pos: Vec3<f32>);
}
```

---

### Phase 2: LLM Integration (Context-Aware Dialogue)

**Goal:** Connect voice I/O to MEMZ + LLM for intelligent responses

#### 4.3 LLM Inference Engine

**Dependencies:**
```toml
[dependencies]
llama-cpp-rs = "0.3"  # Rust bindings for llama.cpp
```

**Components:**
1. **Model Loading:**
   - Download Llama-3.2-1B-Instruct GGUF (~700MB)
   - Load model on game startup (or lazy load on first NPC interaction)
   - Use 4-bit quantization for speed

2. **Prompt Engineering:**
   - Inject NPC personality (OCEAN traits)
   - Inject MEMZ memories (episodic, semantic, emotional, social)
   - Inject world lore (faction, location, quests)
   - Format as chat template

**Prompt Template:**
```
You are {npc_name}, a {profession} in {location}.

Personality:
- Openness: {openness}/255
- Conscientiousness: {conscientiousness}/255
- Extraversion: {extraversion}/255
- Agreeableness: {agreeableness}/255
- Neuroticism: {neuroticism}/255

Recent Memories:
{episodic_memories}

Knowledge:
{semantic_memories}

Feelings toward {player_name}:
{sentiment_description}

Social Context:
{gossip_memories}

Player said: "{player_input}"

Respond in character as {npc_name}. Keep it brief (1-3 sentences).
```

**API:**
```rust
pub struct DialogueLLM {
    llama_ctx: LlamaContext,
}

impl DialogueLLM {
    pub fn new(model_path: &str) -> Result<Self>;
    pub fn generate_response(
        &self,
        npc_context: &NpcContext,
        player_input: &str,
    ) -> Result<String>;
}
```

---

### Phase 3: Veloren Integration

**Goal:** Wire everything into Veloren's game loop

#### 4.4 Voice Dialogue System

**Location:** `memz-veloren/src/voice_dialogue.rs` (new file)

**Components:**
1. **Voice Interaction Manager:**
   - Detect when player is near NPC and presses PTT key
   - Start recording player voice
   - Transcribe speech to text
   - Query MEMZ for NPC context
   - Generate LLM response
   - Synthesize TTS audio
   - Play audio at NPC position
   - Display text in dialogue box (fallback)

2. **State Machine:**
   ```
   Idle → Recording → Transcribing → Thinking → Speaking → Idle
   ```

3. **Keybinds:**
   - **V (hold):** Push-to-talk (record voice)
   - **E:** Traditional text dialogue (fallback)
   - **ESC:** Cancel voice interaction

**API:**
```rust
pub struct VoiceDialogueSystem {
    voice_input: VoiceInput,
    voice_output: VoiceOutput,
    dialogue_llm: DialogueLLM,
    active_sessions: HashMap<NpcId, VoiceSession>,
}

impl VoiceDialogueSystem {
    pub fn new() -> Result<Self>;
    
    pub fn start_voice_interaction(&mut self, npc_id: NpcId, player_id: EntityId);
    pub fn update(&mut self, dt: f32);
    pub fn handle_ptt_press(&mut self);
    pub fn handle_ptt_release(&mut self);
}
```

---

### Phase 4: Advanced Features

**Goal:** Polish and edge case handling

#### 4.5 Semantic Interruption Handling

**Problem:** Player interrupts NPC mid-sentence

**Solution:**
1. While NPC is speaking (TTS playing), continue listening for player voice
2. Use VAD to detect if player starts speaking
3. If detected, immediately stop TTS playback
4. Start recording player's interruption
5. Process as new input

**Implementation:**
```rust
pub struct InterruptionDetector {
    vad: SileroVAD,
    is_npc_speaking: bool,
}

impl InterruptionDetector {
    pub fn check_interruption(&mut self, audio_chunk: &[f32]) -> bool {
        if self.is_npc_speaking && self.vad.is_speech(audio_chunk) {
            return true; // Player interrupted!
        }
        false
    }
}
```

---

#### 4.6 Multi-NPC Conversations

**Problem:** Multiple NPCs nearby, which one to talk to?

**Solution:**
1. Raycast from player camera to find NPC in crosshair
2. Highlight selected NPC with UI indicator
3. Only that NPC responds to voice input

**Alternative:** Voice command "Hey [NPC name]!" to select target

---

#### 4.7 Environmental Noise Handling

**Problem:** In-game sounds (combat, explosions) interfere with STT

**Solution:**
1. Mute game audio during voice recording (optional setting)
2. Use noise suppression in Whisper (built-in)
3. Increase VAD threshold in noisy environments

---

#### 4.8 Accent & Slang Support

**Problem:** Whisper may not understand gaming slang or accents

**Solution:**
1. Fine-tune Whisper on gaming datasets (Twitch, esports comms)
2. Add custom vocabulary for Veloren-specific terms (e.g., "rtsim", "voxygen")
3. Use Whisper's language detection for multilingual support

---

#### 4.9 Toxicity & Safety

**Problem:** Players may say inappropriate things to NPCs

**Solution:**
1. Add lightweight toxicity classifier (~50MB)
2. If toxic input detected, NPC responds with:
   - "I don't appreciate that tone."
   - NPC walks away (ends dialogue)
   - Sentiment decreases significantly
3. Log toxic interactions for moderation (optional)

---

## 5. Implementation Roadmap

### Milestone 1: Proof of Concept (2-3 weeks)

**Goal:** Basic voice I/O working in standalone Rust app

**Tasks:**
- [ ] Set up `memz-voice` crate
- [ ] Implement microphone capture with `cpal`
- [ ] Integrate Whisper-tiny for STT
- [ ] Integrate Kokoro TTS for voice synthesis
- [ ] Test latency (target <500ms)
- [ ] Create demo: speak → transcribe → synthesize → play

**Deliverable:** Standalone CLI app that echoes your voice

---

### Milestone 2: LLM Integration (2-3 weeks)

**Goal:** Add context-aware NPC responses

**Tasks:**
- [ ] Download Llama-3.2-1B-Instruct GGUF
- [ ] Integrate `llama-cpp-rs`
- [ ] Design prompt template with MEMZ context
- [ ] Test response quality with sample NPC personalities
- [ ] Optimize inference speed (target <200ms)

**Deliverable:** CLI app with NPC personality responses

---

### Milestone 3: Veloren Integration (3-4 weeks)

**Goal:** Voice dialogue working in-game

**Tasks:**
- [ ] Add `VoiceDialogueSystem` to `memz-veloren`
- [ ] Hook into Veloren's input system (PTT keybind)
- [ ] Connect to MEMZ memory retrieval
- [ ] Integrate with Veloren's audio system (spatial audio)
- [ ] Display transcribed text in dialogue box
- [ ] Test with multiple NPCs in singleplayer

**Deliverable:** In-game voice dialogue with NPCs

---

### Milestone 4: Polish & Edge Cases (2-3 weeks)

**Goal:** Production-ready system

**Tasks:**
- [ ] Implement semantic interruption handling
- [ ] Add multi-NPC selection (raycast)
- [ ] Optimize memory usage (<1GB total)
- [ ] Add settings UI (enable/disable voice, PTT key, etc.)
- [ ] Test on Mac M4 (your system)
- [ ] Test on Windows/Linux (cross-platform)
- [ ] Add toxicity filtering
- [ ] Performance profiling (ensure 60 FPS)

**Deliverable:** Polished voice dialogue system

---

### Milestone 5: Fine-Tuning & Datasets (Optional, 4-6 weeks)

**Goal:** Gaming-specific model improvements

**Tasks:**
- [ ] Curate Twitch chat dataset
- [ ] Curate esports voice comms dataset
- [ ] Fine-tune Whisper on gaming audio (LoRA)
- [ ] Fine-tune Llama on gaming dialogue (LoRA)
- [ ] Evaluate improvements (WER, response quality)

**Deliverable:** Gaming-optimized models

---

## 6. Technical Challenges & Solutions

### Challenge 1: Latency Budget

**Problem:** 500ms feels sluggish, need <300ms for natural conversation

**Solutions:**
- Use Whisper-tiny (fastest variant)
- Use Llama-1B (not 3B or 7B)
- Use 4-bit quantization (GGUF Q4_K_M)
- Run inference on GPU if available (Metal on M4)
- Stream TTS output (start playing before full generation)

**Latency Breakdown:**
- STT: 50-100ms (Whisper-tiny)
- MEMZ context: 20-50ms (database query)
- LLM: 100-150ms (Llama-1B, 20 tokens @ 150 tok/s)
- TTS: 50-100ms (Kokoro)
- **Total: 220-400ms** ✅

---

### Challenge 2: Memory Usage

**Problem:** Loading all models at once may exceed RAM

**Solutions:**
- Lazy load models (only when first NPC interaction)
- Unload models after 5 minutes of inactivity
- Use model quantization (4-bit GGUF)
- Share LLM context across NPCs (don't load per-NPC)

**Memory Breakdown:**
- Whisper-tiny: ~200MB RAM
- Llama-1B Q4: ~800MB RAM
- Kokoro TTS: ~150MB RAM
- **Total: ~1.2GB RAM** (acceptable on modern systems)

---

### Challenge 3: Cross-Platform Audio

**Problem:** Audio I/O differs on macOS, Windows, Linux

**Solutions:**
- Use `cpal` (cross-platform audio library)
- Test on all platforms
- Fallback to text dialogue if audio fails

---

### Challenge 4: Model Distribution

**Problem:** How to distribute 850MB of models with the game?

**Solutions:**
- **Option A:** Bundle models in game download (increases size by 850MB)
- **Option B:** Download models on first launch (like Airshipper does for assets)
- **Option C:** Make voice dialogue an optional DLC/mod

**Recommendation:** Option B (download on demand)

---

## 7. User Experience Design

### 7.1 Voice Interaction Flow

**Step 1: Approach NPC**
- Player walks near NPC
- UI indicator: "Press V to talk" (if voice enabled)
- UI indicator: "Press E for text dialogue" (fallback)

**Step 2: Start Voice Input**
- Player holds V key
- Microphone icon appears on screen
- Audio waveform visualizer (optional)
- Status: "Listening..."

**Step 3: Speak**
- Player speaks into microphone
- VAD detects speech
- Audio buffer fills

**Step 4: Release & Process**
- Player releases V key
- Status: "Transcribing..."
- Whisper converts speech to text
- Transcribed text appears in dialogue box (for confirmation)

**Step 5: NPC Thinks**
- Status: "Thinking..."
- MEMZ retrieves context
- LLM generates response
- Progress indicator (spinning icon)

**Step 6: NPC Responds**
- Status: "Speaking..."
- TTS synthesizes audio
- Audio plays from NPC's position (spatial 3D)
- Response text appears in dialogue box
- NPC's mouth animates (if possible)

**Step 7: Continue or End**
- Player can press V again to continue
- Or press ESC to end dialogue
- Or walk away (auto-end after 5 seconds)

---

### 7.2 Settings & Configuration

**New Settings Menu: "Voice Dialogue"**

```
[ ] Enable Voice Dialogue (default: OFF)

Voice Input:
  - Microphone Device: [Dropdown]
  - Push-to-Talk Key: [V] (rebindable)
  - Input Volume: [Slider]
  - Noise Suppression: [Low/Medium/High]

Voice Output:
  - TTS Voice: [Male 1 / Female 1 / ...] (per NPC type)
  - Speech Speed: [Slider] (0.8x - 1.5x)
  - Voice Volume: [Slider]

Advanced:
  - Show Transcription: [✓] (display STT text)
  - Allow Interruptions: [✓] (full-duplex)
  - Mute Game Audio During Recording: [ ]
  - Model Quality: [Fast / Balanced / Quality]
```

---

### 7.3 Accessibility

**Fallbacks:**
- Text dialogue always available (press E)
- Subtitles for NPC voice (existing system)
- Visual indicators for voice state (listening, thinking, speaking)
- Keyboard-only mode (no voice required)

---

## 8. Performance Targets

| Metric | Target | Acceptable | Unacceptable |
|--------|--------|------------|--------------|
| **Latency** | <300ms | <500ms | >500ms |
| **FPS Impact** | <5% | <10% | >10% |
| **RAM Usage** | <1GB | <1.5GB | >2GB |
| **Disk Space** | <850MB | <1.2GB | >2GB |
| **CPU Usage** | <20% | <30% | >50% |

---

## 9. Testing Plan

### 9.1 Unit Tests

- [ ] Microphone capture works on all platforms
- [ ] Whisper transcribes common phrases accurately
- [ ] LLM generates in-character responses
- [ ] TTS produces clear, natural audio
- [ ] Spatial audio positioning is correct

### 9.2 Integration Tests

- [ ] Voice dialogue works with MEMZ memory system
- [ ] NPC remembers previous voice conversations
- [ ] Sentiment changes based on voice interactions
- [ ] Multiple NPCs can be talked to sequentially

### 9.3 Performance Tests

- [ ] Latency <500ms on Mac M4
- [ ] FPS stays above 55 during voice dialogue
- [ ] Memory usage <1.5GB
- [ ] No memory leaks after 100 conversations

### 9.4 User Acceptance Tests

- [ ] Players can understand NPC voices
- [ ] NPCs understand player speech (various accents)
- [ ] Interruptions feel natural
- [ ] Voice dialogue is more immersive than text

---

## 10. Future Enhancements

### 10.1 Emotion-Aware TTS

**Goal:** NPC voice reflects emotional state

**Implementation:**
- Use MEMZ emotional memory (PAD model)
- Map PAD values to TTS prosody parameters:
  - Pleasure → pitch (happy = higher pitch)
  - Arousal → speed (excited = faster)
  - Dominance → volume (confident = louder)

---

### 10.2 Voice Cloning

**Goal:** Each NPC has unique voice

**Implementation:**
- Use voice cloning TTS (e.g., Coqui XTTS)
- Generate voice profiles from text descriptions
- Cache voice embeddings per NPC

---

### 10.3 Multiplayer Voice Chat

**Goal:** Players talk to each other via voice

**Implementation:**
- Use WebRTC for P2P voice
- Apply spatial audio (hear nearby players)
- Optional voice effects (underwater, echo, etc.)

---

### 10.4 Voice Commands

**Goal:** Control game with voice

**Examples:**
- "Open inventory"
- "Follow me" (to NPC)
- "Attack that enemy"

**Implementation:**
- Add intent classification layer
- Map intents to game actions

---

## 11. Resources & References

### 11.1 Models

**STT:**
- [Whisper.cpp](https://github.com/ggerganov/whisper.cpp) - Fast C++ implementation
- [whisper-rs](https://github.com/tazz4843/whisper-rs) - Rust bindings
- [Moonshine](https://github.com/usefulsensors/moonshine) - Lightweight alternative

**LLM:**
- [Llama-3.2-1B-Instruct](https://huggingface.co/meta-llama/Llama-3.2-1B-Instruct) - Base model
- [llama.cpp](https://github.com/ggerganov/llama.cpp) - Fast inference
- [llama-cpp-rs](https://github.com/utilityai/llama-cpp-rs) - Rust bindings

**TTS:**
- [Kokoro TTS](https://huggingface.co/hexgrad/Kokoro-82M) - Lightweight, high-quality
- [Piper TTS](https://github.com/rhasspy/piper) - Fast neural TTS

**VAD:**
- [Silero VAD](https://github.com/snakers4/silero-vad) - Voice activity detection

### 11.2 Libraries

**Audio:**
- [cpal](https://github.com/RustAudio/cpal) - Cross-platform audio I/O
- [rodio](https://github.com/RustAudio/rodio) - Audio playback (already in Veloren)
- [hound](https://github.com/ruuda/hound) - WAV file handling

**Inference:**
- [candle](https://github.com/huggingface/candle) - Rust ML framework
- [burn](https://github.com/tracel-ai/burn) - Rust deep learning

### 11.3 Research Papers

- [Whisper: Robust Speech Recognition](https://arxiv.org/abs/2212.04356)
- [LLaMA: Open and Efficient Foundation Language Models](https://arxiv.org/abs/2302.13971)
- [StyleTTS 2: Towards Human-Level Text-to-Speech](https://arxiv.org/abs/2306.07691)

### 11.4 Inspiration Projects

- [LLaMA-Omni](https://github.com/ictnlp/LLaMA-Omni) - End-to-end speech LLM
- [RealtimeVoiceChat](https://github.com/KoljaB/RealtimeSTT) - Low-latency voice chat
- [Convai](https://convai.com/) - Conversational AI for games (commercial)

---

## 12. Conclusion

This implementation plan provides a comprehensive roadmap for adding voice-based NPC dialogue to Veloren, inspired by cutting-edge AI voice infrastructure research. The system will be:

✅ **Local-first** - No cloud dependencies, works offline  
✅ **Low-latency** - <500ms response time, feels real-time  
✅ **Lightweight** - <1GB footprint, acceptable for game distribution  
✅ **Context-aware** - Powered by MEMZ memory system  
✅ **Immersive** - Spatial 3D audio, natural conversations  
✅ **Accessible** - Text fallback, subtitles, visual indicators  

**Next Steps:**
1. Review this plan with the team
2. Set up development environment (Rust, models)
3. Start Milestone 1 (Proof of Concept)
4. Iterate based on testing feedback

**Estimated Timeline:** 10-15 weeks for full implementation

---

**Document Version:** 1.0  
**Last Updated:** March 22, 2026  
**Author:** Kiro AI Assistant  
**Status:** Ready for Implementation
