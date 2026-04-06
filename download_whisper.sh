#!/bin/bash
# Simple script to download Whisper model

echo "Downloading Whisper tiny.en model (~75MB)..."
echo ""

mkdir -p models
cd models

if [ -f "whisper-tiny.en.bin" ]; then
    echo "✓ Model already exists!"
    ls -lh whisper-tiny.en.bin
else
    echo "Downloading from Hugging Face..."
    curl -L --progress-bar -o whisper-tiny.en.bin \
      https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin
    
    if [ -f "whisper-tiny.en.bin" ]; then
        echo ""
        echo "✓ Download complete!"
        ls -lh whisper-tiny.en.bin
    else
        echo "✗ Download failed"
        exit 1
    fi
fi

cd ..
echo ""
echo "Ready to test STT!"
echo "Run: cd memz-voice && cargo run --example test_stt --release"
