//! Text-to-Speech engine with SOTA multi-backend synthesis.
//!
//! Backend priority (highest quality first):
//! 1. Blitz TTS HTTP server (http://localhost:8890) — expressive local neural quality
//! 2. Kokoro TTS HTTP server (http://localhost:8880) — neural fallback
//! 3. Optional macOS `say` fallback (disabled by default)
//! 4. Placeholder sine wave — absolute fallback when no real backend is available
//!
//! The Blitz and Kokoro servers are started separately.

use crate::error::{Result, VoiceError};
use crate::voice_profile::VoiceProfile;
use std::time::Duration;

const VOICE_AUDIO_TARGET_PEAK: f32 = 0.92;
const VOICE_AUDIO_TARGET_RMS: f32 = 0.16;
const VOICE_AUDIO_MAX_GAIN: f32 = 3.0;
const VOICE_AUDIO_TRIM_FLOOR: f32 = 0.0015;
const VOICE_AUDIO_TRIM_PEAK_RATIO: f32 = 0.01;
const VOICE_AUDIO_EDGE_FADE_SECS: f32 = 0.02;

/// Which TTS backend is active.
#[derive(Debug, Clone, PartialEq)]
enum TtsBackend {
    Blitz,
    Kokoro,
    MacOsSay,
    Placeholder,
}

/// Configuration for the TTS engine.
#[derive(Debug, Clone)]
pub struct TtsConfig {
    /// URL of the Blitz TTS HTTP server (default: http://localhost:8890).
    pub blitz_server_url: String,
    /// Path to the Kokoro TTS model directory (used if running in-process).
    pub model_path: String,
    /// URL of the Kokoro TTS HTTP server (default: http://localhost:8880).
    pub kokoro_server_url: String,
    /// Output sample rate for audio (Kokoro outputs 24kHz by default).
    pub sample_rate: u32,
    /// Default voice profile if none specified per NPC.
    pub default_voice: VoiceProfile,
    /// Silence Blitz should append after synthesis.
    ///
    /// For gameplay streaming this should stay very small so sentence chunks can
    /// join smoothly without sounding like separate unrelated clips.
    pub blitz_silence_duration: f32,
    /// Whether macOS `say` can be used as a final fallback.
    pub allow_macos_say_fallback: bool,
}

impl Default for TtsConfig {
    fn default() -> Self {
        Self {
            blitz_server_url: "http://localhost:8890".to_string(),
            model_path: String::new(),
            kokoro_server_url: "http://localhost:8880".to_string(),
            sample_rate: 24000,
            default_voice: VoiceProfile::default(),
            blitz_silence_duration: 0.02,
            allow_macos_say_fallback: false,
        }
    }
}

/// SOTA Text-to-Speech engine with automatic backend selection.
///
/// Detects the best available TTS backend at initialization and uses it
/// for all synthesis calls. Falls back gracefully through the priority chain.
pub struct TextToSpeech {
    config: TtsConfig,
    backend: TtsBackend,
    http_client: reqwest::blocking::Client,
}

impl TextToSpeech {
    /// Create a new TTS engine and detect the best available backend.
    pub fn new(config: TtsConfig) -> Result<Self> {
        let http_client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| VoiceError::TtsError(format!("Failed to create HTTP client: {}", e)))?;

        let backend = Self::detect_backend(&config, &http_client);

        log::info!("TTS engine initialized with backend: {:?}", backend);

        Ok(Self {
            config,
            backend,
            http_client,
        })
    }

    /// Create with default settings.
    pub fn with_defaults() -> Result<Self> {
        Self::new(TtsConfig::default())
    }

    /// Detect the best available TTS backend.
    fn detect_backend(config: &TtsConfig, client: &reqwest::blocking::Client) -> TtsBackend {
        // 1. Try Blitz TTS HTTP server
        if Self::server_is_healthy("Blitz TTS", &config.blitz_server_url, client) {
            return TtsBackend::Blitz;
        }

        // 2. Try Kokoro TTS HTTP server
        if Self::server_is_healthy("Kokoro TTS", &config.kokoro_server_url, client) {
            return TtsBackend::Kokoro;
        }

        // 3. Try macOS `say` command if explicitly allowed
        #[cfg(target_os = "macos")]
        if config.allow_macos_say_fallback {
            let result = std::process::Command::new("say").arg("--version").output();
            // `say --version` may fail but that's fine — just check if it exists
            let say_exists = std::process::Command::new("which")
                .arg("say")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

            if say_exists || result.is_ok() {
                log::info!("Using macOS 'say' as the final TTS fallback");
                return TtsBackend::MacOsSay;
            }
        }

        log::warn!(
            "No neural TTS backend available. Start blitz_tts_server.py or kokoro_server.py."
        );
        TtsBackend::Placeholder
    }

    fn server_is_healthy(
        label: &str,
        base_url: &str,
        client: &reqwest::blocking::Client,
    ) -> bool {
        if base_url.is_empty() {
            return false;
        }

        let health_url = format!("{}/health", base_url);
        match client
            .get(&health_url)
            .timeout(Duration::from_millis(500))
            .send()
        {
            Ok(resp) if resp.status().is_success() => {
                log::info!("{} server available at {}", label, base_url);
                true
            }
            Ok(resp) => {
                log::info!(
                    "{} server at {} returned status {}",
                    label,
                    base_url,
                    resp.status()
                );
                false
            }
            Err(e) => {
                log::info!("{} server not available at {} ({})", label, base_url, e);
                false
            }
        }
    }

    fn try_kokoro_fallback(&mut self, text: &str, profile: &VoiceProfile) -> Result<Vec<f32>> {
        if self.config.kokoro_server_url.is_empty() {
            return Err(VoiceError::TtsError(
                "Kokoro fallback URL is not configured".to_string(),
            ));
        }
        self.synthesize_with_kokoro(text, profile)
    }

    #[cfg(target_os = "macos")]
    fn try_macos_say_fallback(&mut self, text: &str, profile: &VoiceProfile) -> Result<Vec<f32>> {
        if !self.config.allow_macos_say_fallback {
            return Err(VoiceError::TtsError(
                "macOS say fallback is disabled in TtsConfig".to_string(),
            ));
        }
        self.synthesize_with_macos_say(text, profile)
    }

    #[cfg(not(target_os = "macos"))]
    fn try_macos_say_fallback(
        &mut self,
        _text: &str,
        _profile: &VoiceProfile,
    ) -> Result<Vec<f32>> {
        Err(VoiceError::TtsError(
            "macOS say is not available on this platform".to_string(),
        ))
    }

    /// Re-check if Kokoro server is now available (useful after startup).
    pub fn refresh_backend(&mut self) {
        let new_backend = Self::detect_backend(&self.config, &self.http_client);
        if new_backend != self.backend {
            log::info!(
                "TTS backend changed: {:?} → {:?}",
                self.backend,
                new_backend
            );
            self.backend = new_backend;
        }
    }

    /// Synthesize speech from text using the given voice profile.
    ///
    /// Returns PCM audio samples at 24kHz mono f32.
    pub fn synthesize(&mut self, text: &str, profile: &VoiceProfile) -> Result<Vec<f32>> {
        if text.is_empty() {
            return Ok(Vec::new());
        }
        self.refresh_backend();

        let start_time = std::time::Instant::now();
        log::info!(
            "TTS synthesizing ({:?}): \"{}\" [voice={}]",
            self.backend,
            &text[..text.len().min(60)],
            profile.voice_id,
        );

        let audio = match &self.backend.clone() {
            TtsBackend::Blitz => {
                match self.synthesize_with_blitz(text, profile) {
                    Ok(a) => a,
                    Err(e) => {
                        log::warn!("Blitz synthesis failed ({}), trying Kokoro fallback", e);
                        match self.try_kokoro_fallback(text, profile) {
                            Ok(a) => {
                                self.backend = TtsBackend::Kokoro;
                                a
                            }
                            Err(e2) => {
                                log::warn!("Kokoro fallback failed: {}", e2);
                                match self.try_macos_say_fallback(text, profile) {
                                    Ok(a) => {
                                        self.backend = TtsBackend::MacOsSay;
                                        a
                                    }
                                    Err(e3) => {
                                        log::warn!("No real fallback TTS available: {}", e3);
                                        self.backend = TtsBackend::Placeholder;
                                        self.generate_placeholder_audio(text, profile)
                                    }
                                }
                            }
                        }
                    }
                }
            }
            TtsBackend::Kokoro => {
                match self.synthesize_with_kokoro(text, profile) {
                    Ok(a) => a,
                    Err(e) => {
                        log::warn!("Kokoro synthesis failed ({}), trying remaining fallbacks", e);
                        match self.try_macos_say_fallback(text, profile) {
                            Ok(a) => {
                                self.backend = TtsBackend::MacOsSay;
                                a
                            }
                            Err(e2) => {
                                log::warn!("No real fallback TTS available: {}", e2);
                                self.backend = TtsBackend::Placeholder;
                                self.generate_placeholder_audio(text, profile)
                            }
                        }
                    }
                }
            }
            TtsBackend::MacOsSay => {
                match self.synthesize_with_macos_say(text, profile) {
                    Ok(a) => a,
                    Err(e) => {
                        log::warn!("macOS say failed ({}), using placeholder", e);
                        self.backend = TtsBackend::Placeholder;
                        self.generate_placeholder_audio(text, profile)
                    }
                }
            }
            TtsBackend::Placeholder => self.generate_placeholder_audio(text, profile),
        };

        log::info!(
            "TTS completed in {:.0}ms ({} samples, {:.1}s audio)",
            start_time.elapsed().as_millis(),
            audio.len(),
            audio.len() as f32 / self.config.sample_rate as f32,
        );

        Ok(audio)
    }

    /// Synthesize speech sentence-by-sentence for streaming output.
    ///
    /// Splits text at sentence boundaries and calls `on_chunk` for each.
    /// This lets the NPC start "speaking" before the full response is synthesized.
    pub fn synthesize_streaming<F>(
        &mut self,
        text: &str,
        profile: &VoiceProfile,
        mut on_chunk: F,
    ) -> Result<()>
    where
        F: FnMut(Vec<f32>) -> bool, // Returns false to cancel
    {
        let sentences = split_into_sentences(text);

        for sentence in sentences {
            if sentence.trim().is_empty() {
                continue;
            }

            let audio = self.synthesize(sentence, profile)?;

            if !on_chunk(audio) {
                log::info!("TTS streaming cancelled by consumer");
                break;
            }
        }

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════
    // Backend: Blitz TTS HTTP Server
    // ═══════════════════════════════════════════════════════════

    /// Synthesize using the Blitz TTS HTTP server.
    fn synthesize_with_blitz(&self, text: &str, profile: &VoiceProfile) -> Result<Vec<f32>> {
        let url = format!("{}/synthesize", self.config.blitz_server_url);
        let blitz_voice = profile_to_blitz_voice(profile);

        let body = serde_json::json!({
            "text": text,
            "voice": blitz_voice,
            "lang": "en",
            "speed": profile.speed.clamp(0.8, 1.2),
            "steps": 7,
            "silence_duration": self.config.blitz_silence_duration.clamp(0.0, 0.2),
        });

        log::debug!("Blitz request: voice={} speed={:.2}", blitz_voice, profile.speed);

        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .map_err(|e| VoiceError::TtsError(format!("Blitz server request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(VoiceError::TtsError(format!(
                "Blitz server error {}: {}",
                status, body
            )));
        }

        let wav_bytes = response
            .bytes()
            .map_err(|e| VoiceError::TtsError(format!("Failed to read Blitz response: {}", e)))?;

        parse_wav_to_f32(&wav_bytes)
    }

    // ═══════════════════════════════════════════════════════════
    // Backend: Kokoro TTS HTTP Server
    // ═══════════════════════════════════════════════════════════

    /// Synthesize using the Kokoro TTS HTTP server.
    ///
    /// Sends text + voice params to the local server, receives WAV audio.
    fn synthesize_with_kokoro(&self, text: &str, profile: &VoiceProfile) -> Result<Vec<f32>> {
        let url = format!("{}/synthesize", self.config.kokoro_server_url);
        let kokoro_voice = profile_to_kokoro_voice(profile);

        let body = serde_json::json!({
            "text": text,
            "voice": kokoro_voice,
            "speed": profile.speed,
        });

        log::debug!("Kokoro request: voice={} speed={:.2}", kokoro_voice, profile.speed);

        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .map_err(|e| VoiceError::TtsError(format!("Kokoro server request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(VoiceError::TtsError(format!(
                "Kokoro server error {}: {}",
                status, body
            )));
        }

        let wav_bytes = response
            .bytes()
            .map_err(|e| VoiceError::TtsError(format!("Failed to read Kokoro response: {}", e)))?;

        parse_wav_to_f32(&wav_bytes)
    }

    // ═══════════════════════════════════════════════════════════
    // Backend: macOS `say` Command
    // ═══════════════════════════════════════════════════════════

    /// Synthesize using macOS built-in TTS (`say` command).
    ///
    /// Maps NPC voice profiles to macOS voice names. Outputs 24kHz WAV.
    /// This provides high-quality, natural-sounding speech with no downloads.
    #[cfg(target_os = "macos")]
    fn synthesize_with_macos_say(&self, text: &str, profile: &VoiceProfile) -> Result<Vec<f32>> {
        let voice_name = profile_to_macos_voice(profile);

        // Speech rate: say uses words/minute, default ~200. Map speed multiplier.
        // speed 0.7 → ~140 wpm, speed 1.0 → 200 wpm, speed 1.3 → 260 wpm
        let rate = (200.0 * profile.speed).round() as u32;

        // Unique temp file per call to avoid race conditions
        let tmp_wav = std::env::temp_dir().join(format!(
            "veloren_npc_{}.wav",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .subsec_nanos()
        ));

        log::debug!(
            "say: voice={} rate={} file={}",
            voice_name,
            rate,
            tmp_wav.display()
        );

        // say -v <voice> -r <rate> -o <outfile.wav> --data-format=LEI16@24000 "<text>"
        let status = std::process::Command::new("say")
            .args([
                "-v",
                &voice_name,
                "-r",
                &rate.to_string(),
                "-o",
                tmp_wav.to_str().unwrap_or("/tmp/veloren_npc.wav"),
                "--data-format=LEI16@24000",
                text,
            ])
            .status()
            .map_err(|e| VoiceError::TtsError(format!("Failed to run 'say': {}", e)))?;

        if !status.success() {
            let _ = std::fs::remove_file(&tmp_wav);
            return Err(VoiceError::TtsError(format!(
                "say command exited with: {}",
                status
            )));
        }

        if !tmp_wav.exists() {
            return Err(VoiceError::TtsError(
                "say command produced no output file".to_string(),
            ));
        }

        // Read WAV file
        let wav_bytes = std::fs::read(&tmp_wav)
            .map_err(|e| VoiceError::TtsError(format!("Failed to read WAV: {}", e)))?;
        let _ = std::fs::remove_file(&tmp_wav);

        let audio = parse_wav_to_f32(&wav_bytes)?;

        if audio.is_empty() {
            return Err(VoiceError::TtsError("say produced empty audio".to_string()));
        }

        Ok(audio)
    }

    #[cfg(not(target_os = "macos"))]
    fn synthesize_with_macos_say(&self, _text: &str, _profile: &VoiceProfile) -> Result<Vec<f32>> {
        Err(VoiceError::TtsError(
            "macOS say not available on this platform".to_string(),
        ))
    }

    // ═══════════════════════════════════════════════════════════
    // Backend: Placeholder (last resort)
    // ═══════════════════════════════════════════════════════════

    /// Generate placeholder audio (harmonic tone) for testing / last resort.
    fn generate_placeholder_audio(&self, text: &str, profile: &VoiceProfile) -> Vec<f32> {
        // Duration proportional to text length
        let duration_secs = (text.len() as f32 * 0.06 / profile.speed).max(0.5);
        let num_samples = (self.config.sample_rate as f32 * duration_secs) as usize;

        let base_freq = 180.0 + profile.pitch_shift * 20.0;
        let sample_rate = self.config.sample_rate as f32;

        (0..num_samples)
            .map(|i| {
                let t = i as f32 / sample_rate;
                let fundamental = (2.0 * std::f32::consts::PI * base_freq * t).sin();
                let harmonic2 =
                    (2.0 * std::f32::consts::PI * base_freq * 2.0 * t).sin() * 0.3;
                let harmonic3 =
                    (2.0 * std::f32::consts::PI * base_freq * 3.0 * t).sin() * 0.1;
                let fade_in = (t * 20.0).min(1.0);
                let fade_out = ((duration_secs - t) * 20.0).min(1.0).max(0.0);
                let envelope = fade_in * fade_out;
                (fundamental + harmonic2 + harmonic3) * envelope * 0.3 * profile.energy
            })
            .collect()
    }

    /// Get the output sample rate.
    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate
    }

    /// Get the active backend name for logging.
    pub fn backend_name(&self) -> &str {
        match self.backend {
            TtsBackend::Blitz => "Blitz TTS Server",
            TtsBackend::Kokoro => "Kokoro TTS Server",
            TtsBackend::MacOsSay => "macOS say",
            TtsBackend::Placeholder => "Placeholder (sine wave)",
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Voice mapping: VoiceProfile → backend-specific voice identifier
// ═══════════════════════════════════════════════════════════════════════

/// Map a NPC voice profile to a Kokoro voice ID.
///
/// Kokoro voice IDs: af_heart, af_bella, af_nicole, af_sarah, af_sky,
/// am_adam, am_michael, bf_emma, bf_isabella, bm_george, bm_lewis
fn profile_to_kokoro_voice(profile: &VoiceProfile) -> &str {
    // The voice_id in VoiceProfile is already a Kokoro voice ID
    match profile.voice_id.as_str() {
        id @ ("af_heart" | "af_bella" | "af_nicole" | "af_sarah" | "af_sky" | "am_adam"
        | "am_michael" | "bf_emma" | "bf_isabella" | "bm_george" | "bm_lewis") => id,
        _ => "af_heart", // Default Kokoro voice
    }
}

/// Map a NPC voice profile to a Blitz TTS voice ID.
///
/// Blitz voices are currently exposed as F1-F5 and M1-M5. We map the existing
/// profile space onto the more expressive mid/high-tier Blitz presets.
fn profile_to_blitz_voice(profile: &VoiceProfile) -> &str {
    match profile.voice_id.as_str() {
        "af_heart" => "F4",
        "af_bella" => "F5",
        "af_nicole" => "F3",
        "af_sarah" => "F2",
        "af_sky" => "F1",
        "am_adam" => "M3",
        "am_michael" => "M4",
        "bf_emma" => "F2",
        "bf_isabella" => "F5",
        "bm_george" => "M4",
        "bm_lewis" => "M5",
        _ => "M4",
    }
}

/// Map a NPC voice profile to a macOS `say` voice name.
///
/// Kokoro voice IDs are mapped to approximate macOS equivalents:
/// - af_* (American female) → Samantha, Ava, Victoria, Allison, Susan
/// - am_* (American male) → Tom, Alex, Fred
/// - bf_* (British female) → Fiona, Serena
/// - bm_* (British male) → Daniel, Oliver
fn profile_to_macos_voice(profile: &VoiceProfile) -> String {
    match profile.voice_id.as_str() {
        // American female voices
        "af_heart" => "Samantha",
        "af_bella" => "Ava",
        "af_nicole" => "Victoria",
        "af_sarah" => "Allison",
        "af_sky" => "Susan",
        // American male voices
        "am_adam" => "Tom",
        "am_michael" => "Alex",
        // British female voices
        "bf_emma" => "Fiona",
        "bf_isabella" => "Serena",
        // British male voices
        "bm_george" => "Daniel",
        "bm_lewis" => "Oliver",
        // Fallback
        _ => "Samantha",
    }
    .to_string()
}

// ═══════════════════════════════════════════════════════════════════════
// WAV parsing
// ═══════════════════════════════════════════════════════════════════════

/// Parse WAV bytes to f32 PCM samples.
///
/// Handles both 16-bit int and 32-bit float WAV formats.
fn parse_wav_to_f32(wav_bytes: &[u8]) -> Result<Vec<f32>> {
    let cursor = std::io::Cursor::new(wav_bytes);
    let mut reader = hound::WavReader::new(cursor)
        .map_err(|e| VoiceError::TtsError(format!("Failed to parse WAV: {}", e)))?;

    let spec = reader.spec();

    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => {
            match spec.bits_per_sample {
                16 => reader
                    .samples::<i16>()
                    .filter_map(|s| s.ok())
                    .map(|s| s as f32 / i16::MAX as f32)
                    .collect(),
                24 => reader
                    .samples::<i32>()
                    .filter_map(|s| s.ok())
                    .map(|s| s as f32 / 8_388_607.0) // 2^23 - 1
                    .collect(),
                32 => reader
                    .samples::<i32>()
                    .filter_map(|s| s.ok())
                    .map(|s| s as f32 / i32::MAX as f32)
                    .collect(),
                bps => {
                    return Err(VoiceError::TtsError(format!(
                        "Unsupported bits per sample: {}",
                        bps
                    )));
                }
            }
        }
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .filter_map(|s| s.ok())
            .collect(),
    };

    if samples.is_empty() {
        return Err(VoiceError::TtsError("WAV file contained no audio".to_string()));
    }

    // If stereo, convert to mono by averaging channels
    let mut audio = if spec.channels == 2 {
        samples
            .chunks(2)
            .map(|frame| (frame[0] + frame.get(1).copied().unwrap_or(0.0)) * 0.5)
            .collect()
    } else {
        samples
    };

    condition_voice_audio(&mut audio, spec.sample_rate);

    Ok(audio)
}

fn condition_voice_audio(audio: &mut Vec<f32>, sample_rate: u32) {
    if audio.is_empty() {
        return;
    }

    trim_voice_silence(audio, sample_rate);
    apply_voice_edge_fade(audio, sample_rate);

    let (rms, peak) = voice_audio_stats(audio);
    if peak <= 0.0 || rms <= 0.0 {
        return;
    }

    let gain = (VOICE_AUDIO_TARGET_PEAK / peak)
        .min(VOICE_AUDIO_TARGET_RMS / rms)
        .clamp(1.0, VOICE_AUDIO_MAX_GAIN);

    if gain > 1.01 {
        for sample in audio.iter_mut() {
            *sample = (*sample * gain).clamp(-1.0, 1.0);
        }
    }
}

fn trim_voice_silence(audio: &mut Vec<f32>, sample_rate: u32) {
    if audio.is_empty() {
        return;
    }

    let (_, peak) = voice_audio_stats(audio);
    let threshold = (peak * VOICE_AUDIO_TRIM_PEAK_RATIO).max(VOICE_AUDIO_TRIM_FLOOR);
    let padding = ((sample_rate as f32) * 0.03) as usize;

    let Some(start) = audio.iter().position(|sample| sample.abs() >= threshold) else {
        return;
    };
    let Some(end) = audio.iter().rposition(|sample| sample.abs() >= threshold) else {
        return;
    };

    let start = start.saturating_sub(padding);
    let end = (end + padding + 1).min(audio.len());
    *audio = audio[start..end].to_vec();
}

fn apply_voice_edge_fade(audio: &mut [f32], sample_rate: u32) {
    if audio.is_empty() {
        return;
    }

    let fade_len = ((sample_rate as f32) * VOICE_AUDIO_EDGE_FADE_SECS) as usize;
    let fade_len = fade_len.min(audio.len() / 2);
    if fade_len == 0 {
        return;
    }

    for i in 0..fade_len {
        let gain = i as f32 / fade_len as f32;
        audio[i] *= gain;
        let end_idx = audio.len() - 1 - i;
        audio[end_idx] *= gain;
    }
}

fn voice_audio_stats(audio: &[f32]) -> (f32, f32) {
    if audio.is_empty() {
        return (0.0, 0.0);
    }

    let rms = (audio.iter().map(|sample| sample * sample).sum::<f32>() / audio.len() as f32).sqrt();
    let peak = audio.iter().fold(0.0f32, |acc, &sample| acc.max(sample.abs()));
    (rms, peak)
}

// ═══════════════════════════════════════════════════════════════════════
// Sentence splitting for streaming
// ═══════════════════════════════════════════════════════════════════════

/// Split text into sentences for streaming synthesis.
fn split_into_sentences(text: &str) -> Vec<&str> {
    let mut sentences = Vec::new();
    let mut start = 0;

    for (i, c) in text.char_indices() {
        if c == '.' || c == '!' || c == '?' {
            let end = i + c.len_utf8();
            let sentence = &text[start..end];
            if !sentence.trim().is_empty() {
                sentences.push(sentence.trim());
            }
            start = end;
        }
    }

    // Remaining text without sentence-ending punctuation
    if start < text.len() {
        let remaining = text[start..].trim();
        if !remaining.is_empty() {
            sentences.push(remaining);
        }
    }

    // If no sentences found, return whole text
    if sentences.is_empty() && !text.trim().is_empty() {
        sentences.push(text.trim());
    }

    sentences
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sentence_splitting() {
        let sentences = split_into_sentences("Hello traveler. What brings you here? I can help!");
        assert_eq!(sentences.len(), 3);
        assert_eq!(sentences[0], "Hello traveler.");
        assert_eq!(sentences[1], "What brings you here?");
        assert_eq!(sentences[2], "I can help!");
    }

    #[test]
    fn test_no_punctuation() {
        let sentences = split_into_sentences("Just a simple sentence");
        assert_eq!(sentences.len(), 1);
    }

    #[test]
    fn test_placeholder_audio() {
        let mut tts = TextToSpeech::with_defaults().unwrap();
        let profile = VoiceProfile::default();
        let audio = tts.synthesize("Hello world.", &profile).unwrap();
        // Should produce audio regardless of backend
        assert!(!audio.is_empty());
    }

    #[test]
    fn test_streaming_synthesis() {
        let mut tts = TextToSpeech::with_defaults().unwrap();
        let profile = VoiceProfile::default();

        let mut chunk_count = 0;
        tts.synthesize_streaming("Hello. How are you?", &profile, |_audio| {
            chunk_count += 1;
            true
        })
        .unwrap();

        assert_eq!(chunk_count, 2); // Two sentences
    }

    #[test]
    fn test_empty_text() {
        let mut tts = TextToSpeech::with_defaults().unwrap();
        let profile = VoiceProfile::default();
        let audio = tts.synthesize("", &profile).unwrap();
        assert!(audio.is_empty());
    }

    #[test]
    fn test_kokoro_voice_mapping() {
        let guard_profile = VoiceProfile::from_npc("guard", 0.4, 0.3);
        let voice = profile_to_kokoro_voice(&guard_profile);
        assert!(!voice.is_empty());
    }

    #[test]
    fn test_blitz_voice_mapping() {
        let guard_profile = VoiceProfile::from_npc("guard", 0.4, 0.3);
        let voice = profile_to_blitz_voice(&guard_profile);
        assert!(!voice.is_empty());
    }

    #[test]
    fn test_macos_voice_mapping() {
        let merchant_profile = VoiceProfile::from_npc("merchant", 0.8, 0.4);
        let voice = profile_to_macos_voice(&merchant_profile);
        assert!(!voice.is_empty());
    }

    #[test]
    fn test_wav_parse_placeholder() {
        // Test WAV parsing with a minimal WAV header
        let mut wav_bytes = Vec::new();
        // RIFF header
        wav_bytes.extend_from_slice(b"RIFF");
        wav_bytes.extend_from_slice(&(36u32 + 8u32).to_le_bytes()); // file size - 8
        wav_bytes.extend_from_slice(b"WAVE");
        // fmt chunk
        wav_bytes.extend_from_slice(b"fmt ");
        wav_bytes.extend_from_slice(&16u32.to_le_bytes()); // chunk size
        wav_bytes.extend_from_slice(&1u16.to_le_bytes());  // PCM
        wav_bytes.extend_from_slice(&1u16.to_le_bytes());  // mono
        wav_bytes.extend_from_slice(&24000u32.to_le_bytes()); // sample rate
        wav_bytes.extend_from_slice(&48000u32.to_le_bytes()); // byte rate
        wav_bytes.extend_from_slice(&2u16.to_le_bytes());  // block align
        wav_bytes.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
        // data chunk with a few samples
        let samples: Vec<i16> = vec![0, 100, 200, -100, -200, 0];
        let data_size = (samples.len() * 2) as u32;
        wav_bytes.extend_from_slice(b"data");
        wav_bytes.extend_from_slice(&data_size.to_le_bytes());
        for s in &samples {
            wav_bytes.extend_from_slice(&s.to_le_bytes());
        }

        let result = parse_wav_to_f32(&wav_bytes);
        assert!(result.is_ok());
        let audio = result.unwrap();
        assert_eq!(audio.len(), samples.len());
    }

    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "requires macOS say command and produces real audio output"]
    fn test_macos_say_synthesis() {
        let mut tts = TextToSpeech::new(TtsConfig {
            blitz_server_url: String::new(),
            kokoro_server_url: String::new(), // Force fallback to say
            allow_macos_say_fallback: true,
            ..TtsConfig::default()
        })
        .unwrap();
        let profile = VoiceProfile::from_npc("guard", 0.5, 0.3);
        let audio = tts.synthesize("Hello, traveler. What brings you here?", &profile);
        assert!(audio.is_ok());
        let audio = audio.unwrap();
        // Real speech should be substantially longer than placeholder
        assert!(audio.len() > 10000, "Expected real audio, got {} samples", audio.len());
    }
}
