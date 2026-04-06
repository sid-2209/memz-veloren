#!/bin/bash
# Run Veloren from the correct directory so it can find assets

set -e

cd "$(dirname "$0")"
export VELOREN_ASSETS="$(pwd)/assets"
export VELOREN_USERDATA="$(pwd)/userdata"

ensure_voice_rust_log() {
  local required_entries="memz_voice=info,memz_voice::stt=info,memz_veloren=info"
  if [ -z "${RUST_LOG:-}" ]; then
    export RUST_LOG="warn,${required_entries}"
    return
  fi

  case ",${RUST_LOG}," in
    *,memz_voice=info,*|*,memz_voice::stt=info,*|*,memz_veloren=info,*)
      ;;
    *)
      export RUST_LOG="${RUST_LOG},${required_entries}"
      ;;
  esac
}

ensure_voice_rust_log

PROJECT_ROOT="$(cd .. && pwd)"
BINARY_PATH="./target/release/veloren-voxygen"
RUST_TOOLCHAIN="${VELOREN_RUSTUP_TOOLCHAIN:-nightly-2025-09-08-aarch64-apple-darwin}"
STT_PORT="${MEMZ_STT_PORT:-8891}"
STT_HEALTH_URL="http://127.0.0.1:${STT_PORT}/health"
BLITZ_PORT="${BLITZ_TTS_PORT:-8890}"
BLITZ_HEALTH_URL="http://127.0.0.1:${BLITZ_PORT}/health"
OLLAMA_HEALTH_URL="http://127.0.0.1:11434/api/tags"

find_python_with_module() {
  local module_name="$1"
  local candidate=""
  local candidates=()

  if [ -n "${MEMZ_PYTHON_BIN:-}" ]; then
    candidates+=("$MEMZ_PYTHON_BIN")
  fi
  candidates+=(
    "$(command -v python3 2>/dev/null || true)"
    "/opt/anaconda3/bin/python3"
    "/opt/homebrew/bin/python3"
    "/opt/homebrew/bin/python3.11"
    "/usr/bin/python3"
  )

  for candidate in "${candidates[@]}"; do
    if [ -n "$candidate" ] && [ -x "$candidate" ] && "$candidate" -c "import ${module_name}" >/dev/null 2>&1; then
      echo "$candidate"
      return 0
    fi
  done

  return 1
}

detect_macos_default_input_device() {
  if [ "$(uname -s)" != "Darwin" ] || ! command -v system_profiler >/dev/null 2>&1; then
    return
  fi

  system_profiler SPAudioDataType 2>/dev/null | awk '
    function finish_block() {
      if (device == "" || default_input != "Yes") {
        return
      }

      if (fallback == "") {
        fallback = device
      }

      if (transport != "Virtual" && best == "") {
        best = device
      }
    }

    /^        [^ ].*:$/ {
      finish_block()
      device = $0
      sub(/^        /, "", device)
      sub(/:$/, "", device)
      default_input = ""
      transport = ""
      next
    }

    /Default Input Device:/ {
      default_input = $NF
      next
    }

    /Transport:/ {
      transport = $NF
      next
    }

    END {
      finish_block()
      if (best != "") {
        print best
      } else if (fallback != "") {
        print fallback
      }
    }
  '
}

configure_voice_input_device() {
  if [ -n "${MEMZ_VOICE_INPUT_DEVICE:-}" ]; then
    echo "Using MEMZ_VOICE_INPUT_DEVICE=$MEMZ_VOICE_INPUT_DEVICE"
    return
  fi

  local detected_device
  detected_device="$(detect_macos_default_input_device)"
  if [ -n "$detected_device" ]; then
    export MEMZ_VOICE_INPUT_DEVICE="$detected_device"
    echo "Using macOS default input device for voice capture: $MEMZ_VOICE_INPUT_DEVICE"
  fi
}

find_newer_source_file() {
  local watch_paths=(
    "$PROJECT_ROOT/memz-voice/Cargo.toml"
    "$PROJECT_ROOT/memz-voice/src"
    "$PROJECT_ROOT/memz-veloren/Cargo.toml"
    "$PROJECT_ROOT/memz-veloren/src"
    "./Cargo.toml"
    "./Cargo.lock"
    "./voxygen/Cargo.toml"
    "./voxygen/src"
  )

  find "${watch_paths[@]}" -type f -newer "$BINARY_PATH" -print -quit 2>/dev/null || true
}

ensure_release_binary() {
  local newer_file=""

  if [ -x "$BINARY_PATH" ]; then
    newer_file="$(find_newer_source_file)"
    if [ -z "$newer_file" ]; then
      return
    fi
  fi

  if [ "${VELOREN_AUTO_BUILD:-1}" = "0" ]; then
    if [ -n "$newer_file" ]; then
      echo "Release binary is stale but VELOREN_AUTO_BUILD=0; newer source: $newer_file"
    else
      echo "Release binary is missing but VELOREN_AUTO_BUILD=0."
    fi
    return
  fi

  if [ -n "$newer_file" ]; then
    echo "Release binary is older than $newer_file"
  else
    echo "Release binary not found."
  fi
  echo "Building veloren-voxygen release binary..."
  CARGO_WORKSPACE_DIR="$(pwd)" cargo +"$RUST_TOOLCHAIN" build \
    --manifest-path Cargo.toml \
    -p veloren-voxygen \
    --bin veloren-voxygen \
    --release
}

start_blitz_tts() {
  local python_bin=""

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

  if ! python_bin="$(find_python_with_module blitz_tts)"; then
    echo "No Python interpreter with blitz_tts found; skipping Blitz startup."
    return
  fi

  echo "Starting Blitz TTS server..."
  BLITZ_TTS_WHEEL="$wheel_path" \
  BLITZ_TTS_PORT="$BLITZ_PORT" \
    nohup "$python_bin" -u "$PROJECT_ROOT/blitz_tts_server.py" >/tmp/blitz_tts_server.log 2>&1 &

  for _ in $(seq 1 45); do
    if curl -sf "$BLITZ_HEALTH_URL" >/dev/null 2>&1; then
      echo "Blitz TTS server ready."
      return
    fi
    sleep 1
  done

  echo "Blitz TTS server did not become ready in time. Check /tmp/blitz_tts_server.log."
}

start_mlx_stt() {
  local python_bin=""

  if curl -sf "$STT_HEALTH_URL" >/dev/null 2>&1; then
    echo "MLX STT server already running."
    return
  fi

  if [ ! -f "$PROJECT_ROOT/mlx_stt_server.py" ]; then
    echo "MLX STT server script not found; skipping STT server startup."
    return
  fi

  if ! python_bin="$(find_python_with_module mlx_whisper)"; then
    echo "No Python interpreter with mlx_whisper found; skipping HTTP STT startup."
    return
  fi

  echo "Starting MLX Whisper STT server..."
  local stt_pid=""
  MEMZ_STT_PORT="$STT_PORT" \
  MEMZ_STT_MODEL="${MEMZ_STT_MODEL:-}" \
    nohup "$python_bin" -u "$PROJECT_ROOT/mlx_stt_server.py" >/tmp/mlx_stt_server.log 2>&1 &
  stt_pid=$!

  for _ in $(seq 1 120); do
    if curl -sf "$STT_HEALTH_URL" >/dev/null 2>&1; then
      echo "MLX STT server ready."
      return
    fi
    if ! kill -0 "$stt_pid" >/dev/null 2>&1; then
      echo "MLX STT server exited before becoming ready. Check /tmp/mlx_stt_server.log."
      if [ -f /tmp/mlx_stt_server.log ]; then
        tail -n 40 /tmp/mlx_stt_server.log || true
      fi
      return
    fi
    sleep 1
  done

  echo "MLX STT server did not become ready in time. Check /tmp/mlx_stt_server.log."
}

maybe_start_mlx_stt() {
  case "${MEMZ_STT_URL:-}" in
    "" )
      if [ "${MEMZ_ENABLE_HTTP_STT:-0}" = "1" ] || [ -n "${MEMZ_STT_MODEL:-}" ]; then
        export MEMZ_STT_URL="http://127.0.0.1:${STT_PORT}"
      else
        echo "Using bundled local Whisper STT (HTTP STT disabled by default)."
        return
      fi
      ;;
    "http://127.0.0.1:${STT_PORT}"|"http://localhost:${STT_PORT}" )
      ;;
    * )
      echo "Using external HTTP STT endpoint: ${MEMZ_STT_URL}"
      return
      ;;
  esac

  start_mlx_stt
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

ensure_release_binary
configure_voice_input_device
maybe_start_mlx_stt
start_blitz_tts
start_ollama

"$BINARY_PATH" "$@"
