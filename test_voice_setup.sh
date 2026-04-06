#!/bin/bash

echo "=== Voice NPC Setup Verification ==="
echo ""

# Check Ollama
echo "1. Checking Ollama..."
if command -v ollama &> /dev/null; then
    echo "   ✓ Ollama installed"
    if ollama list | grep -q "llama3.2:1b"; then
        echo "   ✓ llama3.2:1b model available"
    else
        echo "   ✗ llama3.2:1b model not found"
        echo "   Run: ollama pull llama3.2:1b"
    fi
else
    echo "   ✗ Ollama not installed"
    echo "   Run: brew install ollama"
fi
echo ""

# Check Whisper model
echo "2. Checking Whisper model..."
if [ -f "models/whisper-tiny.en.bin" ]; then
    echo "   ✓ Whisper model found"
    ls -lh models/whisper-tiny.en.bin
else
    echo "   ✗ Whisper model not found at models/whisper-tiny.en.bin"
    echo "   Download with:"
    echo "   mkdir -p models"
    echo "   cd models"
    echo "   curl -L -o whisper-tiny.en.bin \\"
    echo "     https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin"
fi
echo ""

# Check if memz-voice builds
echo "3. Checking memz-voice build..."
cd memz-voice
if cargo build --quiet 2>&1 | grep -q "error"; then
    echo "   ✗ Build failed"
else
    echo "   ✓ memz-voice builds successfully"
fi
cd ..
echo ""

echo "=== Setup Status ==="
echo "Ready to test voice components!"
echo ""
echo "Next steps:"
echo "  1. Test LLM: cd memz-voice && echo 'quit' | cargo run --example test_llm"
echo "  2. Test STT: cd memz-voice && cargo run --example test_stt"
echo "  3. Test TTS: cd memz-voice && cargo run --example test_tts"
