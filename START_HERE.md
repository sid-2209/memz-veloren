# 🚀 START HERE - Voice NPC Quick Start

## You're Almost There!

Everything is set up and ready. You're just **ONE STEP** away from having voice conversations with NPCs!

---

## What's Already Done ✅

- ✅ Ollama installed with llama3.2:1b model
- ✅ All voice components implemented (STT, TTS, LLM)
- ✅ Test examples created and ready
- ✅ LLM tested and generating responses
- ✅ Documentation complete

---

## What You Need to Do (2 Minutes)

### Step 1: Download Whisper Model

**Open a NEW terminal window** (not the one with test_llm running) and run:

```bash
cd "/Users/siddhartha/Downloads/dev/Project Vyuh/memz"

bash download_whisper.sh
```

This downloads the speech-to-text model (~75MB, takes 2 minutes).

### Step 2: Test Voice Conversation

```bash
cd memz-voice
cargo run --example test_full --release
```

**Then:**
1. Press Enter when prompted
2. Speak into your microphone: "Hello, who are you?"
3. Wait 3-4 seconds
4. Listen to the NPC respond!

---

## What Happens

```
YOU SPEAK → Microphone captures → Whisper transcribes → 
Ollama generates response → macOS speaks → YOU HEAR NPC
```

**Total time:** ~3-4 seconds from speaking to hearing response

---

## Quick Test Conversation

Try these phrases:

1. "Hello, who are you?"
2. "What do you do here?"
3. "Can you help me?"
4. "What weapons do you have?"
5. "Tell me about this place"

The NPC will respond in character as a grumpy guard!

---

## Troubleshooting

### Microphone Not Working?
1. System Settings → Privacy & Security → Microphone
2. Enable for Terminal
3. Test with: `say "Testing"`

### No Audio Output?
1. Check speaker volume
2. Test with: `say "Hello"`

### Whisper Model Not Found?
```bash
cd models
curl -L -o whisper-tiny.en.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin
```

---

## Full Documentation

- **READY_TO_TEST.md** - Detailed testing guide
- **VOICE_TEST_CHECKLIST.md** - Step-by-step checklist
- **VOICE_COMMANDS_CHEATSHEET.md** - Quick commands
- **SETUP_COMPLETE.md** - What we've accomplished

---

## Next Steps After Testing

Once you've tested voice conversations:

1. **Integrate with Veloren** - Add voice to in-game NPCs
2. **Connect MEMZ** - Make NPCs remember conversations
3. **Add Keybinds** - Press V to talk to NPCs
4. **Test In-Game** - Have voice chats while playing

---

## 🎯 Your Goal Today

**Test the full voice pipeline and have a conversation with an NPC!**

**Time needed:** 5 minutes  
**Commands:**
```bash
# 1. Download model (2 min)
bash download_whisper.sh

# 2. Test voice chat (3 min)
cd memz-voice
cargo run --example test_full --release
```

---

## 🎉 Ready?

Open a new terminal and run:

```bash
cd "/Users/siddhartha/Downloads/dev/Project Vyuh/memz"
bash download_whisper.sh
```

Then test:

```bash
cd memz-voice
cargo run --example test_full --release
```

**Have fun talking to your NPCs!** 🎙️🎮
