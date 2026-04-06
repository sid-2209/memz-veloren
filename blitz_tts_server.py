#!/usr/bin/env python3
"""
Blitz TTS HTTP server for the Veloren voice demo.

This server loads the Blitz TTS wheel directly from the repo root, keeps the
model warm in memory, and exposes a tiny HTTP API:

GET  /health
GET  /voices
POST /synthesize
  {
    "text": "...",
    "voice": "M4",
    "lang": "en",
    "speed": 1.0,
    "steps": 7
  }

The response body is a WAV file resampled to 24 kHz mono PCM16 so the current
Rust voice playback path can consume it without further changes.
"""

from __future__ import annotations

import io
import json
import os
import sys
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path

import numpy as np
import soundfile as sf

HOST = os.environ.get("BLITZ_TTS_HOST", "127.0.0.1")
PORT = int(os.environ.get("BLITZ_TTS_PORT", "8890"))
TARGET_SAMPLE_RATE = int(os.environ.get("BLITZ_TTS_OUTPUT_SR", "24000"))
DEFAULT_LANG = os.environ.get("BLITZ_TTS_LANG", "en")
DEFAULT_VOICE = os.environ.get("BLITZ_TTS_DEFAULT_VOICE", "M4")
DEFAULT_STEPS = int(os.environ.get("BLITZ_TTS_STEPS", "7"))
DEFAULT_SPEED = float(os.environ.get("BLITZ_TTS_SPEED", "1.0"))
DEFAULT_SILENCE_DURATION = float(os.environ.get("BLITZ_TTS_SILENCE_DURATION", "0.02"))


def _repo_root() -> Path:
    return Path(__file__).resolve().parent


def _resolve_wheel_path() -> Path | None:
    explicit = os.environ.get("BLITZ_TTS_WHEEL")
    if explicit:
        path = Path(explicit).expanduser().resolve()
        if path.is_file():
            return path

    matches = sorted(_repo_root().glob("blitz_tts-*.whl"))
    return matches[-1] if matches else None


wheel_path = _resolve_wheel_path()
if wheel_path is not None:
    sys.path.insert(0, str(wheel_path))

try:
    from blitz_tts import TTS
except Exception as exc:  # pragma: no cover - depends on local environment
    print("Failed to import Blitz TTS SDK from wheel.", file=sys.stderr)
    print(f"Wheel path: {wheel_path}", file=sys.stderr)
    raise


def _providers() -> list[str] | None:
    value = os.environ.get("BLITZ_TTS_PROVIDERS", "").strip()
    if not value:
        return None
    return [item.strip() for item in value.split(",") if item.strip()]


def _resample(audio: np.ndarray, source_rate: int, target_rate: int) -> np.ndarray:
    if source_rate == target_rate:
        return np.asarray(audio, dtype=np.float32)

    if audio.size == 0:
        return np.asarray(audio, dtype=np.float32)

    target_len = max(1, int(round(audio.shape[0] * target_rate / float(source_rate))))
    source_positions = np.arange(audio.shape[0], dtype=np.float64)
    target_positions = np.linspace(0, audio.shape[0] - 1, num=target_len, dtype=np.float64)
    return np.interp(target_positions, source_positions, audio).astype(np.float32)


print("Loading Blitz TTS model...")
tts = TTS(
    auto_download=True,
    providers=_providers(),
    model_dir=os.environ.get("BLITZ_TTS_MODEL_DIR") or None,
    cache_dir=os.environ.get("BLITZ_TTS_CACHE_DIR") or None,
    bundle_url=os.environ.get("BLITZ_TTS_MODEL_BUNDLE_URL") or None,
    bundle_sha256=os.environ.get("BLITZ_TTS_MODEL_BUNDLE_SHA256") or None,
    bundle_sha256_url=os.environ.get("BLITZ_TTS_MODEL_BUNDLE_SHA256_URL") or None,
)
available_voices = sorted(tts.list_voices())
tts_lock = threading.Lock()

print(f"Blitz TTS ready at http://{HOST}:{PORT}")
print(f"Voices: {', '.join(available_voices)}")
print(f"Assets: {tts.assets_dir}")
print("Press Ctrl+C to stop.")


class TtsHandler(BaseHTTPRequestHandler):
    def do_GET(self) -> None:  # noqa: N802
        if self.path == "/health":
            body = json.dumps(
                {
                    "ok": True,
                    "sample_rate": TARGET_SAMPLE_RATE,
                    "voices": available_voices,
                }
            ).encode("utf-8")
            self._respond(200, body, "application/json")
            return

        if self.path == "/voices":
            body = json.dumps({"voices": available_voices}).encode("utf-8")
            self._respond(200, body, "application/json")
            return

        self._respond(404, b"Not Found", "text/plain")

    def do_POST(self) -> None:  # noqa: N802
        if self.path != "/synthesize":
            self._respond(404, b"Not Found", "text/plain")
            return

        try:
            length = int(self.headers.get("Content-Length", "0"))
        except ValueError:
            length = 0

        if length <= 0:
            self._respond(400, b"Empty body", "text/plain")
            return

        try:
            payload = json.loads(self.rfile.read(length))
        except json.JSONDecodeError:
            self._respond(400, b"Invalid JSON", "text/plain")
            return

        text = str(payload.get("text", "")).strip()
        voice = str(payload.get("voice", DEFAULT_VOICE)).strip() or DEFAULT_VOICE
        lang = str(payload.get("lang", DEFAULT_LANG)).strip() or DEFAULT_LANG
        speed = float(payload.get("speed", DEFAULT_SPEED))
        steps = int(payload.get("steps", DEFAULT_STEPS))
        silence_duration = float(payload.get("silence_duration", DEFAULT_SILENCE_DURATION))

        if not text:
            self._respond(400, b"Empty text", "text/plain")
            return

        if voice not in available_voices:
            voice = DEFAULT_VOICE if DEFAULT_VOICE in available_voices else available_voices[0]

        speed = max(0.8, min(1.2, speed))
        steps = max(4, min(12, steps))
        silence_duration = max(0.0, min(0.2, silence_duration))

        print(
            f'Synthesizing voice={voice} lang={lang} speed={speed:.2f} '
            f'steps={steps} silence={silence_duration:.2f} text="{text[:72]}"'
        )

        try:
            with tts_lock:
                result = tts.synthesize(
                    text,
                    voice=voice,
                    lang=lang,
                    steps=steps,
                    speed=speed,
                    silence_duration=silence_duration,
                )
            audio = _resample(np.asarray(result.audio, dtype=np.float32), result.sample_rate, TARGET_SAMPLE_RATE)
            buffer = io.BytesIO()
            sf.write(buffer, audio, TARGET_SAMPLE_RATE, format="WAV", subtype="PCM_16")
            self._respond(200, buffer.getvalue(), "audio/wav")
        except Exception as exc:  # pragma: no cover - depends on runtime model/assets
            print(f"Synthesis error: {exc}", file=sys.stderr)
            self._respond(500, str(exc).encode("utf-8"), "text/plain")

    def _respond(self, code: int, body: bytes, content_type: str) -> None:
        self.send_response(code)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, fmt: str, *args) -> None:  # noqa: A003
        # Silence the default access log.
        return


if __name__ == "__main__":
    HTTPServer((HOST, PORT), TtsHandler).serve_forever()
