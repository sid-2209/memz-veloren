#!/usr/bin/env python3
"""
Kokoro TTS HTTP Server for Veloren NPC Voice Dialogue.

Provides SOTA neural text-to-speech synthesis for NPC voices.
The Veloren game client connects to this server to synthesize NPC speech.

Usage:
    pip install kokoro soundfile numpy
    python kokoro_server.py

The server listens on http://localhost:8880

API:
    GET  /health          → 200 OK if ready
    POST /synthesize      → WAV audio bytes
        Body: { "text": "...", "voice": "af_heart", "speed": 1.0 }
        Response: audio/wav bytes (24kHz, 16-bit PCM)

Available Kokoro voices:
    American female:  af_heart, af_bella, af_nicole, af_sarah, af_sky
    American male:    am_adam, am_michael
    British female:   bf_emma, bf_isabella
    British male:     bm_george, bm_lewis
"""

import io
import json
import sys
import threading
from http.server import BaseHTTPRequestHandler, HTTPServer

HOST = "localhost"
PORT = 8880

# ─── Model loading ──────────────────────────────────────────────────────────

print("Loading Kokoro TTS model...")
try:
    from kokoro import KPipeline
    import soundfile as sf
    import numpy as np
except ImportError as e:
    print(f"\nERROR: Missing Python dependencies: {e}")
    print("\nInstall with:")
    print("  pip install kokoro soundfile numpy")
    print("\nFor Apple Silicon (M-series) acceleration:")
    print("  pip install kokoro soundfile numpy torch")
    sys.exit(1)

# Initialize Kokoro pipeline (American English)
# Lang codes: 'a' = American English, 'b' = British English
pipeline = KPipeline(lang_code='a')
pipeline_lock = threading.Lock()
model_ready = True

print(f"✅ Kokoro TTS ready at http://{HOST}:{PORT}")
print("   Voices: af_heart, af_bella, am_adam, am_michael, bm_george, ...")
print("   Press Ctrl+C to stop.\n")


# ─── HTTP handler ────────────────────────────────────────────────────────────

class TtsHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/health":
            self._respond(200, b"OK", "text/plain")
        elif self.path == "/voices":
            voices = json.dumps({
                "american_female": ["af_heart", "af_bella", "af_nicole", "af_sarah", "af_sky"],
                "american_male":   ["am_adam", "am_michael"],
                "british_female":  ["bf_emma", "bf_isabella"],
                "british_male":    ["bm_george", "bm_lewis"],
            }).encode()
            self._respond(200, voices, "application/json")
        else:
            self._respond(404, b"Not Found", "text/plain")

    def do_POST(self):
        if self.path != "/synthesize":
            self._respond(404, b"Not Found", "text/plain")
            return

        content_len = int(self.headers.get("Content-Length", 0))
        if content_len == 0:
            self._respond(400, b"Empty body", "text/plain")
            return

        try:
            body = json.loads(self.rfile.read(content_len))
        except json.JSONDecodeError:
            self._respond(400, b"Invalid JSON", "text/plain")
            return

        text  = body.get("text", "").strip()
        voice = body.get("voice", "af_heart")
        speed = float(body.get("speed", 1.0))

        if not text:
            self._respond(400, b"Empty text", "text/plain")
            return

        # Clamp speed to reasonable range
        speed = max(0.5, min(2.0, speed))

        # Validate voice
        valid_voices = {
            "af_heart", "af_bella", "af_nicole", "af_sarah", "af_sky",
            "am_adam", "am_michael",
            "bf_emma", "bf_isabella",
            "bm_george", "bm_lewis",
        }
        if voice not in valid_voices:
            print(f"  Unknown voice '{voice}', using af_heart")
            voice = "af_heart"

        try:
            wav_bytes = self._synthesize(text, voice, speed)
            self._respond(200, wav_bytes, "audio/wav")
        except Exception as e:
            print(f"  Synthesis error: {e}", file=sys.stderr)
            self._respond(500, str(e).encode(), "text/plain")

    def _synthesize(self, text: str, voice: str, speed: float) -> bytes:
        """Run Kokoro TTS and return WAV bytes (24kHz, 16-bit PCM)."""
        print(f"  Synthesizing: voice={voice} speed={speed:.2f} text=\"{text[:60]}\"")

        chunks = []
        with pipeline_lock:
            for _, _, audio_chunk in pipeline(text, voice=voice, speed=speed):
                chunks.append(audio_chunk)

        if not chunks:
            raise RuntimeError("Kokoro produced no audio chunks")

        import numpy as np
        audio = np.concatenate(chunks)

        # Encode to WAV (16-bit PCM at 24kHz)
        buf = io.BytesIO()
        sf.write(buf, audio, 24000, format="WAV", subtype="PCM_16")
        wav_bytes = buf.getvalue()

        print(f"  → {len(audio)/24000:.2f}s audio, {len(wav_bytes)} bytes WAV")
        return wav_bytes

    def _respond(self, code: int, body: bytes, content_type: str):
        self.send_response(code)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, fmt, *args):
        # Suppress default HTTP access log (we do our own above)
        pass


# ─── Main ────────────────────────────────────────────────────────────────────

if __name__ == "__main__":
    server = HTTPServer((HOST, PORT), TtsHandler)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nKokoro TTS server stopped.")
