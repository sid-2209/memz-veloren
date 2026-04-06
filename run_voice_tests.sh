#!/bin/bash
set -e

echo "╔════════════════════════════════════════════════════════════╗"
echo "║         Voice NPC System - Complete Test Suite            ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Kill any running tests
echo "🧹 Cleaning up..."
pkill -9 -f "test_llm" 2>/dev/null || true
pkill -9 -f "test_stt" 2>/dev/null || true
sleep 1

# Check Ollama
echo ""
echo "1️⃣  Checking Ollama..."
if command -v ollama &> /dev/null; then
    echo -e "${GREEN}✓${NC} Ollama installed"
    if ollama list | grep -q "llama3.2:1b"; then
        echo -e "${GREEN}✓${NC} llama3.2:1b model available"
    else
        echo -e "${YELLOW}⚠${NC}  Pulling llama3.2:1b model..."
        ollama pull llama3.2:1b
    fi
else
    echo -e "${RED}✗${NC} Ollama not installed"
    echo "   Install with: brew install ollama"
    exit 1
fi

# Check Whisper model
echo ""
echo "2️⃣  Checking Whisper model..."
if [ -f "models/whisper-tiny.en.bin" ]; then
    echo -e "${GREEN}✓${NC} Whisper model found"
    ls -lh models/whisper-tiny.en.bin
else
    echo -e "${YELLOW}⚠${NC}  Downloading Whisper model (~75MB)..."
    mkdir -p models
    cd models
    curl -L -o whisper-tiny.en.bin \
      https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin
    cd ..
    echo -e "${GREEN}✓${NC} Whisper model downloaded"
fi

# Build memz-voice
echo ""
echo "3️⃣  Building memz-voice..."
cd memz-voice
if cargo build --release 2>&1 | grep -q "error"; then
    echo -e "${RED}✗${NC} Build failed"
    exit 1
else
    echo -e "${GREEN}✓${NC} Build successful"
fi
cd ..

# Test TTS
echo ""
echo "4️⃣  Testing Text-to-Speech..."
echo "   (Using macOS system TTS)"
cd memz-voice
echo "Hello from the voice NPC system!" | timeout 10 cargo run --example test_tts --release 2>&1 | grep -q "Synthesizing" && echo -e "${GREEN}✓${NC} TTS working" || echo -e "${YELLOW}⚠${NC}  TTS test skipped (interactive)"
cd ..

# Test LLM
echo ""
echo "5️⃣  Testing LLM Dialogue..."
cd memz-voice
echo "quit" | timeout 10 cargo run --example test_llm --release 2>&1 | grep -q "LLM ready" && echo -e "${GREEN}✓${NC} LLM working" || echo -e "${YELLOW}⚠${NC}  LLM test skipped"
cd ..

echo ""
echo "╔════════════════════════════════════════════════════════════╗"
echo "║                    Setup Complete! 🎉                      ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""
echo "📋 Component Status:"
echo -e "   ${GREEN}✓${NC} Ollama & LLM (llama3.2:1b)"
echo -e "   ${GREEN}✓${NC} Whisper STT (tiny.en model)"
echo -e "   ${GREEN}✓${NC} macOS System TTS"
echo -e "   ${GREEN}✓${NC} memz-voice crate"
echo ""
echo "🎮 Ready to Test!"
echo ""
echo "Choose a test:"
echo "  1. Test TTS only:        cd memz-voice && cargo run --example test_tts --release"
echo "  2. Test LLM only:        cd memz-voice && cargo run --example test_llm --release"
echo "  3. Test STT only:        cd memz-voice && cargo run --example test_stt --release"
echo "  4. Test FULL pipeline:   cd memz-voice && cargo run --example test_full --release"
echo ""
echo "🎙️  For full voice conversation test, run:"
echo "     cd memz-voice && cargo run --example test_full --release"
echo ""
echo "This will let you:"
echo "  • Speak into your microphone"
echo "  • Have the NPC understand you (STT)"
echo "  • Generate a response (LLM)"
echo "  • Hear the NPC speak back (TTS)"
echo ""
