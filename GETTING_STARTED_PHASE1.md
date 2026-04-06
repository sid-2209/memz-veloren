# 🚀 Getting Started - Phase 1: Voice-Only NPC Dialogue

**Goal:** Get basic voice dialogue working (no MEMZ integration yet)

---

## What You'll Build

A system where you can:
1. Hold V key and speak to an NPC
2. NPC generates a response using LLM
3. NPC responds with synthesized voice
4. Text appears in dialogue box

**Timeline:** 2-3 weeks  
**Complexity:** Medium

---

## Step 1: Download Models (~850MB)

```bash
cd /Users/siddhartha/Downloads/dev/Project\ Vyuh/memz

# Create models directory
mkdir -p models
cd models

# Download Whisper-tiny (~50MB)
curl -L -o whisper-tiny.en.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin

# Download Llama-3.2-1B (~700MB, may take 5-10 minutes)
curl -L -o llama-3.2-1b-q4.gguf \
  https://huggingface.co/bartowski/Llama-3.2-1B-Instruct-GGUF/resolve/main/Llama-3.2-1B-Instruct-Q4_K_M.gguf

cd ..
```

**Verify downloads:**
```bash
ls -lh models/
# Should show:
# whisper-tiny.en.bin (~50MB)
# llama-3.2-1b-q4.gguf (~700MB)
```

---

## Step 2: Build memz-voice Crate

```bash
cd memz-voice
cargo build --release
```

This will download dependencies and compile. First build may take 5-10 minutes.

---

## Step 3: Test Components

### Test 3.1: Speech-to-Text

```bash
cargo run --example test_stt
```

**What it does:**
- Loads Whisper model
- Records from your microphone for 5 seconds
- Transcribes your speech to text

**Expected output:**
```
Loading Whisper model...
Model loaded!

Press Enter to start recording...
🎤 Recording... (will record for 5 seconds)
⏳ Transcribing...

✅ Transcription:
  "Hello, how are you today?"
```

**Troubleshooting:**
- If "No input device available": Check microphone permissions in System Settings
- If transcription is empty: Speak louder or closer to mic
- If model not found: Re-run Step 1

---

### Test 3.2: Text-to-Speech

```bash
cargo run --example test_tts
```

**What it does:**
- Uses macOS system TTS (temporary solution)
- Converts text to speech
- Plays audio

**Expected output:**
```
Enter text to synthesize (or 'quit' to exit): Hello there!
🎵 Synthesizing...
✅ Generated 48000 audio samples
🔊 Playing audio...
Done!
```

---

### Test 3.3: LLM Dialogue

```bash
cargo run --example test_llm
```

**What it does:**
- Loads Llama-3.2-1B model
- Generates NPC responses to your text input

**Expected output:**
```
Loading LLM model (this may take a moment)...
Model loaded!

NPC: blacksmith (friendly)
Type your message to the NPC (or 'quit' to exit)

You: Do you have any weapons for sale?
🤔 Thinking...
NPC: Aye, I've got some fine blades fresh from the forge! Take a look at what I have in stock.

You: How much for a sword?
🤔 Thinking...
NPC: A good steel sword will run you about 50 gold pieces. Worth every coin, I assure you!
```

**Note:** First response may take 10-20 seconds as model loads into memory. Subsequent responses should be faster (2-5 seconds).

---

### Test 3.4: Full Voice Dialogue

```bash
cargo run --example test_full
```

**What it does:**
- Complete voice loop
- You speak → NPC responds with voice

**Expected output:**
```
Loading models (this may take a moment)...
All models loaded!

You approach a guard NPC (grumpy)
Press Enter to talk (hold for 5 seconds), or type 'quit' to exit

Press Enter to start talking: [Enter]
🎤 Recording... (speak now, will record for 5 seconds)
⏳ Processing...

✅ NPC responds:
  "What do you want? Make it quick, I'm on duty."

🔊 Playing NPC voice...
Done!
```

---

## Step 4: Performance Check

After running `test_full`, check:

- **Latency:** Should feel <1 second from speaking to hearing response
- **Quality:** Transcription should be accurate (>90%)
- **Audio:** NPC voice should be clear and audible

**If latency is too high:**
- Close other applications
- Ensure Metal GPU is being used (check logs for "Using Metal")
- Try reducing max_tokens in llm.rs

---

## Step 5: Next Steps

Once all tests pass, you're ready for Veloren integration!

### Option A: Continue with Veloren Integration
Follow `VOICE_PHASE1_PLAN.md` Milestone 1.3 to integrate with Veloren.

### Option B: Improve Voice Quality
- Integrate Kokoro TTS (better quality than system TTS)
- Add voice activity detection (VAD)
- Implement streaming audio

### Option C: Test with Different NPCs
Modify `test_full.rs` to try different NPC types:
```rust
let npc_profession = "merchant"; // or "guard", "farmer", "wizard"
let npc_mood = "cheerful"; // or "grumpy", "nervous", "confident"
```

---

## Common Issues

### Issue: "Model not found"
**Solution:** Re-run Step 1 to download models

### Issue: "No input device available"
**Solution:** 
1. Check microphone is connected
2. macOS: System Settings → Privacy & Security → Microphone → Allow Terminal

### Issue: "Transcription is empty"
**Solution:**
- Speak louder and clearer
- Check microphone is not muted
- Try different microphone

### Issue: "LLM too slow"
**Solution:**
- First inference is always slow (model loading)
- Subsequent inferences should be faster
- Close other applications to free RAM
- Ensure Metal GPU is enabled

### Issue: "Audio playback fails"
**Solution:**
- Check speakers/headphones are connected
- Increase system volume
- Try `afplay /tmp/memz_playback.wav` manually

---

## File Structure

```
memz-voice/
├── src/
│   ├── lib.rs           # Main API
│   ├── stt.rs           # Speech-to-Text (Whisper)
│   ├── tts.rs           # Text-to-Speech (macOS/Kokoro)
│   ├── llm.rs           # LLM Dialogue (Llama)
│   └── error.rs         # Error types
├── examples/
│   ├── test_stt.rs      # Test microphone → text
│   ├── test_tts.rs      # Test text → audio
│   ├── test_llm.rs      # Test LLM responses
│   └── test_full.rs     # Full voice dialogue loop
├── Cargo.toml
└── README.md
```

---

## What's Next?

After Phase 1 is working:

**Phase 2:** Add MEMZ integration
- NPC remembers past conversations
- Personality-driven responses (OCEAN traits)
- Sentiment tracking
- Context-aware greetings
- Gossip and social memory

**Phase 3:** Veloren integration
- Hook into game's input system
- Connect to spatial audio
- Display in dialogue box
- Test with 1,976 NPCs

---

## Success Criteria

✅ All 4 test examples run successfully  
✅ Transcription accuracy >90%  
✅ NPC responses make sense  
✅ Audio plays clearly  
✅ Total latency <1 second  
✅ No crashes or errors  

---

## Need Help?

- Check `memz-voice/README.md` for API documentation
- Review `VOICE_PHASE1_PLAN.md` for detailed roadmap
- Check logs with `RUST_LOG=debug cargo run --example test_full`

---

**Ready to start!** Run Step 1 to download models, then test each component.
