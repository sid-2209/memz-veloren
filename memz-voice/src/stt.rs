//! Speech-to-Text engine using Whisper.
//!
//! SOTA overhaul: Integrates VAD for speech detection, uses proper
//! resampling, and supports both PTT and continuous listening modes.

use crate::error::{Result, VoiceError};
use crate::resampler::{self, Resampler};
use crate::vad::{Vad, VadEvent};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{Arc, Mutex};

/// Configuration for the STT engine.
#[derive(Debug, Clone)]
pub struct SttConfig {
    /// Path to the Whisper model file (.bin).
    pub model_path: String,
    /// Language for transcription (e.g., "en").
    pub language: String,
    /// VAD sensitivity threshold (0.0-1.0). Higher = more selective.
    pub vad_threshold: f32,
    /// Whether to use GPU acceleration (Metal on macOS).
    pub use_gpu: bool,
}

impl Default for SttConfig {
    fn default() -> Self {
        Self {
            model_path: String::new(),
            language: "en".to_string(),
            vad_threshold: 0.55,
            use_gpu: true,
        }
    }
}

/// SOTA Speech-to-Text engine.
///
/// Captures audio from the microphone, runs VAD to detect speech,
/// resamples to 16kHz mono, and transcribes using Whisper.
pub struct SpeechToText {
    /// Whisper model context (loaded once, reused for all transcriptions).
    whisper_ctx: whisper_rs::WhisperContext,
    /// Rolling audio buffer, drained every ~50ms by try_transcribe_partial for VAD.
    audio_buffer: Arc<Mutex<Vec<f32>>>,
    /// Complete audio buffer — accumulates every sample from start_recording()
    /// to stop_and_transcribe(). Never drained mid-recording. Used for PTT
    /// transcription so the full utterance is always available.
    complete_buffer: Arc<Mutex<Vec<f32>>>,
    /// Audio stream handle (keeps the stream alive).
    _stream: Option<cpal::Stream>,
    /// VAD engine for detecting speech vs. silence.
    vad: Vad,
    /// Resampler for converting mic sample rate to Whisper's 16kHz.
    resampler: Resampler,
    /// Whether we're currently recording.
    is_recording: bool,
    /// The input device's native sample rate.
    native_sample_rate: u32,
    /// Number of channels from the input device.
    native_channels: u16,
}

impl SpeechToText {
    /// Create a new STT engine with the specified configuration.
    ///
    /// Loads the Whisper model and sets up the microphone capture.
    pub fn new(config: SttConfig) -> Result<Self> {
        log::info!("Initializing STT engine...");

        // Validate model path
        if !config.model_path.is_empty() && !std::path::Path::new(&config.model_path).exists() {
            return Err(VoiceError::ModelNotFound(config.model_path));
        }

        // Initialize Whisper
        let whisper_ctx = if config.model_path.is_empty() {
            log::warn!("No Whisper model path specified — STT will return placeholder text");
            // Create a placeholder context — in real usage, model_path must be set
            return Err(VoiceError::ModelError(
                "Whisper model path is required. Download whisper-tiny.en from: \
                 https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin"
                    .to_string(),
            ));
        } else {
            whisper_rs::WhisperContext::new_with_params(
                &config.model_path,
                whisper_rs::WhisperContextParameters::default(),
            )
            .map_err(|e| VoiceError::ModelError(format!("Failed to load Whisper model: {}", e)))?
        };

        // Set up microphone
        let host = cpal::default_host();
        let input_device = host
            .default_input_device()
            .ok_or_else(|| VoiceError::AudioError("No microphone found".to_string()))?;

        let input_config = input_device
            .default_input_config()
            .map_err(|e| VoiceError::AudioError(format!("No input config: {}", e)))?;

        let native_sample_rate = input_config.sample_rate().0;
        let native_channels = input_config.channels();

        log::info!(
            "Microphone: {} ({}Hz, {} ch)",
            input_device.name().unwrap_or_default(),
            native_sample_rate,
            native_channels,
        );

        // Set up resampler (native rate → 16kHz for Whisper)
        let resampler = Resampler::new(native_sample_rate, 16000);

        // Set up VAD
        let vad = Vad::new(config.vad_threshold, 16000)?;

        // Shared audio buffers
        let audio_buffer = Arc::new(Mutex::new(Vec::with_capacity(native_sample_rate as usize * 30)));
        let complete_buffer = Arc::new(Mutex::new(Vec::with_capacity(native_sample_rate as usize * 60)));

        Ok(Self {
            whisper_ctx,
            audio_buffer,
            complete_buffer,
            _stream: None,
            vad,
            resampler,
            is_recording: false,
            native_sample_rate,
            native_channels,
        })
    }

    /// Start recording audio from the microphone.
    ///
    /// Automatically selects the best input device: prefers a non-Bluetooth
    /// device if the default one is AirPods/Bluetooth (which often deliver
    /// silence due to A2DP mode). Falls back to default if no better device found.
    pub fn start_recording(&mut self) -> Result<()> {
        if self.is_recording {
            log::warn!("Already recording — ignoring start_recording()");
            return Ok(());
        }

        log::info!("Starting microphone recording...");

        // Clear both buffers
        self.audio_buffer.lock().unwrap().clear();
        self.complete_buffer.lock().unwrap().clear();
        self.vad.reset();

        let host = cpal::default_host();

        // Device selection: if default device is Bluetooth (AirPods etc.), try
        // to find a built-in mic first. Bluetooth mics often deliver silence when
        // the headphone is in A2DP (high-quality audio output) mode.
        let device = pick_best_input_device(&host)?;

        // Use the device's supported config to avoid format mismatches.
        // We re-query in case the chosen device differs from what was probed in new().
        let supported_config = device
            .default_input_config()
            .map_err(|e| VoiceError::AudioError(format!("No input config: {}", e)))?;

        let channels = supported_config.channels() as usize;
        let sample_rate = supported_config.sample_rate().0;

        // Update resampler if sample rate changed (different device from init)
        if sample_rate != self.native_sample_rate {
            log::info!("Input device sample rate changed: {}Hz → {}Hz, updating resampler", self.native_sample_rate, sample_rate);
            self.resampler = Resampler::new(sample_rate, 16000);
            self.native_sample_rate = sample_rate;
            self.native_channels = channels as u16;
        }

        let config = cpal::StreamConfig {
            channels: channels as u16,
            sample_rate: cpal::SampleRate(sample_rate),
            buffer_size: cpal::BufferSize::Default,
        };

        log::info!(
            "Recording on: {} ({}Hz, {}ch)",
            device.name().unwrap_or_default(),
            sample_rate,
            channels
        );

        let buffer_clone = Arc::clone(&self.audio_buffer);
        let complete_clone = Arc::clone(&self.complete_buffer);

        let stream = device
            .build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let mono = if channels > 1 {
                        resampler::to_mono(data, channels)
                    } else {
                        data.to_vec()
                    };
                    buffer_clone.lock().unwrap().extend_from_slice(&mono);
                    complete_clone.lock().unwrap().extend_from_slice(&mono);
                },
                |err| {
                    log::error!("Audio stream error: {}", err);
                },
                None,
            )
            .map_err(|e| VoiceError::AudioError(format!("Failed to open stream: {}", e)))?;

        stream
            .play()
            .map_err(|e| VoiceError::AudioError(format!("Failed to start stream: {}", e)))?;

        self._stream = Some(stream);
        self.is_recording = true;

        Ok(())
    }

    /// Check the current buffer for completed speech segments (natural pauses).
    /// Extracts the audio internally and transcribes it if found.
    pub fn try_transcribe_partial(&mut self) -> Result<Option<String>> {
        if !self.is_recording {
            return Ok(None);
        }

        let raw_audio = {
            let mut buf = self.audio_buffer.lock().unwrap();
            std::mem::take(&mut *buf)
        };

        if raw_audio.is_empty() {
            return Ok(None);
        }

        let resampled = self.resampler.resample(&raw_audio)?;

        let chunk_size = 1600; // 100ms at 16kHz
        let mut completed_speech = None;

        for chunk in resampled.chunks(chunk_size) {
            if let VadEvent::SpeechEnd(audio) = self.vad.process_chunk(chunk) {
                completed_speech = Some(audio);
            }
        }

        if let Some(speech_audio) = completed_speech {
            let start_time = std::time::Instant::now();
            let text = self.transcribe_audio(&speech_audio)?;
            if !text.is_empty() {
                log::info!("Partial transcription completed in {:.0}ms", start_time.elapsed().as_millis());
                return Ok(Some(text));
            }
        }

        Ok(None)
    }

    /// Stop recording and transcribe the captured audio.
    ///
    /// Returns the transcribed text. Returns empty string if no speech was detected.
    pub fn stop_and_transcribe(&mut self) -> Result<String> {
        if !self.is_recording {
            log::warn!("Not recording — nothing to transcribe");
            return Ok(String::new());
        }

        log::info!("Stopping recording, transcribing...");

        // Stop the stream first (no more callbacks after this)
        self._stream = None;
        self.is_recording = false;

        // Use the COMPLETE buffer — it has every sample since start_recording().
        // audio_buffer is drained every ~50ms by try_transcribe_partial (for VAD),
        // so it only holds the last ~50ms. complete_buffer is never drained.
        let raw_audio = {
            let mut buf = self.complete_buffer.lock().unwrap();
            std::mem::take(&mut *buf)
        };

        if raw_audio.is_empty() {
            log::warn!("No audio captured");
            return Ok(String::new());
        }

        log::info!(
            "PTT: captured {:.1}s of audio ({} samples at {}Hz)",
            raw_audio.len() as f32 / self.native_sample_rate as f32,
            raw_audio.len(),
            self.native_sample_rate,
        );

        // Resample to 16kHz for Whisper
        let resampled = self.resampler.resample(&raw_audio)?;

        // Run VAD to check if there's actual speech
        // Feed the whole buffer through VAD in chunks
        let chunk_size = 1600; // 100ms at 16kHz
        let mut has_speech = false;
        for chunk in resampled.chunks(chunk_size) {
            match self.vad.process_chunk(chunk) {
                VadEvent::SpeechStart | VadEvent::SpeechContinuing => {
                    has_speech = true;
                }
                _ => {}
            }
        }

        // PTT bypass: if the user explicitly held the key, trust them and
        // transcribe all captured audio. VAD is useful for continuous listening
        // (avoiding compute on silence) but for PTT the keypress IS the signal.
        // Whisper returns empty or "[BLANK_AUDIO]" on silence anyway.
        let min_ptt_samples = 8000usize; // 0.5s at 16kHz — ignore accidental taps
        let speech_audio = if let Some(audio) = self.vad.force_end_speech() {
            audio
        } else if has_speech {
            // VAD detected speech but didn't end cleanly — use full resampled
            resampled
        } else if resampled.len() >= min_ptt_samples {
            // PTT fallback: VAD missed it (quiet mic / noise cancellation),
            // but the user held the key long enough — transcribe everything
            log::info!(
                "VAD found no speech but {:.1}s captured — transcribing (PTT fallback)",
                resampled.len() as f32 / 16000.0
            );
            resampled
        } else {
            log::info!("No speech detected by VAD — skipping transcription");
            return Ok(String::new());
        };

        if speech_audio.is_empty() {
            return Ok(String::new());
        }

        // Log audio levels to diagnose microphone issues
        let rms = (speech_audio.iter().map(|s| s * s).sum::<f32>() / speech_audio.len() as f32).sqrt();
        let peak = speech_audio.iter().fold(0.0f32, |acc, &s| acc.max(s.abs()));
        log::info!(
            "Audio levels: RMS={:.4} peak={:.4} ({} samples) — {}",
            rms, peak, speech_audio.len(),
            if rms < 0.001 { "⚠️ VERY QUIET — check mic permissions/input device" }
            else if rms < 0.01 { "⚠️ quiet mic — may affect accuracy" }
            else { "✓ good levels" }
        );

        // Transcribe with Whisper
        let start_time = std::time::Instant::now();
        let raw_text = self.transcribe_audio(&speech_audio)?;
        let elapsed = start_time.elapsed();

        log::info!(
            "Transcription completed in {:.0}ms: \"{}\"",
            elapsed.as_millis(),
            raw_text
        );

        // Filter out Whisper silence/noise artifacts. These appear when there
        // is no real speech: the model hallucinates tokens instead of returning empty.
        if filter_whisper_artifacts(&raw_text).is_empty() {
            log::info!("Whisper artifact filtered: {:?} → empty (skipping)", raw_text);
            return Ok(String::new());
        }

        Ok(raw_text)
    }

    /// Transcribe audio samples using Whisper.
    fn transcribe_audio(&self, audio: &[f32]) -> Result<String> {
        let mut state = self
            .whisper_ctx
            .create_state()
            .map_err(|e| VoiceError::SttError(format!("Failed to create state: {}", e)))?;

        let mut params = whisper_rs::FullParams::new(whisper_rs::SamplingStrategy::Greedy {
            best_of: 1,
        });

        // Optimize for speed
        params.set_language(Some("en"));
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_single_segment(true);
        params.set_no_context(true);
        params.set_n_threads(4);

        // Suppress non-speech tokens for cleaner output
        params.set_suppress_blank(true);

        state
            .full(params, audio)
            .map_err(|e| VoiceError::SttError(format!("Whisper transcription failed: {}", e)))?;

        // Extract text from all segments
        let num_segments = state.full_n_segments()
            .map_err(|e| VoiceError::SttError(format!("Failed to get segments: {}", e)))?;

        let mut text = String::new();
        for i in 0..num_segments {
            if let Ok(segment) = state.full_get_segment_text(i) {
                text.push_str(&segment);
            }
        }

        Ok(text.trim().to_string())
    }

    /// Check if currently recording.
    pub fn is_recording(&self) -> bool {
        self.is_recording
    }
}

/// Choose the best available input device.
///
/// Bluetooth headphones (AirPods, etc.) often deliver silence when in A2DP
/// (high-quality stereo output) mode — the mic only works in the lower-quality
/// HFP/SCO mode, which macOS doesn't switch to automatically in all cases.
///
/// Strategy:
/// 1. List all input devices.
/// 2. If the default is Bluetooth, prefer a wired/built-in device.
/// 3. Fall back to default if no wired device is found.
fn pick_best_input_device(host: &cpal::Host) -> std::result::Result<cpal::Device, VoiceError> {
    use cpal::traits::HostTrait;

    let default = host
        .default_input_device()
        .ok_or_else(|| VoiceError::AudioError("No microphone found".to_string()))?;

    let default_name = default.name().unwrap_or_default().to_lowercase();
    let is_bluetooth = default_name.contains("airpods")
        || default_name.contains("bluetooth")
        || default_name.contains("bose")
        || default_name.contains("beats")
        || default_name.contains("sony")
        || default_name.contains("jabra")
        || default_name.contains("sennheiser");

    if !is_bluetooth {
        log::info!("Using input device: {} (non-Bluetooth, preferred)", default.name().unwrap_or_default());
        return Ok(default);
    }

    log::warn!(
        "Default input device '{}' is Bluetooth — it may deliver silence in A2DP mode. \
         Looking for a built-in microphone...",
        default.name().unwrap_or_default()
    );

    // Try to find a real (non-Bluetooth, non-virtual) microphone
    if let Ok(devices) = host.input_devices() {
        for device in devices {
            let name = device.name().unwrap_or_default().to_lowercase();

            // Skip virtual/software microphones — they always deliver silence
            let is_virtual = name.contains("steam")
                || name.contains("virtual")
                || name.contains("aggregate")
                || name.contains("multi-output")
                || name.contains("loopback")
                || name.contains("soundflower")
                || name.contains("blackhole");

            // Skip Bluetooth devices
            let is_bluetooth = name.contains("airpods")
                || name.contains("bluetooth")
                || name.contains("bose")
                || name.contains("beats")
                || name.contains("sony")
                || name.contains("jabra")
                || name.contains("sennheiser");

            if !is_virtual && !is_bluetooth {
                if device.default_input_config().is_ok() {
                    log::info!("Using real (non-BT, non-virtual) input device: {}", device.name().unwrap_or_default());
                    return Ok(device);
                }
            }
        }
    }

    // No real non-BT mic found (Mac Mini has no built-in mic).
    // Fall back to AirPods — they work if user sets them as input in
    // System Settings → Sound → Input, which forces macOS HFP mode.
    log::warn!(
        "No real non-Bluetooth microphone found. Falling back to '{}'. \
         For best results: System Settings → Sound → Input → select your AirPods. \
         This forces HFP mode so the mic works (audio quality will be lower).",
        default.name().unwrap_or_default()
    );

    // Fall back to default (Bluetooth) with a warning
    log::warn!(
        "No built-in microphone found. Using Bluetooth device '{}'. \
         If you get silence, try: System Settings → Sound → Input → select Built-in Microphone",
        default.name().unwrap_or_default()
    );
    Ok(default)
}

/// Filter out known Whisper silence/noise hallucination tokens.
///
/// When Whisper processes silence or very quiet audio it produces these
/// placeholder tokens instead of an empty result. We treat them as "no speech".
fn filter_whisper_artifacts(text: &str) -> &str {
    // Trim whitespace for matching, but return the original slice
    let trimmed = text.trim();
    let lower = trimmed.to_lowercase();

    // Known Whisper silence/noise artifacts (exact and partial matches)
    let silence_tokens = [
        "[blank_audio]",
        "(blank audio)",
        "[inaudible]",
        "(inaudible)",
        "[noise]",
        "(noise)",
        "[music]",
        "(music)",
        "[applause]",
        "[ blank_audio ]",
        "...",
        ". . .",
    ];

    for token in &silence_tokens {
        if lower == *token || lower.starts_with(token) {
            return "";
        }
    }

    // Also reject very short "transcriptions" that are clearly noise artifacts
    // (single punctuation, brackets only, etc.)
    if trimmed.len() <= 2 && !trimmed.chars().any(|c| c.is_alphabetic()) {
        return "";
    }

    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let config = SttConfig::default();
        assert_eq!(config.language, "en");
        assert!(config.vad_threshold > 0.0);
    }
}
