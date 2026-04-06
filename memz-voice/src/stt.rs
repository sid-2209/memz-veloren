//! Speech-to-Text engine using Whisper.
//!
//! SOTA overhaul: Integrates VAD for speech detection, uses proper
//! resampling, and supports both PTT and continuous listening modes.

use crate::error::{Result, VoiceError};
use crate::resampler::Resampler;
use crate::vad::{Vad, VadEvent};

use cpal::{Sample, SampleFormat, SizedSample};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

const WHISPER_SAMPLE_RATE: u32 = 16000;
const WHISPER_CHUNK_SIZE: usize = 1600; // 100ms at 16kHz
const TRANSCRIPTION_TARGET_PEAK: f32 = 0.88;
const TRANSCRIPTION_TARGET_RMS: f32 = 0.085;
const MAX_TRANSCRIPTION_GAIN: f32 = 10.0;
const TRIM_SILENCE_FLOOR: f32 = 0.0015;
const TRIM_SILENCE_PEAK_RATIO: f32 = 0.01;
const TRIM_SILENCE_PADDING_SAMPLES: usize = 1600; // 100ms at 16kHz
const PTT_PREROLL_MS: u32 = 350;
const PTT_STOP_FLUSH_MS: u64 = 35;
const CAPTURE_DESYNC_RATIO_THRESHOLD: f32 = 1.75;
const CAPTURE_DESYNC_SLACK_MS: u64 = 750;
const PRIMARY_BEAM_SIZE: i32 = 4;
const VERIFICATION_BEAM_SIZE: i32 = 6;
const MIN_VERIFY_AUDIO_SAMPLES: usize = 16_000 / 3; // ~333ms at 16kHz
const MAX_VERIFY_AUDIO_SAMPLES: usize = 16_000 * 10; // 10s cap to keep verification bounded
const LOW_CONFIDENCE_TOKEN_PROB: f32 = 0.52;
const SHORT_UTTERANCE_LOW_CONFIDENCE_TOKEN_PROB: f32 = 0.68;
const VERY_LOW_TOKEN_PROB: f32 = 0.18;
const VERIFIED_CANDIDATE_MARGIN: f32 = 0.03;
const GAMEPLAY_STT_INITIAL_PROMPT: &str =
    "Short spoken conversational English for a fantasy game. \
     Expect phrases like hello, who are you, what are you doing, where am I, can you help me.";

/// Configuration for the STT engine.
#[derive(Debug, Clone)]
pub struct SttConfig {
    /// Path to the Whisper model file (.bin).
    pub model_path: String,
    /// Optional stronger local Whisper verifier model (.bin). When present,
    /// low-confidence transcripts are re-checked on this model before we send
    /// text to the NPC pipeline.
    pub verification_model_path: String,
    /// Optional HTTP STT backend URL. When healthy, this is preferred over the
    /// in-process whisper-rs path.
    pub server_url: String,
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
            verification_model_path: String::new(),
            server_url: String::new(),
            language: "en".to_string(),
            vad_threshold: 0.55,
            use_gpu: true,
        }
    }
}

struct LocalSttBackend {
    model_label: String,
    whisper_ctx: whisper_rs::WhisperContext,
}

struct HttpSttBackend {
    url: String,
    client: reqwest::blocking::Client,
    model_label: Option<String>,
}

enum SttBackend {
    LocalOnly(LocalSttBackend),
    HttpOnly(HttpSttBackend),
    Hybrid {
        http: HttpSttBackend,
        local: LocalSttBackend,
    },
}

/// SOTA Speech-to-Text engine.
///
/// Captures audio from the microphone, runs VAD to detect speech,
/// resamples to 16kHz mono, and transcribes using Whisper.
pub struct SpeechToText {
    /// Primary STT backend. Can be an in-process Whisper context or a local
    /// HTTP server backed by a larger accelerator-friendly model.
    backend: SttBackend,
    /// Optional local verifier backend used only for low-confidence transcripts.
    verification_backend: Option<LocalSttBackend>,
    /// Rolling audio buffer, drained every ~50ms by try_transcribe_partial for VAD.
    audio_buffer: Arc<Mutex<Vec<f32>>>,
    /// Complete audio buffer — accumulates every sample from start_recording()
    /// to stop_and_transcribe(). Never drained mid-recording. Used for PTT
    /// transcription so the full utterance is always available.
    complete_buffer: Arc<Mutex<Vec<f32>>>,
    /// Rolling microphone history kept live even when PTT is idle so we can
    /// prepend the first few hundred milliseconds of speech on key-down.
    pre_roll_buffer: Arc<Mutex<VecDeque<f32>>>,
    /// Audio stream handle (keeps the stream alive).
    _stream: Option<cpal::Stream>,
    /// Whether the always-on callback should copy captured samples into the
    /// active recording buffers.
    recording_gate: Arc<AtomicBool>,
    /// Guards against stale callbacks from an older stream if the input stream
    /// is ever rebuilt.
    active_stream_generation: Arc<AtomicU64>,
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
    /// Whisper language hint.
    language: String,
    /// Human-readable device name for logs and diagnostics.
    input_device_name: String,
    /// Max samples retained in the live pre-roll ring buffer.
    pre_roll_capacity_samples: usize,
    /// Wall-clock start time of the active push-to-talk capture window.
    recording_started_at: Option<Instant>,
    /// Whether local Whisper should request GPU acceleration.
    use_gpu: bool,
}

#[derive(Debug, Clone)]
struct TranscriptionCandidate {
    text: String,
    backend_label: String,
    model_tier: u8,
    confidence: f32,
    min_token_prob: f32,
    lexical_token_count: usize,
    has_token_confidence: bool,
}

#[derive(Clone, Copy, Debug)]
enum LocalDecodeProfile {
    Realtime,
    Verification,
}

impl SpeechToText {
    /// Create a new STT engine with the specified configuration.
    ///
    /// Loads the Whisper model and sets up the microphone capture.
    pub fn new(config: SttConfig) -> Result<Self> {
        log::info!("Initializing STT engine...");

        let http_backend = build_http_backend(&config.server_url)?;
        if let Some(http_backend) = &http_backend {
            if probe_http_backend(http_backend) {
                log::info!("HTTP STT backend is healthy at init: {}", http_backend.url);
            } else {
                log::warn!(
                    "HTTP STT backend {} is configured but unavailable at init. \
                     It will be retried automatically and fall back to local Whisper when possible.",
                    http_backend.url
                );
            }
        }

        let local_backend = if config.model_path.is_empty() {
            None
        } else if !std::path::Path::new(&config.model_path).exists() {
            if http_backend.is_some() {
                log::warn!(
                    "Local Whisper model not found at {}. The pipeline will rely on the HTTP STT backend.",
                    config.model_path
                );
                None
            } else {
                return Err(VoiceError::ModelNotFound(config.model_path));
            }
        } else {
            log::info!("Loading local whisper-rs fallback: {}", config.model_path);
            Some(load_local_backend(&config.model_path, config.use_gpu)?)
        };

        let verification_backend = if config.verification_model_path.trim().is_empty() {
            None
        } else if config.verification_model_path == config.model_path {
            log::info!(
                "Skipping duplicate STT verifier model because it matches the primary model: {}",
                config.verification_model_path
            );
            None
        } else if !std::path::Path::new(&config.verification_model_path).exists() {
            log::warn!(
                "Configured STT verifier model not found at {}. Continuing without verifier.",
                config.verification_model_path
            );
            None
        } else {
            log::info!(
                "Loading local Whisper verifier model: {}",
                config.verification_model_path
            );
            Some(load_local_backend(
                &config.verification_model_path,
                config.use_gpu,
            )?)
        };

        let backend = match (http_backend, local_backend) {
            (Some(http), Some(local)) => {
                log::info!("Configured hybrid STT stack: HTTP primary, local Whisper fallback");
                SttBackend::Hybrid { http, local }
            }
            (Some(http), None) => {
                log::info!("Configured HTTP-only STT backend: {}", http.url);
                SttBackend::HttpOnly(http)
            }
            (None, Some(local)) => {
                log::info!("Using local whisper-rs backend only");
                SttBackend::LocalOnly(local)
            }
            (None, None) => {
                log::warn!("No Whisper model path specified and no STT server configured");
                return Err(VoiceError::ModelError(
                    "Either a local Whisper model path or an STT server URL is required."
                        .to_string(),
                ));
            }
        };

        // Set up microphone
        let host = cpal::default_host();
        let input_device = pick_input_device(&host)?;

        let input_config = input_device
            .default_input_config()
            .map_err(|e| VoiceError::AudioError(format!("No input config: {}", e)))?;

        let native_sample_rate = input_config.sample_rate().0;
        let native_channels = input_config.channels();
        let input_device_name = input_device.name().unwrap_or_default();

        log::info!(
            "Microphone: {} ({}Hz, {} ch)",
            input_device_name,
            native_sample_rate,
            native_channels,
        );

        // Set up resampler (native rate → 16kHz for Whisper)
        let resampler = Resampler::new(native_sample_rate, WHISPER_SAMPLE_RATE);

        // Set up VAD
        let vad = Vad::new(config.vad_threshold, WHISPER_SAMPLE_RATE)?;

        // Shared audio buffers
        let audio_buffer = Arc::new(Mutex::new(Vec::with_capacity(native_sample_rate as usize * 30)));
        let complete_buffer = Arc::new(Mutex::new(Vec::with_capacity(native_sample_rate as usize * 60)));
        let pre_roll_capacity_samples =
            ((native_sample_rate as f32) * (PTT_PREROLL_MS as f32 / 1000.0)).round() as usize;
        let pre_roll_buffer = Arc::new(Mutex::new(VecDeque::with_capacity(
            pre_roll_capacity_samples.max(1),
        )));
        let recording_gate = Arc::new(AtomicBool::new(false));
        let active_stream_generation = Arc::new(AtomicU64::new(0));

        let mut stt = Self {
            backend,
            verification_backend,
            audio_buffer,
            complete_buffer,
            pre_roll_buffer,
            _stream: None,
            recording_gate,
            active_stream_generation,
            vad,
            resampler,
            is_recording: false,
            native_sample_rate,
            native_channels,
            language: config.language,
            input_device_name,
            pre_roll_capacity_samples,
            recording_started_at: None,
            use_gpu: config.use_gpu,
        };

        stt.attach_input_stream(&input_device, input_config)?;
        Ok(stt)
    }

    fn attach_input_stream(
        &mut self,
        device: &cpal::Device,
        supported_config: cpal::SupportedStreamConfig,
    ) -> Result<()> {
        let sample_format = supported_config.sample_format();
        let config = supported_config.config();
        let channels = config.channels as usize;
        let sample_rate = config.sample_rate.0;

        self.native_sample_rate = sample_rate;
        self.native_channels = channels as u16;
        self.resampler = Resampler::new(sample_rate, WHISPER_SAMPLE_RATE);
        self.pre_roll_capacity_samples =
            ((sample_rate as f32) * (PTT_PREROLL_MS as f32 / 1000.0)).round() as usize;
        self.input_device_name = device.name().unwrap_or_default();
        self.pre_roll_buffer.lock().unwrap().clear();

        let buffer_clone = Arc::clone(&self.audio_buffer);
        let complete_clone = Arc::clone(&self.complete_buffer);
        let pre_roll_clone = Arc::clone(&self.pre_roll_buffer);
        let recording_gate = Arc::clone(&self.recording_gate);
        let active_stream_generation = Arc::clone(&self.active_stream_generation);
        let stream_generation = active_stream_generation.fetch_add(1, Ordering::SeqCst) + 1;
        let pre_roll_capacity_samples = self.pre_roll_capacity_samples.max(1);

        let stream = match sample_format {
            SampleFormat::F32 => build_input_stream::<f32>(
                device,
                &config,
                channels,
                buffer_clone,
                complete_clone,
                pre_roll_clone,
                pre_roll_capacity_samples,
                recording_gate,
                active_stream_generation,
                stream_generation,
            )?,
            SampleFormat::I8 => build_input_stream::<i8>(
                device,
                &config,
                channels,
                buffer_clone,
                complete_clone,
                pre_roll_clone,
                pre_roll_capacity_samples,
                recording_gate,
                active_stream_generation,
                stream_generation,
            )?,
            SampleFormat::I16 => build_input_stream::<i16>(
                device,
                &config,
                channels,
                buffer_clone,
                complete_clone,
                pre_roll_clone,
                pre_roll_capacity_samples,
                recording_gate,
                active_stream_generation,
                stream_generation,
            )?,
            SampleFormat::I32 => build_input_stream::<i32>(
                device,
                &config,
                channels,
                buffer_clone,
                complete_clone,
                pre_roll_clone,
                pre_roll_capacity_samples,
                recording_gate,
                active_stream_generation,
                stream_generation,
            )?,
            SampleFormat::I64 => build_input_stream::<i64>(
                device,
                &config,
                channels,
                buffer_clone,
                complete_clone,
                pre_roll_clone,
                pre_roll_capacity_samples,
                recording_gate,
                active_stream_generation,
                stream_generation,
            )?,
            SampleFormat::U8 => build_input_stream::<u8>(
                device,
                &config,
                channels,
                buffer_clone,
                complete_clone,
                pre_roll_clone,
                pre_roll_capacity_samples,
                recording_gate,
                active_stream_generation,
                stream_generation,
            )?,
            SampleFormat::U16 => build_input_stream::<u16>(
                device,
                &config,
                channels,
                buffer_clone,
                complete_clone,
                pre_roll_clone,
                pre_roll_capacity_samples,
                recording_gate,
                active_stream_generation,
                stream_generation,
            )?,
            SampleFormat::U32 => build_input_stream::<u32>(
                device,
                &config,
                channels,
                buffer_clone,
                complete_clone,
                pre_roll_clone,
                pre_roll_capacity_samples,
                recording_gate,
                active_stream_generation,
                stream_generation,
            )?,
            SampleFormat::U64 => build_input_stream::<u64>(
                device,
                &config,
                channels,
                buffer_clone,
                complete_clone,
                pre_roll_clone,
                pre_roll_capacity_samples,
                recording_gate,
                active_stream_generation,
                stream_generation,
            )?,
            SampleFormat::F64 => build_input_stream::<f64>(
                device,
                &config,
                channels,
                buffer_clone,
                complete_clone,
                pre_roll_clone,
                pre_roll_capacity_samples,
                recording_gate,
                active_stream_generation,
                stream_generation,
            )?,
            sample_format => {
                return Err(VoiceError::AudioError(format!(
                    "Unsupported microphone sample format: {:?}",
                    sample_format
                )));
            },
        };

        stream
            .play()
            .map_err(|e| VoiceError::AudioError(format!("Failed to start stream: {}", e)))?;

        self._stream = Some(stream);

        log::info!(
            "Live microphone stream ready on: {} ({}Hz, {}ch, {:?}) with {}ms pre-roll",
            self.input_device_name,
            sample_rate,
            channels,
            sample_format,
            PTT_PREROLL_MS,
        );

        Ok(())
    }

    /// Start recording audio from the microphone.
    ///
    /// Automatically selects the best input device, honoring an explicit
    /// `MEMZ_VOICE_INPUT_DEVICE` override and otherwise preferring the current
    /// system default unless it looks virtual.
    pub fn start_recording(&mut self) -> Result<()> {
        if self.is_recording {
            log::warn!("Already recording — ignoring start_recording()");
            return Ok(());
        }

        if self._stream.is_none() {
            let host = cpal::default_host();
            let device = pick_input_device(&host)?;
            let supported_config = device
                .default_input_config()
                .map_err(|e| VoiceError::AudioError(format!("No input config: {}", e)))?;
            self.attach_input_stream(&device, supported_config)?;
        }

        log::info!("Starting microphone recording on persistent stream...");

        // Clear both buffers
        self.audio_buffer.lock().unwrap().clear();
        self.complete_buffer.lock().unwrap().clear();
        self.vad.reset();

        // Prepend a small history window so PTT can capture the phonemes that
        // often happen right as the player presses V.
        let pre_roll: Vec<f32> = {
            let pre_roll = self.pre_roll_buffer.lock().unwrap();
            pre_roll.iter().copied().collect()
        };
        if !pre_roll.is_empty() {
            self.audio_buffer.lock().unwrap().extend_from_slice(&pre_roll);
            self.complete_buffer.lock().unwrap().extend_from_slice(&pre_roll);
        }

        self.recording_gate.store(true, Ordering::SeqCst);
        self.is_recording = true;
        self.recording_started_at = Some(Instant::now());

        log::info!(
            "Recording on: {} ({}Hz, {}ch) with {}ms pre-roll ({} samples)",
            self.input_device_name,
            self.native_sample_rate,
            self.native_channels,
            PTT_PREROLL_MS,
            pre_roll.len(),
        );

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

        let chunk_size = WHISPER_CHUNK_SIZE;
        let mut completed_speech = None;

        for chunk in resampled.chunks(chunk_size) {
            if let VadEvent::SpeechEnd(audio) = self.vad.process_chunk(chunk) {
                completed_speech = Some(audio);
            }
        }

        if let Some(speech_audio) = completed_speech {
            let start_time = std::time::Instant::now();
            let prepared_audio = preprocess_transcription_audio(&speech_audio);
            let text = self.transcribe_audio(&prepared_audio)?;
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

        // Keep the hot stream alive, but stop copying samples into the active
        // capture buffers and allow any in-flight callback to flush.
        self.recording_gate.store(false, Ordering::SeqCst);
        self.is_recording = false;
        std::thread::sleep(Duration::from_millis(PTT_STOP_FLUSH_MS));
        let recording_elapsed = self
            .recording_started_at
            .take()
            .map(|started_at| started_at.elapsed())
            .unwrap_or_default();

        // Use the COMPLETE buffer — it has every sample since start_recording().
        // audio_buffer is drained every ~50ms by try_transcribe_partial (for VAD),
        // so it only holds the last ~50ms. complete_buffer is never drained.
        let mut raw_audio = {
            let mut buf = self.complete_buffer.lock().unwrap();
            std::mem::take(&mut *buf)
        };

        if raw_audio.is_empty() {
            log::warn!("No audio captured");
            return Ok(String::new());
        }

        if let Some(recovered_audio) = sanitize_capture_against_wall_clock(
            raw_audio,
            self.native_sample_rate,
            recording_elapsed,
            self.pre_roll_capacity_samples,
        ) {
            raw_audio = recovered_audio;
        } else {
            log::warn!("Captured audio failed sanity checks after stop; discarding");
            return Ok(String::new());
        }

        log::info!(
            "PTT: wall {:.2}s, captured {:.2}s ({} samples at {}Hz)",
            recording_elapsed.as_secs_f32(),
            raw_audio.len() as f32 / self.native_sample_rate as f32,
            raw_audio.len(),
            self.native_sample_rate,
        );

        // Resample to 16kHz for Whisper
        let resampled = self.resampler.resample(&raw_audio)?;

        // Run VAD to check if there's actual speech
        // Feed the whole buffer through VAD in chunks
        let chunk_size = WHISPER_CHUNK_SIZE;
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
        let min_ptt_samples = 4000usize; // 0.25s at 16kHz — still ignores taps but is more forgiving
        let speech_audio = if has_speech {
            // For push-to-talk, trust the full captured window. Using the VAD
            // speech buffer here can clip quiet initial phonemes and destroy the
            // exact utterance the user intended.
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

        self.vad.reset();

        if speech_audio.is_empty() {
            return Ok(String::new());
        }

        let prepared_audio = preprocess_transcription_audio(&speech_audio);
        if prepared_audio.is_empty() {
            log::warn!("Prepared transcription audio ended up empty after trimming");
            return Ok(String::new());
        }

        // Log audio levels to diagnose microphone issues
        let stats = audio_stats(&prepared_audio);
        log::info!(
            "Audio levels: RMS={:.4} peak={:.4} ({} samples) — {}",
            stats.rms, stats.peak, prepared_audio.len(),
            if stats.rms < 0.001 { "⚠️ VERY QUIET — check mic permissions/input device" }
            else if stats.rms < 0.01 { "⚠️ quiet mic — may affect accuracy" }
            else { "✓ good levels" }
        );

        // Transcribe with Whisper
        let start_time = std::time::Instant::now();
        let raw_text = self.transcribe_audio(&prepared_audio)?;
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
        let mut candidate = match &self.backend {
            SttBackend::LocalOnly(local) => {
                log::info!("Transcribing via local whisper-rs backend");
                self.transcribe_audio_local(local, audio, LocalDecodeProfile::Realtime)
            }
            SttBackend::HttpOnly(http_backend) => {
                log::info!(
                    "Transcribing via HTTP STT backend: {} ({})",
                    http_backend.url,
                    http_backend.model_label.as_deref().unwrap_or("unknown model"),
                );
                self.transcribe_audio_http(http_backend, audio)
            }
            SttBackend::Hybrid { http, local } => {
                if should_prefer_http_backend(http, local) {
                    log::info!(
                        "Transcribing via HTTP STT backend: {} ({})",
                        http.url,
                        http.model_label.as_deref().unwrap_or("unknown model"),
                    );
                    match self.transcribe_audio_http(http, audio) {
                        Ok(candidate) => Ok(candidate),
                        Err(err) => {
                            log::warn!(
                                "HTTP STT backend failed ({}). Falling back to local whisper-rs.",
                                err
                            );
                            self.transcribe_audio_local(local, audio, LocalDecodeProfile::Realtime)
                        }
                    }
                } else {
                    log::info!(
                        "Transcribing via local whisper-rs backend (preferred over HTTP model {})",
                        http.model_label.as_deref().unwrap_or("unknown"),
                    );
                    match self.transcribe_audio_local(local, audio, LocalDecodeProfile::Realtime) {
                        Ok(candidate) => Ok(candidate),
                        Err(err) => {
                            log::warn!(
                                "Local whisper-rs backend failed ({}). Falling back to HTTP STT.",
                                err
                            );
                            self.transcribe_audio_http(http, audio)
                        }
                    }
                }
            }
        }?;

        candidate = self.maybe_verify_transcription(audio, candidate)?;

        let cleaned = cleaned_transcript(&candidate.text).to_string();
        log::info!(
            "Selected STT transcript from {} (confidence {:.2}, min {:.2}, lexical tokens {}): {:?}",
            candidate.backend_label,
            candidate.confidence,
            candidate.min_token_prob,
            candidate.lexical_token_count,
            cleaned
        );

        Ok(cleaned)
    }

    fn transcribe_audio_local(
        &self,
        backend: &LocalSttBackend,
        audio: &[f32],
        profile: LocalDecodeProfile,
    ) -> Result<TranscriptionCandidate> {
        let mut state = backend
            .whisper_ctx
            .create_state()
            .map_err(|e| VoiceError::SttError(format!("Failed to create state: {}", e)))?;

        let mut params = whisper_rs::FullParams::new(match profile {
            LocalDecodeProfile::Realtime => whisper_rs::SamplingStrategy::BeamSearch {
                beam_size: PRIMARY_BEAM_SIZE,
                patience: 1.0,
            },
            LocalDecodeProfile::Verification => whisper_rs::SamplingStrategy::BeamSearch {
                beam_size: VERIFICATION_BEAM_SIZE,
                patience: 1.0,
            },
        });

        // Tune for short conversational English with better robustness to
        // accents and clipped gameplay utterances. The first pass remains
        // bounded for latency; a stricter verification pass only runs for
        // low-confidence transcripts.
        params.set_language(Some(&self.language));
        params.set_translate(false);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_no_timestamps(true);
        params.set_single_segment(false);
        params.set_no_context(false);
        params.set_n_threads(self.decode_thread_count(profile));
        params.set_n_max_text_ctx(16384);
        params.set_max_len(match profile {
            LocalDecodeProfile::Realtime => 96,
            LocalDecodeProfile::Verification => 128,
        });
        params.set_max_tokens(match profile {
            LocalDecodeProfile::Realtime => 64,
            LocalDecodeProfile::Verification => 96,
        });
        params.set_temperature(0.0);
        params.set_temperature_inc(0.0);
        params.set_entropy_thold(2.4);
        params.set_logprob_thold(match profile {
            LocalDecodeProfile::Realtime => -0.8,
            LocalDecodeProfile::Verification => -0.6,
        });
        params.set_no_speech_thold(match profile {
            LocalDecodeProfile::Realtime => 0.45,
            LocalDecodeProfile::Verification => 0.35,
        });
        params.set_suppress_non_speech_tokens(true);
        params.set_initial_prompt(GAMEPLAY_STT_INITIAL_PROMPT);

        // Suppress non-speech tokens for cleaner output
        params.set_suppress_blank(true);

        log::info!(
            "Running local Whisper decode [{}] on {} with {} threads",
            match profile {
                LocalDecodeProfile::Realtime => "realtime",
                LocalDecodeProfile::Verification => "verification",
            },
            backend.model_label,
            self.decode_thread_count(profile),
        );

        state
            .full(params, audio)
            .map_err(|e| VoiceError::SttError(format!("Whisper transcription failed: {}", e)))?;

        build_candidate_from_state(&state, &backend.model_label)
    }

    fn transcribe_audio_http(
        &self,
        http_backend: &HttpSttBackend,
        audio: &[f32],
    ) -> Result<TranscriptionCandidate> {
        #[derive(serde::Deserialize)]
        struct SttServerResponse {
            text: String,
        }

        let mut cursor = std::io::Cursor::new(Vec::new());
        {
            let spec = hound::WavSpec {
                channels: 1,
                sample_rate: WHISPER_SAMPLE_RATE,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            };
            let mut writer = hound::WavWriter::new(&mut cursor, spec)
                .map_err(|e| VoiceError::SttError(format!("Failed to create STT WAV payload: {}", e)))?;
            for &sample in audio {
                let pcm = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                writer
                    .write_sample(pcm)
                    .map_err(|e| VoiceError::SttError(format!("Failed to encode STT WAV payload: {}", e)))?;
            }
            writer
                .finalize()
                .map_err(|e| VoiceError::SttError(format!("Failed to finalize STT WAV payload: {}", e)))?;
        }

        let response = http_backend
            .client
            .post(format!("{}/transcribe", http_backend.url))
            .header("Content-Type", "audio/wav")
            .header("X-Language", &self.language)
            .header("X-Initial-Prompt", GAMEPLAY_STT_INITIAL_PROMPT)
            .body(cursor.into_inner())
            .send()
            .map_err(|e| VoiceError::SttError(format!("STT server request failed: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .unwrap_or_else(|_| "failed to read error body".to_string());
            return Err(VoiceError::SttError(format!(
                "STT server error {}: {}",
                status, body
            )));
        }

        let payload: SttServerResponse = response
            .json()
            .map_err(|e| VoiceError::SttError(format!("Failed to parse STT server response: {}", e)))?;

        let backend_label = http_backend
            .model_label
            .clone()
            .unwrap_or_else(|| http_backend.url.clone());

        Ok(TranscriptionCandidate {
            text: payload.text.trim().to_string(),
            backend_label: backend_label.clone(),
            model_tier: stt_model_tier(&backend_label),
            confidence: 0.0,
            min_token_prob: 0.0,
            lexical_token_count: lexical_token_estimate(&payload.text),
            has_token_confidence: false,
        })
    }

    /// Check if currently recording.
    pub fn is_recording(&self) -> bool {
        self.is_recording
    }

    fn decode_thread_count(&self, profile: LocalDecodeProfile) -> i32 {
        let available = std::thread::available_parallelism()
            .map(|count| count.get())
            .unwrap_or(4);

        let reserved_for_gameplay = if self.use_gpu { 4 } else { 2 };
        let bounded = available
            .saturating_sub(reserved_for_gameplay)
            .clamp(2, if self.use_gpu { 4 } else { 6 });

        match profile {
            LocalDecodeProfile::Realtime => bounded as i32,
            LocalDecodeProfile::Verification => bounded.min(4) as i32,
        }
    }

    fn maybe_verify_transcription(
        &self,
        audio: &[f32],
        primary: TranscriptionCandidate,
    ) -> Result<TranscriptionCandidate> {
        if !should_verify_candidate(audio, &primary) {
            return Ok(primary);
        }

        let verifier = self
            .verification_backend
            .as_ref()
            .or_else(|| self.primary_local_backend());

        let Some(verifier) = verifier else {
            log::info!(
                "Transcript looked low-confidence, but no local STT verifier is available. Keeping primary result from {}.",
                primary.backend_label
            );
            return Ok(primary);
        };

        log::info!(
            "Low-confidence transcript detected from {} (confidence {:.2}, min {:.2}). Verifying with {}.",
            primary.backend_label,
            primary.confidence,
            primary.min_token_prob,
            verifier.model_label,
        );

        let verified = self.transcribe_audio_local(verifier, audio, LocalDecodeProfile::Verification)?;
        Ok(select_best_candidate(primary, verified))
    }

    fn primary_local_backend(&self) -> Option<&LocalSttBackend> {
        match &self.backend {
            SttBackend::LocalOnly(local) => Some(local),
            SttBackend::Hybrid { local, .. } => Some(local),
            SttBackend::HttpOnly(_) => None,
        }
    }
}

fn build_http_backend(server_url: &str) -> Result<Option<HttpSttBackend>> {
    let server_url = server_url.trim().trim_end_matches('/');
    if server_url.is_empty() {
        return Ok(None);
    }

    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(2))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| VoiceError::SttError(format!("Failed to build STT HTTP client: {}", e)))?;

    let model_label = fetch_http_backend_model_label(&client, server_url);

    Ok(Some(HttpSttBackend {
        url: server_url.to_string(),
        client,
        model_label,
    }))
}

fn probe_http_backend(http_backend: &HttpSttBackend) -> bool {
    http_backend.model_label.is_some()
}

fn fetch_http_backend_model_label(
    client: &reqwest::blocking::Client,
    server_url: &str,
) -> Option<String> {
    #[derive(serde::Deserialize)]
    struct SttHealthResponse {
        model: Option<String>,
    }

    match client.get(format!("{server_url}/health")).send() {
        Ok(response) if response.status().is_success() => response
            .json::<SttHealthResponse>()
            .ok()
            .and_then(|payload| payload.model),
        Ok(response) => {
            log::warn!(
                "STT server at {}/health returned {}",
                server_url,
                response.status()
            );
            None
        }
        Err(err) => {
            log::warn!(
                "STT server at {}/health is unavailable ({})",
                server_url,
                err
            );
            None
        }
    }
}

fn load_local_backend(model_path: &str, use_gpu: bool) -> Result<LocalSttBackend> {
    let mut params = whisper_rs::WhisperContextParameters::default();
    params.use_gpu(use_gpu);
    if use_gpu {
        params.flash_attn(true);
        params.gpu_device(0);
    }

    let whisper_ctx = whisper_rs::WhisperContext::new_with_params(model_path, params)
        .map_err(|e| VoiceError::ModelError(format!("Failed to load Whisper model: {}", e)))?;

    Ok(LocalSttBackend {
        model_label: model_path.to_string(),
        whisper_ctx,
    })
}

fn build_candidate_from_state(
    state: &whisper_rs::WhisperState,
    backend_label: &str,
) -> Result<TranscriptionCandidate> {
    let num_segments = state
        .full_n_segments()
        .map_err(|e| VoiceError::SttError(format!("Failed to get segments: {}", e)))?;

    let mut text = String::new();
    let mut lexical_prob_sum = 0.0f32;
    let mut lexical_token_count = 0usize;
    let mut all_prob_sum = 0.0f32;
    let mut all_prob_count = 0usize;
    let mut min_token_prob = 1.0f32;

    for segment_idx in 0..num_segments {
        if let Ok(segment) = state.full_get_segment_text(segment_idx) {
            text.push_str(&segment);
        }

        let token_count = state
            .full_n_tokens(segment_idx)
            .map_err(|e| VoiceError::SttError(format!("Failed to get token count: {}", e)))?;

        for token_idx in 0..token_count {
            let prob = state
                .full_get_token_prob(segment_idx, token_idx)
                .unwrap_or(0.0)
                .clamp(0.0, 1.0);
            let token_text = state
                .full_get_token_text_lossy(segment_idx, token_idx)
                .unwrap_or_default();

            all_prob_sum += prob;
            all_prob_count += 1;
            min_token_prob = min_token_prob.min(prob);

            if token_is_lexical(&token_text) {
                lexical_prob_sum += prob;
                lexical_token_count += 1;
            }
        }
    }

    let confidence = if lexical_token_count > 0 {
        lexical_prob_sum / lexical_token_count as f32
    } else if all_prob_count > 0 {
        all_prob_sum / all_prob_count as f32
    } else {
        0.0
    };

    Ok(TranscriptionCandidate {
        text: text.trim().to_string(),
        backend_label: backend_label.to_string(),
        model_tier: stt_model_tier(backend_label),
        confidence,
        min_token_prob: if all_prob_count > 0 { min_token_prob } else { 0.0 },
        lexical_token_count,
        has_token_confidence: all_prob_count > 0,
    })
}

fn token_is_lexical(token: &str) -> bool {
    token.chars().any(|ch| ch.is_ascii_alphanumeric())
}

fn lexical_token_estimate(text: &str) -> usize {
    text.split_whitespace()
        .filter(|token| token.chars().any(|ch| ch.is_ascii_alphanumeric()))
        .count()
}

fn cleaned_transcript(text: &str) -> &str {
    filter_whisper_artifacts(text).trim()
}

fn should_verify_candidate(audio: &[f32], candidate: &TranscriptionCandidate) -> bool {
    if audio.len() < MIN_VERIFY_AUDIO_SAMPLES || audio.len() > MAX_VERIFY_AUDIO_SAMPLES {
        return false;
    }

    let cleaned = cleaned_transcript(&candidate.text);
    if cleaned.is_empty() {
        return true;
    }

    if !candidate.has_token_confidence {
        return false;
    }

    if candidate.lexical_token_count == 0 {
        return true;
    }

    candidate.confidence < LOW_CONFIDENCE_TOKEN_PROB
        || (candidate.lexical_token_count <= 4
            && candidate.confidence < SHORT_UTTERANCE_LOW_CONFIDENCE_TOKEN_PROB)
        || (candidate.min_token_prob < VERY_LOW_TOKEN_PROB
            && candidate.confidence < SHORT_UTTERANCE_LOW_CONFIDENCE_TOKEN_PROB)
}

fn select_best_candidate(
    primary: TranscriptionCandidate,
    verified: TranscriptionCandidate,
) -> TranscriptionCandidate {
    let primary_clean = cleaned_transcript(&primary.text);
    let verified_clean = cleaned_transcript(&verified.text);

    if verified_clean.is_empty() {
        log::info!(
            "Discarding verified transcript from {} because it was empty/artifact-pruned.",
            verified.backend_label
        );
        return primary;
    }

    if primary_clean.is_empty() {
        log::info!(
            "Replacing empty/artifact primary transcript from {} with verified result from {}.",
            primary.backend_label,
            verified.backend_label
        );
        return verified;
    }

    let primary_score = candidate_score(&primary);
    let verified_score = candidate_score(&verified);

    log::info!(
        "STT candidate comparison: primary {} score {:.3} ({:.2} conf) vs verified {} score {:.3} ({:.2} conf)",
        primary.backend_label,
        primary_score,
        primary.confidence,
        verified.backend_label,
        verified_score,
        verified.confidence,
    );

    if verified_clean.eq_ignore_ascii_case(primary_clean) {
        return if verified_score >= primary_score {
            verified
        } else {
            primary
        };
    }

    if verified_score > primary_score + VERIFIED_CANDIDATE_MARGIN {
        log::info!(
            "Using verified STT transcript from {} instead of {}.",
            verified.backend_label,
            primary.backend_label
        );
        verified
    } else {
        primary
    }
}

fn candidate_score(candidate: &TranscriptionCandidate) -> f32 {
    let cleaned = cleaned_transcript(&candidate.text);
    if cleaned.is_empty() {
        return -1.0;
    }

    let lexical_char_count = cleaned
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .count() as f32;
    let total_char_count = cleaned.chars().count().max(1) as f32;
    let lexical_ratio = lexical_char_count / total_char_count;
    let confidence = if candidate.has_token_confidence {
        candidate.confidence
    } else {
        0.72
    };
    let min_prob_penalty = if candidate.has_token_confidence && candidate.min_token_prob < VERY_LOW_TOKEN_PROB {
        0.04
    } else {
        0.0
    };

    confidence
        + lexical_ratio * 0.08
        + candidate.model_tier as f32 * 0.03
        - min_prob_penalty
}

fn build_input_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: usize,
    audio_buffer: Arc<Mutex<Vec<f32>>>,
    complete_buffer: Arc<Mutex<Vec<f32>>>,
    pre_roll_buffer: Arc<Mutex<VecDeque<f32>>>,
    pre_roll_capacity_samples: usize,
    recording_gate: Arc<AtomicBool>,
    active_stream_generation: Arc<AtomicU64>,
    stream_generation: u64,
) -> Result<cpal::Stream>
where
    T: Sample + SizedSample,
    f32: cpal::FromSample<T>,
{
    device
        .build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                if active_stream_generation.load(Ordering::Relaxed) != stream_generation {
                    return;
                }

                let mono = capture_mono_samples(data, channels);
                if mono.is_empty() {
                    return;
                }

                {
                    let mut pre_roll = pre_roll_buffer.lock().unwrap();
                    let overflow = pre_roll
                        .len()
                        .saturating_add(mono.len())
                        .saturating_sub(pre_roll_capacity_samples);
                    if overflow > 0 {
                        pre_roll.drain(0..overflow);
                    }
                    pre_roll.extend(mono.iter().copied());
                }

                if recording_gate.load(Ordering::Relaxed) {
                    audio_buffer.lock().unwrap().extend_from_slice(&mono);
                    complete_buffer.lock().unwrap().extend_from_slice(&mono);
                }
            },
            |err| {
                log::error!("Audio stream error: {}", err);
            },
            None,
        )
        .map_err(|e| VoiceError::AudioError(format!("Failed to open stream: {}", e)))
}

fn capture_mono_samples<T>(data: &[T], channels: usize) -> Vec<f32>
where
    T: Sample,
    f32: cpal::FromSample<T>,
{
    if channels > 1 {
        data.chunks(channels)
            .map(|frame| {
                let sum: f32 = frame.iter().map(|sample| sample.to_sample::<f32>()).sum();
                sum / frame.len() as f32
            })
            .collect()
    } else {
        data.iter().map(|sample| sample.to_sample::<f32>()).collect()
    }
}

fn sanitize_capture_against_wall_clock(
    raw_audio: Vec<f32>,
    native_sample_rate: u32,
    wall_clock_elapsed: Duration,
    pre_roll_capacity_samples: usize,
) -> Option<Vec<f32>> {
    if raw_audio.is_empty() {
        return None;
    }

    let native_sample_rate = native_sample_rate.max(1) as usize;
    let wall_clock_secs = wall_clock_elapsed.as_secs_f32().max(0.0);
    let captured_secs = raw_audio.len() as f32 / native_sample_rate as f32;
    let allowed_secs = (wall_clock_elapsed
        + Duration::from_millis(CAPTURE_DESYNC_SLACK_MS)
        + Duration::from_millis(PTT_STOP_FLUSH_MS))
        .as_secs_f32();
    let allowed_samples = (((allowed_secs * native_sample_rate as f32).ceil() as usize)
        + pre_roll_capacity_samples)
        .max(pre_roll_capacity_samples.max(1));

    if captured_secs <= 0.0 {
        return None;
    }

    if captured_secs > (wall_clock_secs * CAPTURE_DESYNC_RATIO_THRESHOLD)
        && raw_audio.len() > allowed_samples
    {
        let recovered = raw_audio[raw_audio.len().saturating_sub(allowed_samples)..].to_vec();
        log::warn!(
            "Captured audio drifted far beyond wall clock ({:.2}s captured vs {:.2}s held). \
             Using the most recent {:.2}s window instead.",
            captured_secs,
            wall_clock_secs,
            recovered.len() as f32 / native_sample_rate as f32,
        );
        return Some(recovered);
    }

    Some(raw_audio)
}

fn should_prefer_http_backend(http: &HttpSttBackend, local: &LocalSttBackend) -> bool {
    let http_tier = http
        .model_label
        .as_deref()
        .map(stt_model_tier)
        .unwrap_or_default();
    let local_tier = stt_model_tier(&local.model_label);
    http_tier > local_tier
}

fn stt_model_tier(model_label: &str) -> u8 {
    let lower = model_label.to_lowercase();
    if lower.contains("large-v3-turbo") || lower.contains("large") {
        5
    } else if lower.contains("medium") {
        4
    } else if lower.contains("small") {
        3
    } else if lower.contains("base") {
        2
    } else if lower.contains("tiny") {
        1
    } else {
        0
    }
}

/// Choose the best available input device.
///
/// Strategy:
/// 1. Honor `MEMZ_VOICE_INPUT_DEVICE` when provided.
/// 2. Otherwise trust the system default when it looks like a real mic.
/// 3. If the default looks virtual, fall back to the highest-scoring physical mic.
fn pick_input_device(host: &cpal::Host) -> std::result::Result<cpal::Device, VoiceError> {
    let default = host
        .default_input_device()
        .ok_or_else(|| VoiceError::AudioError("No microphone found".to_string()))?;
    let default_name = default.name().unwrap_or_default();

    if let Ok(requested_name) = std::env::var("MEMZ_VOICE_INPUT_DEVICE") {
        let requested_name = requested_name.trim();
        if !requested_name.is_empty() {
            if let Some(device) = find_named_input_device(host, requested_name) {
                log::info!(
                    "Using requested input device '{}' from MEMZ_VOICE_INPUT_DEVICE",
                    device.name().unwrap_or_default()
                );
                return Ok(device);
            }

            log::warn!(
                "Requested input device '{}' was not found. Falling back to system default '{}'",
                requested_name,
                default_name
            );
        }
    }

    if !is_virtual_input_device(&default_name) {
        if is_bluetooth_input_device(&default_name) {
            log::warn!(
                "Using Bluetooth input device '{}' because it is the system default. \
                 If macOS routes silence, choose another input in System Settings or set MEMZ_VOICE_INPUT_DEVICE.",
                default_name
            );
        } else {
            log::info!("Using system default input device: {}", default_name);
        }
        return Ok(default);
    }

    log::warn!(
        "System default input device '{}' looks virtual. Looking for a physical microphone...",
        default_name
    );

    if let Some(device) = find_fallback_physical_input_device(host) {
        log::info!(
            "Using fallback physical input device: {}",
            device.name().unwrap_or_default()
        );
        return Ok(device);
    }

    log::warn!(
        "No physical fallback microphone found. Using virtual/default device '{}'",
        default_name
    );
    Ok(default)
}

#[derive(Clone, Copy)]
struct AudioStats {
    rms: f32,
    peak: f32,
}

fn audio_stats(audio: &[f32]) -> AudioStats {
    if audio.is_empty() {
        return AudioStats { rms: 0.0, peak: 0.0 };
    }

    let rms = (audio.iter().map(|s| s * s).sum::<f32>() / audio.len() as f32).sqrt();
    let peak = audio.iter().fold(0.0f32, |acc, &sample| acc.max(sample.abs()));

    AudioStats { rms, peak }
}

fn preprocess_transcription_audio(audio: &[f32]) -> Vec<f32> {
    if audio.is_empty() {
        return Vec::new();
    }

    let mut processed = audio.to_vec();
    remove_dc_offset(&mut processed);

    let before = audio_stats(&processed);
    let gain = compute_transcription_gain(before);
    if gain > 1.01 {
        for sample in &mut processed {
            *sample = (*sample * gain).clamp(-1.0, 1.0);
        }
    }

    let threshold = (audio_stats(&processed).peak * TRIM_SILENCE_PEAK_RATIO).max(TRIM_SILENCE_FLOOR);
    let trimmed = trim_silence_with_padding(&processed, threshold, TRIM_SILENCE_PADDING_SAMPLES);
    let after = audio_stats(&trimmed);

    log::info!(
        "Prepared transcription audio: gain={:.2}x threshold={:.4} {:.2}s -> {:.2}s (RMS {:.4} -> {:.4}, peak {:.4} -> {:.4})",
        gain,
        threshold,
        audio.len() as f32 / WHISPER_SAMPLE_RATE as f32,
        trimmed.len() as f32 / WHISPER_SAMPLE_RATE as f32,
        before.rms,
        after.rms,
        before.peak,
        after.peak,
    );

    trimmed
}

fn remove_dc_offset(audio: &mut [f32]) {
    if audio.is_empty() {
        return;
    }

    let mean = audio.iter().sum::<f32>() / audio.len() as f32;
    if mean.abs() < 1e-6 {
        return;
    }

    for sample in audio.iter_mut() {
        *sample -= mean;
    }
}

fn compute_transcription_gain(stats: AudioStats) -> f32 {
    if stats.peak <= 0.0 || stats.rms <= 0.0 {
        return 1.0;
    }

    let peak_gain = TRANSCRIPTION_TARGET_PEAK / stats.peak;
    let rms_gain = TRANSCRIPTION_TARGET_RMS / stats.rms;
    peak_gain.min(rms_gain).clamp(1.0, MAX_TRANSCRIPTION_GAIN)
}

fn trim_silence_with_padding(audio: &[f32], threshold: f32, padding: usize) -> Vec<f32> {
    if audio.is_empty() {
        return Vec::new();
    }

    let start = match audio.iter().position(|sample| sample.abs() >= threshold) {
        Some(idx) => idx.saturating_sub(padding),
        None => return audio.to_vec(),
    };
    let end = match audio.iter().rposition(|sample| sample.abs() >= threshold) {
        Some(idx) => (idx + padding + 1).min(audio.len()),
        None => return audio.to_vec(),
    };

    audio[start..end].to_vec()
}

fn is_virtual_input_device(name: &str) -> bool {
    let name = name.to_lowercase();
    name.contains("steam")
        || name.contains("virtual")
        || name.contains("aggregate")
        || name.contains("multi-output")
        || name.contains("loopback")
        || name.contains("soundflower")
        || name.contains("blackhole")
}

fn is_bluetooth_input_device(name: &str) -> bool {
    let name = name.to_lowercase();
    name.contains("airpods")
        || name.contains("bluetooth")
        || name.contains("bose")
        || name.contains("beats")
        || name.contains("sony")
        || name.contains("jabra")
        || name.contains("sennheiser")
}

fn canonical_input_device_name(name: &str) -> String {
    let mut normalized = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch.to_ascii_lowercase());
        } else {
            normalized.push(' ');
        }
    }

    normalized
        .split_whitespace()
        .filter(|token| {
            !matches!(
                *token,
                "microphone" | "mic" | "input" | "device" | "default" | "built" | "in"
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn input_device_name_matches(requested_name: &str, device_name: &str) -> bool {
    let requested = canonical_input_device_name(requested_name);
    let candidate = canonical_input_device_name(device_name);

    !requested.is_empty()
        && !candidate.is_empty()
        && (candidate.contains(&requested) || requested.contains(&candidate))
}

fn find_named_input_device(host: &cpal::Host, requested_name: &str) -> Option<cpal::Device> {
    host.input_devices().ok()?.find(|device| {
        let Ok(name) = device.name() else {
            return false;
        };
        input_device_name_matches(requested_name, &name) && device.default_input_config().is_ok()
    })
}

fn find_fallback_physical_input_device(host: &cpal::Host) -> Option<cpal::Device> {
    host.input_devices()
        .ok()?
        .filter_map(|device| {
            let name = device.name().ok()?;
            if is_virtual_input_device(&name) || device.default_input_config().is_err() {
                return None;
            }
            Some((fallback_input_score(&name), device))
        })
        .max_by_key(|(score, _)| *score)
        .map(|(_, device)| device)
}

fn fallback_input_score(name: &str) -> i32 {
    let lower = name.to_lowercase();
    let mut score = 0;

    if lower.contains("earpods") || lower.contains("headset") || lower.contains("microphone") {
        score += 5;
    }
    if lower.contains("usb") || lower.contains("brio") {
        score += 3;
    }
    if lower.contains("built-in") || lower.contains("macbook") || lower.contains("mac mini") {
        score += 2;
    }
    if is_bluetooth_input_device(&lower) {
        score -= 1;
    }

    score
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
    use std::time::Duration;

    #[test]
    fn test_config_defaults() {
        let config = SttConfig::default();
        assert_eq!(config.language, "en");
        assert!(config.vad_threshold > 0.0);
    }

    #[test]
    fn test_capture_mono_samples_averages_channels() {
        let stereo = [0.8f32, -0.2, 0.4, 0.2];
        let mono = capture_mono_samples(&stereo, 2);
        assert_eq!(mono.len(), 2);
        assert!((mono[0] - 0.3).abs() < 1e-6);
        assert!((mono[1] - 0.3).abs() < 1e-6);
    }

    #[test]
    fn test_sanitize_capture_against_wall_clock_keeps_normal_capture() {
        let native_sample_rate = 48_000;
        let raw_audio = vec![0.1f32; native_sample_rate * 2];
        let sanitized = sanitize_capture_against_wall_clock(
            raw_audio.clone(),
            native_sample_rate as u32,
            Duration::from_secs(2),
            0,
        )
        .unwrap();
        assert_eq!(sanitized.len(), raw_audio.len());
    }

    #[test]
    fn test_sanitize_capture_against_wall_clock_trims_runaway_capture() {
        let native_sample_rate = 44_100;
        let raw_audio = vec![0.1f32; native_sample_rate * 30];
        let sanitized = sanitize_capture_against_wall_clock(
            raw_audio,
            native_sample_rate as u32,
            Duration::from_secs(2),
            (native_sample_rate as f32 * 0.35) as usize,
        )
        .unwrap();
        assert!(sanitized.len() < native_sample_rate * 10);
        assert!(sanitized.len() > native_sample_rate * 2);
    }

    #[test]
    fn test_model_tier_ordering_prefers_stronger_models() {
        assert!(stt_model_tier("mlx-community/whisper-medium") > stt_model_tier("ggml-base.en.bin"));
        assert!(stt_model_tier("openai/whisper-large-v3-turbo") > stt_model_tier("mlx-community/whisper-medium"));
        assert!(stt_model_tier("whisper-tiny") < stt_model_tier("ggml-base.en.bin"));
    }

    #[test]
    fn test_cleaned_transcript_trims_whisper_artifacts() {
        assert_eq!(cleaned_transcript("  [BLANK_AUDIO] "), "");
        assert_eq!(cleaned_transcript("  who are you? "), "who are you?");
    }

    #[test]
    fn test_low_confidence_short_candidate_triggers_verification() {
        let candidate = TranscriptionCandidate {
            text: "oh i view".to_string(),
            backend_label: "whisper-tiny.en.bin".to_string(),
            model_tier: 1,
            confidence: 0.41,
            min_token_prob: 0.09,
            lexical_token_count: 3,
            has_token_confidence: true,
        };

        assert!(should_verify_candidate(&vec![0.1; WHISPER_SAMPLE_RATE as usize], &candidate));
    }

    #[test]
    fn test_confident_candidate_skips_verification() {
        let candidate = TranscriptionCandidate {
            text: "who are you".to_string(),
            backend_label: "whisper-tiny.en.bin".to_string(),
            model_tier: 1,
            confidence: 0.82,
            min_token_prob: 0.51,
            lexical_token_count: 4,
            has_token_confidence: true,
        };

        assert!(!should_verify_candidate(&vec![0.1; WHISPER_SAMPLE_RATE as usize], &candidate));
    }
}
