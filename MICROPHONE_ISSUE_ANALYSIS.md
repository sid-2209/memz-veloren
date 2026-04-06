# 🎤 Microphone Issue Analysis & Fix

## Problem Identified

Your voice NPC system is working perfectly, but **the microphone isn't capturing audio**!

### Evidence
```
[2026-03-22T15:40:17Z INFO] Transcription: [BLANK_AUDIO]
[2026-03-22T15:40:17Z INFO] Player said: [BLANK_AUDIO]
```

### Root Cause
The system selected "Steam Streaming Microphone" as the default input device, which is a virtual device that doesn't capture real audio.

```
[2026-03-22T15:40:12Z INFO] Using input device: Steam Streaming Microphone
```

## Why NPCs Give Generic Responses

The LLM is receiving `[BLANK_AUDIO]` as input, so it generates responses based on:
- NPC personality (grumpy guard)
- No actual player input
- Random grumpy guard dialogue

**This is actually good!** It means the LLM is working correctly - it's just not getting real input.

## Solutions

### Solution 1: Change Default Microphone (Easiest)

1. Open **System Settings** → **Sound** → **Input**
2. Select your actual microphone (not Steam Streaming Microphone)
3. Test by speaking and watching the input level meter
4. Run the test again

### Solution 2: List Available Devices

Run this to see all available microphones:

```bash
cd memz-voice
cargo run --example list_audio_devices
```

This will show:
- All input devices
- Which one is currently default
- Sample rates and channels

### Solution 3: Modify Code to Select Specific Device

If you want to hardcode a specific microphone, we can modify `memz-voice/src/stt.rs` to:
1. List all devices
2. Let you choose by name or index
3. Use that device instead of default

### Solution 4: Add Device Selection to Test

We can add a command-line argument to specify which device to use:

```bash
cargo run --example test_full --release -- --device "MacBook Pro Microphone"
```

## Quick Fix Steps

### Step 1: Check Your Microphone Settings

```bash
# Open System Settings
open "x-apple.systempreferences:com.apple.preference.sound"
```

1. Go to **Input** tab
2. Look for your actual microphone:
   - "MacBook Pro Microphone" (built-in)
   - "External Microphone"
   - USB microphone name
3. Select it as default
4. Speak and verify the input level moves

### Step 2: Test Microphone

```bash
# Test with macOS say command
say "Testing microphone"

# Record a test
rec test.wav
# Speak for 3 seconds, then Ctrl+C
# Play it back
play test.wav
```

### Step 3: Run Voice Test Again

```bash
cd memz-voice
cargo run --example test_full --release
```

Now when you speak, you should see actual transcription instead of `[BLANK_AUDIO]`.

## Expected Behavior After Fix

### Before (Current)
```
You: [speak into mic]
Transcription: [BLANK_AUDIO]
NPC: "Can't you see I'm busy?" (generic response)
```

### After (Fixed)
```
You: "Hello, can you help me?"
Transcription: "Hello, can you help me?"
NPC: "Help you? What do you need help with, traveler?" (contextual response)
```

## Why This Happened

1. **Steam Streaming Microphone** is a virtual audio device created by Steam
2. It's used for streaming/broadcasting, not for capturing your voice
3. macOS selected it as default (probably because it was created recently)
4. The code uses `default_input_device()` which picked the wrong one

## Improved Code (Coming)

I'll update the STT code to:
1. Show which device is being used
2. Allow device selection
3. Warn if no audio is detected
4. Provide better error messages

## Testing Checklist

After changing your microphone:

- [ ] System Settings shows correct microphone selected
- [ ] Input level meter moves when you speak
- [ ] `say "test"` works (speakers)
- [ ] Run `test_full` again
- [ ] Speak clearly: "Hello, who are you?"
- [ ] See actual transcription (not [BLANK_AUDIO])
- [ ] NPC responds to what you actually said

## Advanced: Device Selection Code

If you want to manually select a device, here's the code pattern:

```rust
// List all devices
for (idx, device) in host.input_devices()?.enumerate() {
    println!("{}: {}", idx, device.name()?);
}

// Select by index
let devices: Vec<_> = host.input_devices()?.collect();
let device = &devices[1]; // Use device #1

// Or select by name
let device = host.input_devices()?
    .find(|d| d.name().unwrap_or_default().contains("MacBook"))
    .ok_or("Device not found")?;
```

## Next Steps

1. **Immediate:** Change default microphone in System Settings
2. **Test:** Run `test_full` again and speak clearly
3. **Verify:** Check transcription is correct
4. **Enjoy:** Have real conversations with NPCs!

## Additional Diagnostics

### Check Microphone Permissions

```bash
# Check if Terminal has microphone access
# System Settings → Privacy & Security → Microphone
# Ensure Terminal (or your IDE) is enabled
```

### Test Audio Capture

```bash
# Install sox if needed
brew install sox

# Record 5 seconds
rec -r 16000 -c 1 test.wav trim 0 5

# Play it back
play test.wav
```

If this works, your microphone is fine - just need to select it as default.

## Why LLM Responses Are Generic

The LLM prompt is:
```
You are a guard NPC in a fantasy RPG game. You are grumpy.
Respond in character with 1-2 sentences. Be concise and natural.

Player: [BLANK_AUDIO]
NPC:
```

Since there's no actual player input, the LLM generates:
- Generic grumpy guard responses
- "Can't you see I'm busy?"
- "Get lost, traveler"
- Random dismissive dialogue

**This is correct behavior!** The LLM is working as designed.

Once you fix the microphone, you'll see:
```
Player: "Can you help me find the blacksmith?"
NPC: "The blacksmith? His forge is down the street, past the tavern. Can't miss the smoke."
```

Much better! 🎉

---

**TL;DR:** Your microphone is set to "Steam Streaming Microphone" (virtual device). Change it to your actual microphone in System Settings → Sound → Input, then test again.
