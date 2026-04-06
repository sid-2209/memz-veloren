# ✅ Voice NPC Testing Checklist

Follow these steps in order to test voice conversations with NPCs.

---

## Prerequisites (Already Done! ✅)

- [x] Ollama installed
- [x] llama3.2:1b model pulled
- [x] memz-voice crate created
- [x] All code implemented
- [x] LLM tested and working

---

## Step-by-Step Testing

### 1. Download Whisper Model ⏳

**Time:** 2 minutes  
**Size:** ~75MB

```bash
# Open a NEW terminal window
cd "/Users/siddhartha/Downloads/dev/Project Vyuh/memz"

# Download
mkdir -p models
cd models
curl -L -o whisper-tiny.en.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin

# Verify
ls -lh whisper-tiny.en.bin
```

**Expected output:**
```
-rw-r--r--  1 user  staff   75M  whisper-tiny.en.bin
```

- [ ] Whisper model downloaded

---

### 2. Test Text-to-Speech (TTS) 🔊

**Time:** 1 minute

```bash
cd memz-voice
cargo run --example test_tts --release
```

**What to do:**
1. Wait for "Enter text to synthesize:" prompt
2. Type: `Hello adventurer, welcome to my shop!`
3. Press Enter
4. Listen to the voice output
5. Type `quit` to exit

**Expected:** You hear a voice speaking your text

- [ ] TTS working (can hear voice output)

---

### 3. Test LLM Dialogue 💬

**Time:** 1 minute

```bash
cargo run --example test_llm --release
```

**What to do:**
1. Wait for "You:" prompt
2. Type: `What weapons do you have?`
3. Press Enter
4. Read the NPC response
5. Type `quit` to exit

**Expected:** NPC responds in character as a blacksmith

- [ ] LLM working (generates contextual responses)

---

### 4. Test Speech-to-Text (STT) 🎤

**Time:** 1 minute  
**Requires:** Microphone

```bash
cargo run --example test_stt --release
```

**What to do:**
1. Press Enter when prompted
2. Speak clearly: "Hello, can you help me?"
3. Wait 5 seconds
4. Read the transcription

**Expected:** Your speech is transcribed to text

**Troubleshooting:**
- If no transcription: Check microphone permissions
  - System Settings → Privacy & Security → Microphone
  - Enable for Terminal

- [ ] STT working (transcribes speech correctly)

---

### 5. Test Full Voice Pipeline 🎉

**Time:** 2-3 minutes  
**Requires:** Microphone + Speakers

```bash
cargo run --example test_full --release
```

**What to do:**
1. Wait for "Press Enter to start talking:" prompt
2. Press Enter
3. Speak clearly for 5 seconds: "Hello, who are you?"
4. Wait for processing (~3-4 seconds)
5. Listen to the NPC response
6. Repeat with different questions
7. Type `quit` to exit

**Test Conversation:**
```
You speak: "Hello, who are you?"
NPC responds: "I'm a guard, what do you want?"

You speak: "What do you do here?"
NPC responds: "I keep watch over this area..."

You speak: "Can you help me?"
NPC responds: "Depends on what you need..."
```

**Expected Flow:**
1. 🎤 You speak → System captures audio
2. 📝 STT transcribes → Shows "Player said: ..."
3. 🤖 LLM generates → Shows "NPC responds: ..."
4. 🔊 TTS speaks → You hear the response

- [ ] Full pipeline working (complete voice conversation)

---

## Success Criteria

You've successfully completed the setup when:

✅ You can speak into your microphone  
✅ The system understands what you said  
✅ The NPC generates a contextual response  
✅ You hear the NPC speak back to you  
✅ The conversation feels natural and responsive  

---

## Performance Check

| Metric | Target | Your Result |
|--------|--------|-------------|
| STT Latency | < 1s | _____ |
| LLM Response | 2-3s | _____ |
| TTS Latency | < 500ms | _____ |
| Total Round-trip | 3-4s | _____ |
| Transcription Accuracy | > 90% | _____ |

---

## Common Issues & Fixes

### Issue: "Microphone not working"
**Fix:**
```bash
# Check permissions
# System Settings → Privacy & Security → Microphone
# Enable for Terminal

# Test microphone
say "Testing microphone"
```

### Issue: "No audio output"
**Fix:**
```bash
# Test speakers
say "Testing speakers"

# Check volume
# System Settings → Sound → Output
```

### Issue: "Whisper model not found"
**Fix:**
```bash
# Check if downloaded
ls -lh models/whisper-tiny.en.bin

# If missing, download
cd models
curl -L -o whisper-tiny.en.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin
```

### Issue: "Ollama not responding"
**Fix:**
```bash
# Check if running
ollama list

# Restart if needed
killall ollama
ollama serve &
ollama pull llama3.2:1b
```

### Issue: "Build errors"
**Fix:**
```bash
cd memz-voice
cargo clean
cargo build --release
```

### Issue: "Transcription is gibberish"
**Possible causes:**
- Speaking too quietly
- Too much background noise
- Microphone too far away

**Fix:**
- Speak clearly and loudly
- Move closer to microphone
- Reduce background noise

---

## Next Steps After Testing

Once all tests pass:

### Immediate (Today)
- [ ] Test with different NPC personalities
- [ ] Try longer conversations
- [ ] Test with background noise

### Short-term (This Week)
- [ ] Integrate with Veloren NPCs
- [ ] Add keybind for voice input
- [ ] Connect to MEMZ memory system

### Medium-term (Next Week)
- [ ] Test in-game with multiple NPCs
- [ ] Optimize latency
- [ ] Add voice activity detection

---

## Test Results Log

**Date:** ___________

**Test 1 - TTS:**
- Status: ⬜ Pass ⬜ Fail
- Notes: _________________________________

**Test 2 - LLM:**
- Status: ⬜ Pass ⬜ Fail
- Notes: _________________________________

**Test 3 - STT:**
- Status: ⬜ Pass ⬜ Fail
- Notes: _________________________________

**Test 4 - Full Pipeline:**
- Status: ⬜ Pass ⬜ Fail
- Notes: _________________________________

**Overall Status:** ⬜ Ready for Veloren Integration

---

## 🎉 Completion

When all checkboxes are marked:

**Congratulations!** You have a working voice-based NPC dialogue system!

You can now:
- Have real-time voice conversations with NPCs
- Generate contextual, in-character responses
- Hear NPCs speak back to you

**Next:** Integrate with Veloren for in-game voice chat!

See `VELOREN_INTEGRATION_GUIDE.md` (coming next) for integration steps.

---

**Start here:** `bash download_whisper.sh`
