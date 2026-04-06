#!/usr/bin/env python3
"""
MLX Whisper HTTP server for the Veloren voice demo.

This server keeps a high-accuracy Whisper model warm on Apple Silicon and
accepts mono WAV requests from the Rust game client:

GET  /health
POST /transcribe
  headers:
    Content-Type: audio/wav
    X-Language: en
    X-Initial-Prompt: optional prompt text
"""

from __future__ import annotations

import io
import json
import os
import shutil
import sys
import threading
import time
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path

import mlx.core as mx
import mlx_whisper
import numpy as np
import soundfile as sf
from mlx_whisper.transcribe import ModelHolder

HOST = os.environ.get("MEMZ_STT_HOST", "127.0.0.1")
PORT = int(os.environ.get("MEMZ_STT_PORT", "8891"))
REQUESTED_MODEL_ID = os.environ.get("MEMZ_STT_MODEL", "").strip()
DEFAULT_LANGUAGE = os.environ.get("MEMZ_STT_LANGUAGE", "en")
DEFAULT_PROMPT = os.environ.get(
    "MEMZ_STT_INITIAL_PROMPT",
    "Short spoken conversational English for a fantasy game. "
    "Expect phrases like hello, who are you, what are you doing, where am I, can you help me.",
)
USE_FP16 = os.environ.get("MEMZ_STT_FP16", "1") != "0"
TARGET_SAMPLE_RATE = 16000
DEFAULT_MODEL_CANDIDATES = (
    "mlx-community/whisper-medium",
    "mlx-community/whisper-tiny",
)
MIN_FREE_DISK_BYTES = {
    "mlx-community/whisper-medium": 1_200_000_000,
    "mlx-community/whisper-tiny": 200_000_000,
}


def _resample(audio: np.ndarray, source_rate: int, target_rate: int) -> np.ndarray:
    if source_rate == target_rate:
        return np.asarray(audio, dtype=np.float32)

    if audio.size == 0:
        return np.asarray(audio, dtype=np.float32)

    target_len = max(1, int(round(audio.shape[0] * target_rate / float(source_rate))))
    source_positions = np.arange(audio.shape[0], dtype=np.float64)
    target_positions = np.linspace(0, audio.shape[0] - 1, num=target_len, dtype=np.float64)
    return np.interp(target_positions, source_positions, audio).astype(np.float32)


def _normalize_audio(audio: np.ndarray) -> np.ndarray:
    audio = np.asarray(audio, dtype=np.float32)
    if audio.ndim == 2:
        audio = audio.mean(axis=1)
    if audio.ndim != 1:
        audio = audio.reshape(-1)
    return np.clip(audio, -1.0, 1.0).astype(np.float32, copy=False)


dtype = mx.float16 if USE_FP16 else mx.float32
stt_lock = threading.Lock()


def _snapshot_dir_for_repo(repo_id: str) -> Path:
    return Path.home() / ".cache" / "huggingface" / "hub" / f"models--{repo_id.replace('/', '--')}" / "snapshots"


def _has_any_cached_snapshot(repo_id: str) -> bool:
    snapshots_dir = _snapshot_dir_for_repo(repo_id)
    return snapshots_dir.exists() and any(path.is_dir() for path in snapshots_dir.iterdir())


def _find_usable_cached_snapshot(repo_id: str) -> str | None:
    snapshots_dir = _snapshot_dir_for_repo(repo_id)
    if not snapshots_dir.exists():
        return None

    snapshots = sorted(
        (path for path in snapshots_dir.iterdir() if path.is_dir()),
        key=lambda path: path.stat().st_mtime,
        reverse=True,
    )
    for snapshot in snapshots:
        if (snapshot / "config.json").exists() and (
            (snapshot / "weights.npz").exists() or (snapshot / "weights.safetensors").exists()
        ):
            return str(snapshot)
    return None


def _has_enough_disk_for_repo(repo_id: str) -> bool:
    required = MIN_FREE_DISK_BYTES.get(repo_id, 1_500_000_000)
    free_bytes = shutil.disk_usage(Path.home()).free
    return free_bytes >= required


def _candidate_model_targets() -> list[tuple[str, str]]:
    candidates: list[tuple[str, str]] = []
    seen_targets: set[str] = set()

    def add_candidate(label: str, target: str) -> None:
        if target in seen_targets:
            return
        seen_targets.add(target)
        candidates.append((label, target))

    if REQUESTED_MODEL_ID:
        cached_snapshot = _find_usable_cached_snapshot(REQUESTED_MODEL_ID)
        if cached_snapshot is not None:
            add_candidate(REQUESTED_MODEL_ID, cached_snapshot)
        elif not _has_any_cached_snapshot(REQUESTED_MODEL_ID):
            add_candidate(REQUESTED_MODEL_ID, REQUESTED_MODEL_ID)
        else:
            print(
                f"Skipping requested STT model {REQUESTED_MODEL_ID}: cached layout is not compatible with mlx_whisper",
                file=sys.stderr,
            )

    for repo_id in DEFAULT_MODEL_CANDIDATES:
        cached_snapshot = _find_usable_cached_snapshot(repo_id)
        if cached_snapshot is not None:
            add_candidate(repo_id, cached_snapshot)
        elif not _has_any_cached_snapshot(repo_id) and _has_enough_disk_for_repo(repo_id):
            add_candidate(repo_id, repo_id)
        else:
            free_gb = shutil.disk_usage(Path.home()).free / (1024 ** 3)
            print(
                f"Skipping STT model {repo_id}: not cached and only {free_gb:.2f} GiB free on disk",
                file=sys.stderr,
            )

    return candidates


def _load_first_available_model() -> tuple[str, str]:
    attempts: list[str] = []
    for label, target in _candidate_model_targets():
        print(f"Loading MLX Whisper candidate: {label} -> {target}")
        try:
            ModelHolder.get_model(target, dtype)
            return label, target
        except Exception as exc:  # pragma: no cover - runtime dependent
            attempts.append(f"{label}: {exc}")
            print(f"Failed to load MLX Whisper candidate {label}: {exc}", file=sys.stderr)

    joined_attempts = "; ".join(attempts) if attempts else "no model candidates were generated"
    raise RuntimeError(f"No compatible MLX Whisper model could be loaded ({joined_attempts})")


ACTIVE_MODEL_LABEL, ACTIVE_MODEL_TARGET = _load_first_available_model()
print(f"MLX Whisper ready at http://{HOST}:{PORT} using {ACTIVE_MODEL_LABEL}")


class SttHandler(BaseHTTPRequestHandler):
    def do_GET(self) -> None:  # noqa: N802
        if self.path != "/health":
            self._respond(404, b"Not Found", "text/plain")
            return

        body = json.dumps(
            {
                "ok": True,
                "model": ACTIVE_MODEL_LABEL,
                "language": DEFAULT_LANGUAGE,
                "sample_rate": TARGET_SAMPLE_RATE,
            }
        ).encode("utf-8")
        self._respond(200, body, "application/json")

    def do_POST(self) -> None:  # noqa: N802
        if self.path != "/transcribe":
            self._respond(404, b"Not Found", "text/plain")
            return

        try:
            length = int(self.headers.get("Content-Length", "0"))
        except ValueError:
            length = 0

        if length <= 0:
            self._respond(400, b"Empty body", "text/plain")
            return

        language = self.headers.get("X-Language", DEFAULT_LANGUAGE).strip() or DEFAULT_LANGUAGE
        initial_prompt = self.headers.get("X-Initial-Prompt", "").strip() or DEFAULT_PROMPT

        try:
            audio, sample_rate = sf.read(io.BytesIO(self.rfile.read(length)), dtype="float32")
        except Exception as exc:
            self._respond(400, f"Invalid WAV payload: {exc}".encode("utf-8"), "text/plain")
            return

        audio = _normalize_audio(audio)
        audio = _resample(audio, int(sample_rate), TARGET_SAMPLE_RATE)

        start = time.perf_counter()
        try:
            with stt_lock:
                result = mlx_whisper.transcribe(
                    audio,
                    path_or_hf_repo=ACTIVE_MODEL_TARGET,
                    verbose=False,
                    language=language,
                    task="transcribe",
                    initial_prompt=initial_prompt,
                    condition_on_previous_text=False,
                    word_timestamps=False,
                    temperature=(0.0, 0.2, 0.4),
                    compression_ratio_threshold=2.4,
                    logprob_threshold=-1.0,
                    no_speech_threshold=0.6,
                    best_of=5,
                )
        except Exception as exc:  # pragma: no cover - runtime dependent
            print(f"STT error: {exc}", file=sys.stderr)
            self._respond(500, str(exc).encode("utf-8"), "text/plain")
            return

        elapsed_ms = int((time.perf_counter() - start) * 1000)
        text = str(result.get("text", "")).strip()
        body = json.dumps(
            {
                "text": text,
                "language": result.get("language", language),
                "elapsed_ms": elapsed_ms,
            }
        ).encode("utf-8")
        self._respond(200, body, "application/json")

    def _respond(self, code: int, body: bytes, content_type: str) -> None:
        self.send_response(code)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, fmt: str, *args) -> None:  # noqa: A003
        return


if __name__ == "__main__":
    HTTPServer((HOST, PORT), SttHandler).serve_forever()
