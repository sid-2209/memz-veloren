#!/bin/bash

echo "╔════════════════════════════════════════════════════════════╗"
echo "║         Voice NPC Test - Setup & Run                      ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

# Check if Ollama is running
echo "1️⃣  Checking Ollama..."
if pgrep -x "ollama" > /dev/null; then
    echo "   ✅ Ollama is running"
else
    echo "   ⚠️  Ollama is NOT running"
    echo ""
    echo "   Starting Ollama in background..."
    ollama serve > /dev/null 2>&1 &
    sleep 2
    echo "   ✅ Ollama started"
fi

echo ""
echo "2️⃣  Checking Ollama model..."
if ollama list | grep -q "llama3.2:1b"; then
    echo "   ✅ llama3.2:1b model available"
else
    echo "   ⚠️  Model not found, pulling..."
    ollama pull llama3.2:1b
fi

echo ""
echo "3️⃣  Microphone Tips:"
echo "   • Speak LOUDLY and clearly"
echo "   • Speak directly into AirPods"
echo "   • Increase microphone volume if needed:"
echo "     System Settings → Sound → Input → Volume slider"

echo ""
echo "4️⃣  Starting voice test..."
echo ""
echo "═══════════════════════════════════════════════════════════"
echo ""

cd memz-veloren
cargo run --example test_voice_ingame --release
