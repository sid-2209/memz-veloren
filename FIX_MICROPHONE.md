# 🎤 Quick Fix: Microphone Not Working

## The Problem

Your voice system is working perfectly, but the microphone isn't capturing audio!

**Evidence:**
```
Transcription: [BLANK_AUDIO]
Using input device: Steam Streaming Microphone
```

## The Solution (2 Minutes)

### Step 1: Change Default Microphone

1. Open **System Settings** (or run: `open "x-apple.systempreferences:com.apple.preference.sound"`)
2. Click **Sound**
3. Click **Input** tab
4. Select your **actual microphone**:
   - "MacBook Pro Microphone" (built-in)
   - "External Microphone"
   - Your USB mic name
   - **NOT** "Steam Streaming Microphone"
5. Speak and watch the input level meter move

### Step 2: Test Microphone

```bash
cd memz-voice
cargo run --example test_microphone --release
```

This will:
- Show all available microphones
- Test recording for 3 seconds
- Analyze audio levels
- Tell you if it's working

**Expected output:**
```
✅ Audio level is GOOD!
   Your microphone is working correctly!
```

### Step 3: Test Voice Conversation Again

```bash
cargo run --example test_full --release
```

Now when you speak, you should see:
```
Transcription: "Hello, can you help me?"  ← Real words!
NPC responds: "Help you? What do you need?" ← Contextual response!
```

## Why This Happened

- **Steam Streaming Microphone** is a virtual audio device
- It's used for streaming/broadcasting, not capturing your voice
- macOS selected it as default (probably recently)
- The code uses the default device, which was the wrong one

## Quick Test Commands

```bash
# 1. List all microphones
cd memz-voice
cargo run --example list_audio_devices

# 2. Test microphone capture
cargo run --example test_microphone --release

# 3. Test full voice pipeline
cargo run --example test_full --release
```

## Troubleshooting

### "No audio captured"
- Check microphone permissions:
  - System Settings → Privacy & Security → Microphone
  - Enable for Terminal (or your IDE)

### "Audio appears silent"
- Increase microphone volume:
  - System Settings → Sound → Input → Input volume slider
- Speak louder
- Move closer to microphone

### "Wrong device selected"
- Change default in System Settings → Sound → Input
- Or we can modify code to select specific device

## After Fix: Expected Behavior

### Before (Current)
```
You: [speak] "Hello, can you help me?"
System: Transcription: [BLANK_AUDIO]
NPC: "Can't you see I'm busy?" (generic)
```

### After (Fixed)
```
You: [speak] "Hello, can you help me?"
System: Transcription: "Hello, can you help me?"
NPC: "Help you? What do you need, traveler?" (contextual!)
```

## Test Conversation Examples

Once fixed, try these:

1. "Hello, who are you?"
   → NPC: "I'm a guard. What do you want?"

2. "What do you do here?"
   → NPC: "I keep watch over this area."

3. "Can you help me find the blacksmith?"
   → NPC: "The blacksmith? His forge is down the street."

4. "Thank you!"
   → NPC: "Just doing my job. Move along."

Much better! 🎉

## Quick Fix Checklist

- [ ] Open System Settings → Sound → Input
- [ ] Select actual microphone (not Steam)
- [ ] Verify input level moves when speaking
- [ ] Run: `cargo run --example test_microphone --release`
- [ ] See "Audio level is GOOD!"
- [ ] Run: `cargo run --example test_full --release`
- [ ] Speak and see real transcription
- [ ] Hear contextual NPC response

## Need More Help?

See `MICROPHONE_ISSUE_ANALYSIS.md` for detailed analysis and advanced solutions.

---

**TL;DR:** 
1. System Settings → Sound → Input
2. Select your real microphone (not Steam)
3. Run `cargo run --example test_full --release`
4. Enjoy real conversations! 🎙️
