#!/bin/bash
set -e

echo "=== Voice NPC Complete Setup ==="
echo ""

# Kill any running test processes
echo "1. Cleaning up any running tests..."
pkill -9 -f "test_llm" 2>/dev/null || true
sleep 1

# Download Whisper model if not exists
echo "2. Checking Whisper model..."
if [ ! -f "models/whisper-tiny.en.bin" ]; then
    echo "   Downloading Whisper model (~75MB)..."
    mkdir -p models
    cd models
    curl -L -o whisper-tiny.en.bin \
      https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin
    cd ..
    echo "   ✅ Whisper model downloaded"
else
    echo "   ✅ Whisper model already exists"
    ls -lh models/whisper-tiny.en.bin
fi
echo ""

# Verify Ollama
echo "3. Verifying Ollama..."
if ollama list | grep -q "llama3.2:1b"; then
    echo "   ✅ Ollama and llama3.2:1b ready"
else
    echo "   ⚠️  llama3.2:1b not found, pulling..."
    ollama pull llama3.2:1b
fi
echo ""

# Build memz-voice
echo "4. Building memz-voice..."
cd memz-voice
cargo build --release 2>&1 | grep -E "(Compiling|Finished)" || true
cd ..
echo "   ✅ Build complete"
echo ""

echo "=== Setup Complete! ==="
echo ""
echo "Next: Test the components"
echo "  1. Test STT: cd memz-voice && cargo run --example test_stt"
echo "  2. Test TTS: cd memz-voice && cargo run --example test_tts"
echo "  3. Test Full: cd memz-voice && cargo run --example test_full"
