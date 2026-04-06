#!/bin/bash

# Setup script for downloading voice models
# Run this before using memz-voice

set -e

echo "=== MEMZ Voice Models Setup ==="
echo ""

# Create models directory
mkdir -p models
cd models

echo "📦 Downloading models..."
echo ""

# Whisper-tiny (~50MB)
if [ ! -f "whisper-tiny.en.bin" ]; then
    echo "1/2 Downloading Whisper-tiny.en (~50MB)..."
    curl -L -o whisper-tiny.en.bin \
        https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin
    echo "✅ Whisper downloaded"
else
    echo "✅ Whisper already downloaded"
fi

echo ""

# Llama-3.2-1B (~700MB)
if [ ! -f "llama-3.2-1b-q4.gguf" ]; then
    echo "2/2 Downloading Llama-3.2-1B-Instruct (~700MB, this may take a while)..."
    curl -L -o llama-3.2-1b-q4.gguf \
        https://huggingface.co/bartowski/Llama-3.2-1B-Instruct-GGUF/resolve/main/Llama-3.2-1B-Instruct-Q4_K_M.gguf
    echo "✅ Llama downloaded"
else
    echo "✅ Llama already downloaded"
fi

echo ""
echo "✅ All models downloaded!"
echo ""
echo "Total size:"
du -sh .
echo ""
echo "Next steps:"
echo "  cd memz-voice"
echo "  cargo run --example test_full"
