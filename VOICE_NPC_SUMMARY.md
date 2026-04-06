# 🎙️ Voice-Based NPC Dialogue: Executive Summary

**Date:** March 22, 2026  
**Project:** Veloren + MEMZ Voice Integration  
**Status:** Design Complete, Ready for Implementation

---

## What We're Building

A real-time voice conversation system that lets players talk to Veloren's 1,976 NPCs using their microphone, with NPCs responding in synthesized voice. The system is:

- **Local-first:** All AI runs on your Mac M4, no cloud/internet required
- **Low-latency:** <500ms response time (feels instant)
- **Lightweight:** <1GB total footprint (acceptable for games)
- **Context-aware:** Powered by MEMZ memory system (NPCs remember you)
- **Immersive:** Spatial 3D audio (voice comes from NPC's position)

---

## How It Works

```
You speak → Microphone → Whisper (STT) → MEMZ Context → Llama (LLM) → Kokoro (TTS) → Speakers
                                                ↓
                                        Dialogue Box (text fallback)
```

**Example Interaction:**

1. You approach a guard NPC
2. Hold **V** key and say: "Have you seen any bandits nearby?"
3. System transcribes your speech (50-100ms)
4. MEMZ retrieves guard's memories and personality (20-50ms)
5. LLM generates response: "Aye, saw some suspicious folk near the old mill this morning. Best be careful." (100-150ms)
6. TTS synthesizes guard's voice (50-100ms)
7. Audio plays from guard's position in 3D space
8. Text appears in dialogue box

**Total time:** ~300ms (feels instant!)

---

## Technology Stack

| Component | Technology | Size | Purpose |
|-----------|-----------|------|---------|
| **Voice Input** | Whisper-tiny.en | ~50MB | Speech-to-text |
| **Voice Output** | Kokoro TTS (82M) | ~100MB | Text-to-speech |
| **NPC Brain** | Llama-3.2-1B | ~700MB | Generate responses |
| **Memory** | MEMZ (existing) | N/A | NPC context/personality |
| **Audio** | Veloren rodio (existing) | N/A | 3D spatial audio |

**Total:** ~850MB (one-time download)

---

## Key Features

### ✅ What's Included

1. **Push-to-Talk (PTT):** Hold V to speak, release to send
2. **Real-time Transcription:** See what you said (confirmation)
3. **Context-Aware Responses:** NPCs remember past interactions
4. **Personality-Driven Dialogue:** Each NPC has unique voice/personality
5. **Spatial Audio:** Voice comes from NPC's 3D position
6. **Text Fallback:** Traditional E-to-interact still works
7. **Interruption Handling:** Can interrupt NPC mid-sentence
8. **Multi-NPC Support:** Talk to any of 1,976 NPCs

### 🚀 Future Enhancements

- Emotion-aware TTS (happy/sad/angry voices)
- Voice cloning (unique voice per NPC)
- Multiplayer voice chat
- Voice commands ("Open inventory", "Follow me")

---

## Inspiration Sources

### Project 1: AI Voice Infrastructure for Games

**Key Insights:**
- Use ultra-lightweight models (<1GB) for game distribution
- Run everything locally (no cloud latency)
- Fine-tune on gaming datasets (Twitch, esports)
- Implement semantic interruption handling (full-duplex)
- Integrate with game's spatial audio system

### Project 2: Model Comparison (BUD-E vs. Lightweight)

**Key Decision:**
- **Rejected:** BUD-E Whisper (4.5GB, 400-800ms latency)
- **Chosen:** Micro-cascaded pipeline (850MB, 250-350ms latency)
- **Reason:** Better for game distribution, faster, mobile-ready

---

## Implementation Roadmap

### Phase 1: Foundation (2-3 weeks)
- Set up `memz-voice` crate
- Implement microphone capture
- Integrate Whisper STT
- Integrate Kokoro TTS
- Test latency (<500ms)

### Phase 2: LLM Integration (2-3 weeks)
- Load Llama-3.2-1B model
- Design prompt templates
- Connect to MEMZ memory system
- Test response quality

### Phase 3: Veloren Integration (3-4 weeks)
- Add voice system to game loop
- Hook PTT keybind (V key)
- Connect to spatial audio
- Display text in dialogue box
- Test with multiple NPCs

### Phase 4: Polish (2-3 weeks)
- Implement interruption handling
- Add settings UI
- Optimize performance
- Cross-platform testing
- Toxicity filtering

**Total Timeline:** 10-15 weeks

---

## Performance Targets

| Metric | Target |
|--------|--------|
| **Latency** | <300ms (acceptable: <500ms) |
| **FPS Impact** | <5% (acceptable: <10%) |
| **RAM Usage** | <1GB (acceptable: <1.5GB) |
| **Disk Space** | <850MB (acceptable: <1.2GB) |

---

## Technical Challenges & Solutions

### Challenge 1: Latency
**Solution:** Use fastest models (Whisper-tiny, Llama-1B), enable Metal GPU on M4

### Challenge 2: Memory Usage
**Solution:** Lazy load models, use 4-bit quantization, unload after inactivity

### Challenge 3: Cross-Platform Audio
**Solution:** Use `cpal` library (works on macOS, Windows, Linux)

### Challenge 4: Model Distribution
**Solution:** Download models on first launch (like Airshipper does for assets)

---

## User Experience

### Voice Interaction Flow

1. **Approach NPC** → UI shows "Press V to talk"
2. **Hold V** → Microphone icon appears, "Listening..."
3. **Speak** → Audio waveform visualizer
4. **Release V** → "Transcribing..." → Shows your text
5. **Wait** → "Thinking..." → Progress spinner
6. **NPC Responds** → "Speaking..." → Audio plays + text appears
7. **Continue or End** → Press V again or walk away

### Settings

```
Voice Dialogue Settings:
  [✓] Enable Voice Dialogue
  
  Microphone: [Default Device ▼]
  Push-to-Talk Key: [V] (rebindable)
  Input Volume: [▓▓▓▓▓▓▓▓░░] 80%
  
  TTS Voice: [Male 1 ▼]
  Speech Speed: [▓▓▓▓▓░░░░░] 1.0x
  Voice Volume: [▓▓▓▓▓▓▓▓░░] 80%
  
  [✓] Show Transcription
  [✓] Allow Interruptions
  [ ] Mute Game Audio During Recording
```

---

## Files Created

1. **VOICE_NPC_IMPLEMENTATION_PLAN.md** (15,000 words)
   - Comprehensive technical design
   - Architecture diagrams
   - API specifications
   - Testing plan
   - Future enhancements

2. **VOICE_QUICK_START.md** (3,000 words)
   - Step-by-step setup guide
   - Code examples
   - Troubleshooting tips
   - Quick reference

3. **VOICE_NPC_SUMMARY.md** (this file)
   - Executive overview
   - Key decisions
   - Timeline
   - Performance targets

---

## Next Steps

### For You (Developer)

1. **Review Documents:**
   - Read `VOICE_NPC_IMPLEMENTATION_PLAN.md` for full details
   - Read `VOICE_QUICK_START.md` for hands-on guide

2. **Download Models:**
   ```bash
   mkdir models
   cd models
   # Download Whisper, Llama, Kokoro (see Quick Start)
   ```

3. **Start Phase 1:**
   ```bash
   cargo new --lib memz-voice
   # Follow Quick Start guide
   ```

4. **Test Standalone:**
   - Build voice I/O system
   - Test latency on your Mac M4
   - Verify <500ms response time

5. **Integrate with Veloren:**
   - Add to game loop
   - Test with NPCs
   - Iterate based on feedback

### For Testing

- **Milestone 1:** Standalone voice echo (speak → transcribe → synthesize → play)
- **Milestone 2:** NPC personality responses (context-aware dialogue)
- **Milestone 3:** In-game voice dialogue (full integration)
- **Milestone 4:** Polish and optimization

---

## Resources

### Models to Download

- [Whisper-tiny.en](https://huggingface.co/ggerganov/whisper.cpp) (~50MB)
- [Llama-3.2-1B-Instruct](https://huggingface.co/meta-llama/Llama-3.2-1B-Instruct) (~700MB)
- [Kokoro TTS](https://huggingface.co/hexgrad/Kokoro-82M) (~100MB)

### Libraries

- [whisper-rs](https://github.com/tazz4843/whisper-rs) - Rust Whisper bindings
- [llama-cpp-rs](https://github.com/utilityai/llama-cpp-rs) - Rust Llama bindings
- [cpal](https://github.com/RustAudio/cpal) - Cross-platform audio I/O

### Research

- [Whisper Paper](https://arxiv.org/abs/2212.04356)
- [LLaMA Paper](https://arxiv.org/abs/2302.13971)
- [StyleTTS 2 Paper](https://arxiv.org/abs/2306.07691)

---

## Conclusion

This project will transform Veloren's NPC interactions from static text to dynamic voice conversations. By leveraging cutting-edge lightweight AI models and your existing MEMZ memory system, players will experience truly immersive, context-aware dialogue with 1,976 unique NPCs.

**The system is designed to be:**
- ✅ Fast enough for real-time gameplay (<500ms)
- ✅ Small enough for game distribution (<1GB)
- ✅ Smart enough for natural conversations (MEMZ + LLM)
- ✅ Immersive enough to feel alive (spatial audio)

**Ready to start building!** 🎙️🎮

---

**Questions?** Review the full implementation plan or quick start guide for details.

**Estimated Effort:** 10-15 weeks for full implementation  
**Complexity:** Medium-High (requires ML model integration)  
**Impact:** High (revolutionary NPC interaction)
