# 🔧 Quick Fix - Two Issues to Resolve

## Issue 1: Ollama Not Running ❌

**Error:** `could not connect to ollama server, run 'ollama serve' to start it`

**Fix:**

Open a **new terminal window** and run:
```bash
ollama serve
```

Leave this running in the background.

## Issue 2: Audio Level Too Low ⚠️

**Problem:** Max amplitude: 0.043 (should be > 0.1)

**This is why you got [BLANK_AUDIO]** - you're not speaking loud enough or microphone volume is too low.

**Fix Option 1: Speak Louder**
- Speak directly into your AirPods
- Speak clearly and loudly
- Move AirPods closer to your mouth

**Fix Option 2: Increase Microphone Volume**
1. System Settings → Sound → Input
2. Select "Siddhartha's AirPods Pro"
3. Increase "Input volume" slider to maximum
4. Test by speaking - watch the input level meter

**Fix Option 3: Test Microphone Gain**
```bash
cd memz-voice
cargo run --example test_microphone --release
```

Speak LOUDLY and check if max amplitude > 0.1

---

## Complete Fix Steps

### Step 1: Start Ollama (New Terminal)
```bash
ollama serve
```

### Step 2: Increase Microphone Volume
1. System Settings → Sound → Input
2. Increase volume slider to maximum

### Step 3: Test Again
```bash
cd memz-veloren
cargo run --example test_voice_ingame --release
```

### Step 4: Speak LOUDLY
When recording starts:
- Speak directly into AirPods
- Use a loud, clear voice
- Say: "HELLO GUARD, CAN YOU HELP ME FIND THE BLACKSMITH?"

---

## Expected Good Output

```
[2026-03-23T09:49:32Z INFO] Audio level OK (max amplitude: 0.500)  ← Should be > 0.1
[2026-03-23T09:49:32Z INFO] Transcription: "Hello guard, can you help me find the blacksmith?"  ← Real words!
[2026-03-23T09:49:32Z INFO] Player said: "Hello guard, can you help me find the blacksmith?"
[2026-03-23T09:49:35Z INFO] Generated response: "The blacksmith? His forge is down the street..."
```

---

## Quick Test Commands

```bash
# Terminal 1: Start Ollama
ollama serve

# Terminal 2: Test microphone
cd memz-voice
cargo run --example test_microphone --release
# Speak LOUDLY - check max amplitude > 0.1

# Terminal 2: Run voice test
cd memz-veloren
cargo run --example test_voice_ingame --release
# Select NPC, speak LOUDLY
```

---

## Why This Happened

1. **Ollama:** You tested earlier with `ollama list` which doesn't require the server. But `ollama run` needs `ollama serve` running.

2. **Audio Level:** AirPods microphone is working, but either:
   - You spoke too quietly
   - Microphone volume is set too low
   - AirPods are too far from your mouth

---

## Try Again!

1. **Start Ollama** in new terminal: `ollama serve`
2. **Increase mic volume** in System Settings
3. **Run test:** `cargo run --example test_voice_ingame --release`
4. **Speak LOUDLY** when recording starts!

You're so close! 🎉
