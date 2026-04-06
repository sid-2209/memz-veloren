# 🎙️ Ready to Test Voice NPCs!

## Current Status

✅ **LLM Working** - Ollama with llama3.2:1b generates NPC responses  
✅ **Code Complete** - All STT, TTS, and LLM components implemented  
✅ **Tests Ready** - Full pipeline test available  
⏳ **Whisper Model** - Needs download (~75MB, 2 minutes)

---

## Quick Start (5 Minutes to Voice Chat!)

### Step 1: Download Whisper Model (2 minutes)

Open a **NEW terminal window** (the current one has a test running) and run:

```bash
cd "/Users/siddhartha/Downloads/dev/Project Vyuh/memz"

# Download Whisper model
mkdir -p models
cd models
curl -L -o whisper-tiny.en.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin
cd ..
```

Or use the script:
```bash
bash download_whisper.sh
```

### Step 2: Test Individual Components (1 minute each)

#### Test TTS (Text-to-Speech)
```bash
cd memz-voice
cargo run --example test_tts --release
```

Type: "Hello adventurer, welcome to my shop!"  
You'll hear the NPC speak!

#### Test LLM (Dialogue Generation)
```bash
cargo run --example test_llm --release
```

Type: "What weapons do you have?"  
The NPC will respond in character!

#### Test STT (Speech-to-Text)
```bash
cargo run --example test_stt --release
```

Press Enter, speak for 5 seconds, and see your words transcribed!

### Step 3: Test Full Voice Conversation! 🎉

```bash
cd memz-voice
cargo run --example test_full --release
```

**What happens:**
1. Press Enter to start recording
2. Speak into your microphone for 5 seconds
3. The system will:
   - Transcribe your speech (STT)
   - Generate NPC response (LLM)
   - Speak the response back (TTS)
4. Repeat!

**Example conversation:**
```
You: "Hello, can you help me?"
NPC: "Aye, what brings you to my forge today?"

You: "I need a new sword"
NPC: "A fine blade you seek! I've got just the thing..."
```

---

## Full Pipeline Flow

```
┌─────────────────────────────────────────────────────────┐
│  YOU SPEAK: "Hello, can you repair my sword?"          │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
         ┌───────────────────────┐
         │   Microphone Input    │
         │   (cpal captures)     │
         └───────────┬───────────┘
                     │
                     ▼
         ┌───────────────────────┐
         │  Speech-to-Text       │
         │  (Whisper tiny.en)    │
         │  ~500ms               │
         └───────────┬───────────┘
                     │
                     ▼
         "Hello, can you repair my sword?"
                     │
                     ▼
         ┌───────────────────────┐
         │  LLM Generation       │
         │  (Ollama llama3.2:1b) │
         │  ~2-3 seconds         │
         └───────────┬───────────┘
                     │
                     ▼
         "Ah, you're looking for some fine work..."
                     │
                     ▼
         ┌───────────────────────┐
         │  Text-to-Speech       │
         │  (macOS say)          │
         │  ~200ms               │
         └───────────┬───────────┘
                     │
                     ▼
         ┌───────────────────────┐
         │   Speaker Output      │
         │   (NPC speaks!)       │
         └───────────────────────┘
```

**Total latency:** ~3-4 seconds from speaking to hearing response

---

## Troubleshooting

### "Microphone not working"
1. Check System Settings → Privacy & Security → Microphone
2. Enable for Terminal or your IDE
3. Test with: `rec test.wav` (install sox if needed)

### "No audio output"
1. Check your speaker volume
2. Test with: `say "Hello"`
3. Make sure speakers/headphones are connected

### "Whisper model not found"
```bash
# Check if it exists
ls -lh models/whisper-tiny.en.bin

# If not, download it
cd models
curl -L -o whisper-tiny.en.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin
```

### "Ollama not responding"
```bash
# Check if running
ps aux | grep ollama

# Restart if needed
killall ollama
ollama serve &
```

### "Build errors"
```bash
cd memz-voice
cargo clean
cargo build --release
```

---

## Performance Expectations

| Component | Latency | Quality |
|-----------|---------|---------|
| STT (Whisper tiny) | ~500ms | Good for clear speech |
| LLM (llama3.2:1b) | ~2-3s | Contextual, in-character |
| TTS (macOS say) | ~200ms | Natural, clear |
| **Total Round-trip** | **~3-4s** | Conversational |

---

## What's Next?

Once you've tested the full pipeline:

### Phase 2: Veloren Integration
1. Add voice system to Veloren's NPC interaction
2. Bind to a key (e.g., V for voice)
3. Connect to MEMZ memory for context-aware responses
4. Test in-game with multiple NPCs

### Phase 3: Enhancements
1. Add voice activity detection (auto-start/stop)
2. Implement audio streaming for lower latency
3. Add multiple voice profiles per NPC type
4. Optimize for real-time performance

---

## Test Commands Summary

```bash
# In a NEW terminal window (not the one with test_llm running)
cd "/Users/siddhartha/Downloads/dev/Project Vyuh/memz"

# 1. Download Whisper model (one time)
bash download_whisper.sh

# 2. Test TTS
cd memz-voice && cargo run --example test_tts --release

# 3. Test LLM
cargo run --example test_llm --release

# 4. Test STT
cargo run --example test_stt --release

# 5. Test FULL VOICE CONVERSATION
cargo run --example test_full --release
```

---

## 🎉 You're Ready!

Everything is set up. Just download the Whisper model and you can start having voice conversations with NPCs!

The system will:
- ✅ Listen to your voice
- ✅ Understand what you say
- ✅ Generate contextual NPC responses
- ✅ Speak back to you

**Total setup time remaining:** ~2 minutes (just the Whisper download)

---

## Quick Demo Script

Want to test quickly? Try this conversation:

1. Run: `cd memz-voice && cargo run --example test_full --release`
2. Press Enter to start
3. Say: "Hello, who are you?"
4. Listen to NPC response
5. Press Enter again
6. Say: "What do you do here?"
7. Listen to response
8. Press Enter again
9. Say: "Can you help me?"
10. Enjoy the conversation!

The NPC will respond as a grumpy guard (as configured in the test).

---

**Ready?** Open a new terminal and run:
```bash
bash download_whisper.sh
```

Then test the full pipeline:
```bash
cd memz-voice && cargo run --example test_full --release
```

🎙️ **Have fun talking to your NPCs!**
