# 🎙️ Voice NPC Commands Cheatsheet

Quick reference for working with the voice NPC system.

## Setup Commands

```bash
# Install Ollama
brew install ollama

# Pull LLM model
ollama pull llama3.2:1b

# Verify model
ollama list

# Download Whisper model
mkdir -p models && cd models
curl -L -o whisper-tiny.en.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin
```

## Test Commands

```bash
# Navigate to voice crate
cd memz-voice

# Test LLM (interactive)
cargo run --example test_llm

# Test LLM (automated)
cargo run --example test_llm_auto

# Test Speech-to-Text
cargo run --example test_stt

# Test Text-to-Speech
cargo run --example test_tts

# Test full pipeline
cargo run --example test_full

# Build everything
cargo build --release
```

## Development Commands

```bash
# Watch for changes and rebuild
cargo watch -x 'build -p memz-voice'

# Run with logging
RUST_LOG=debug cargo run --example test_llm

# Check for errors
cargo check -p memz-voice

# Run tests
cargo test -p memz-voice

# Format code
cargo fmt -p memz-voice

# Lint code
cargo clippy -p memz-voice
```

## Ollama Commands

```bash
# List available models
ollama list

# Pull a different model
ollama pull llama3.2:3b

# Remove a model
ollama rm llama3.2:1b

# Show model info
ollama show llama3.2:1b

# Test model directly
ollama run llama3.2:1b "You are a blacksmith. Greet the player."

# Check Ollama status
ps aux | grep ollama
```

## File Locations

```
memz/
├── models/
│   ├── whisper-tiny.en.bin          # STT model (~75MB)
│   └── llama-3.2-1b-q4.gguf         # Not needed (using Ollama)
├── memz-voice/
│   ├── src/
│   │   ├── llm.rs                   # LLM implementation
│   │   ├── stt.rs                   # Speech-to-text
│   │   ├── tts.rs                   # Text-to-speech
│   │   └── lib.rs                   # Main module
│   └── examples/
│       ├── test_llm.rs              # Interactive LLM test
│       ├── test_llm_auto.rs         # Automated LLM test
│       ├── test_stt.rs              # STT test
│       ├── test_tts.rs              # TTS test
│       └── test_full.rs             # Full pipeline test
└── memz-veloren/
    └── src/
        └── voice_system.rs          # Veloren integration (TODO)
```

## Common Issues & Fixes

### Ollama not responding
```bash
# Restart Ollama
killall ollama
ollama serve &
```

### Model not found
```bash
# Re-pull the model
ollama pull llama3.2:1b
```

### Compilation errors
```bash
# Clean and rebuild
cargo clean
cargo build -p memz-voice
```

### Microphone not working
```bash
# Check permissions (macOS)
# System Settings → Privacy & Security → Microphone
# Enable for Terminal/IDE

# Test microphone
rec test.wav
```

### Slow responses
```bash
# Use smaller model
ollama pull llama3.2:1b  # Already the smallest

# Check CPU/GPU usage
top -o cpu

# Reduce context length in code
# Edit memz-voice/src/llm.rs
```

## Quick Test Script

Save as `test_voice.sh`:
```bash
#!/bin/bash
set -e

echo "🧪 Testing Voice NPC System..."

# Check Ollama
echo "1. Checking Ollama..."
ollama list | grep llama3.2:1b || (echo "❌ Model not found" && exit 1)
echo "✅ Ollama ready"

# Check Whisper model
echo "2. Checking Whisper model..."
[ -f models/whisper-tiny.en.bin ] || (echo "❌ Whisper model not found" && exit 1)
echo "✅ Whisper model ready"

# Build
echo "3. Building memz-voice..."
cd memz-voice
cargo build --quiet 2>&1 | grep -q error && (echo "❌ Build failed" && exit 1)
echo "✅ Build successful"

# Test LLM
echo "4. Testing LLM..."
echo "quit" | cargo run --example test_llm --quiet 2>&1 | grep -q "LLM ready" || (echo "❌ LLM test failed" && exit 1)
echo "✅ LLM working"

echo ""
echo "🎉 All systems ready!"
```

Run with:
```bash
chmod +x test_voice.sh
./test_voice.sh
```

## Integration Checklist

- [x] Ollama installed
- [x] llama3.2:1b model pulled
- [x] memz-voice crate created
- [x] LLM implementation working
- [ ] Whisper model downloaded
- [ ] STT tested
- [ ] TTS tested
- [ ] Full pipeline tested
- [ ] Integrated with Veloren
- [ ] Connected to MEMZ memory
- [ ] In-game testing complete

## Performance Targets

| Metric | Target | Current |
|--------|--------|---------|
| LLM Response Time | < 3s | ~2-3s ✅ |
| STT Latency | < 1s | TBD |
| TTS Latency | < 500ms | TBD |
| Total Round-trip | < 5s | TBD |
| Memory Usage | < 3GB | ~2GB ✅ |

## Next Steps

1. ⏳ Download Whisper model
2. ⏳ Test STT component
3. ⏳ Test TTS component
4. ⏳ Test full pipeline
5. ⏳ Integrate with Veloren
6. ⏳ Add MEMZ context
7. ⏳ In-game testing

---

**Need help?** Check `VOICE_QUICK_START.md` or `SETUP_COMPLETE.md`
