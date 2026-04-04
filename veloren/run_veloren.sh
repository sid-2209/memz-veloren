#!/bin/bash
# Run Veloren from the correct directory so it can find assets

set -e

cd "$(dirname "$0")"
export VELOREN_ASSETS="$(pwd)/assets"
export VELOREN_USERDATA="$(pwd)/userdata"

PROJECT_ROOT="$(cd .. && pwd)"
BLITZ_PORT="${BLITZ_TTS_PORT:-8890}"
BLITZ_HEALTH_URL="http://127.0.0.1:${BLITZ_PORT}/health"
OLLAMA_HEALTH_URL="http://127.0.0.1:11434/api/tags"

start_blitz_tts() {
  if curl -sf "$BLITZ_HEALTH_URL" >/dev/null 2>&1; then
    echo "Blitz TTS server already running."
    return
  fi

  local wheel_path
  wheel_path="$(ls "$PROJECT_ROOT"/blitz_tts-*.whl 2>/dev/null | head -n 1 || true)"
  if [ -z "$wheel_path" ] || [ ! -f "$PROJECT_ROOT/blitz_tts_server.py" ]; then
    echo "Blitz TTS wheel or server script not found; skipping Blitz startup."
    return
  fi

  echo "Starting Blitz TTS server..."
  BLITZ_TTS_WHEEL="$wheel_path" \
  BLITZ_TTS_PORT="$BLITZ_PORT" \
    nohup python3 "$PROJECT_ROOT/blitz_tts_server.py" >/tmp/blitz_tts_server.log 2>&1 &

  for _ in $(seq 1 45); do
    if curl -sf "$BLITZ_HEALTH_URL" >/dev/null 2>&1; then
      echo "Blitz TTS server ready."
      return
    fi
    sleep 1
  done

  echo "Blitz TTS server did not become ready in time. Check /tmp/blitz_tts_server.log."
}

start_ollama() {
  if curl -sf "$OLLAMA_HEALTH_URL" >/dev/null 2>&1; then
    echo "Ollama already running."
    return
  fi

  if ! command -v ollama >/dev/null 2>&1; then
    echo "Ollama command not found; skipping Ollama startup."
    return
  fi

  echo "Starting Ollama..."
  nohup ollama serve >/tmp/ollama_veloren.log 2>&1 &

  for _ in $(seq 1 20); do
    if curl -sf "$OLLAMA_HEALTH_URL" >/dev/null 2>&1; then
      echo "Ollama ready."
      return
    fi
    sleep 1
  done

  echo "Ollama did not become ready in time. Check /tmp/ollama_veloren.log."
}

start_blitz_tts
start_ollama

./target/release/veloren-voxygen "$@"
