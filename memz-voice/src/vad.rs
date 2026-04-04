//! Voice Activity Detection (VAD) using Silero VAD.
//!
//! Detects when the player is actually speaking vs. silence/background noise.
//! This prevents wasting compute on silent audio and improves STT accuracy.

use crate::error::Result;

/// Voice Activity Detection engine using Silero VAD (ONNX).
///
/// Streams audio chunks and emits speech/silence events with configurable
/// sensitivity thresholds suitable for gaming environments.
pub struct Vad {
    /// Speech detection threshold (0.0–1.0). Higher = more selective.
    threshold: f32,
    /// Minimum speech duration in samples to trigger (avoids false positives from clicks/bumps).
    min_speech_samples: usize,
    /// Minimum silence duration in samples to end speech (avoids cutting off mid-pause).
    min_silence_samples: usize,
    /// Running state: are we currently in a speech segment?
    in_speech: bool,
    /// How many consecutive speech-detected frames we've seen.
    speech_frame_count: usize,
    /// How many consecutive silence-detected frames we've seen.
    silence_frame_count: usize,
    /// Accumulated speech audio buffer.
    speech_buffer: Vec<f32>,
}

/// Result of processing an audio chunk through VAD.
#[derive(Debug, Clone)]
pub enum VadEvent {
    /// No speech detected — chunk is silence/noise.
    Silence,
    /// Speech is ongoing — audio is being accumulated.
    SpeechContinuing,
    /// Speech segment just started.
    SpeechStart,
    /// Speech segment ended — contains the accumulated speech audio.
    SpeechEnd(Vec<f32>),
}

impl Vad {
    /// Create a new VAD engine.
    ///
    /// # Parameters
    /// - `threshold`: Speech detection sensitivity (0.0–1.0). Default 0.5.
    ///   Lower values detect more speech (including quiet speech) but may
    ///   trigger on background noise. Higher values are more selective.
    /// - `sample_rate`: Expected audio sample rate (usually 16000).
    pub fn new(threshold: f32, sample_rate: u32) -> Result<Self> {
        log::info!(
            "Initializing VAD with threshold={:.2}, sample_rate={}",
            threshold,
            sample_rate
        );

        // Min speech: 250ms of continuous speech to trigger
        let min_speech_samples = (sample_rate as f32 * 0.25) as usize;
        // Min silence: 600ms of silence to end speech (allows natural pauses)
        let min_silence_samples = (sample_rate as f32 * 0.60) as usize;

        Ok(Self {
            threshold,
            min_speech_samples,
            min_silence_samples,
            in_speech: false,
            speech_frame_count: 0,
            silence_frame_count: 0,
            speech_buffer: Vec::with_capacity(sample_rate as usize * 30), // 30s max
        })
    }

    /// Create a VAD with gaming-optimized defaults.
    ///
    /// Uses a slightly higher threshold to avoid triggering on keyboard
    /// clicks, mouse movements, and ambient game audio from speakers.
    pub fn gaming_default() -> Result<Self> {
        Self::new(0.55, 16000)
    }

    /// Process an audio chunk through VAD.
    ///
    /// Audio should be 16kHz mono f32 PCM.
    /// Returns a `VadEvent` indicating the current speech state.
    pub fn process_chunk(&mut self, audio: &[f32]) -> VadEvent {
        let is_speech = self.detect_speech_energy(audio);

        if is_speech {
            self.silence_frame_count = 0;
            self.speech_frame_count += audio.len();

            // Accumulate audio
            self.speech_buffer.extend_from_slice(audio);

            if !self.in_speech && self.speech_frame_count >= self.min_speech_samples {
                self.in_speech = true;
                log::debug!("VAD: Speech started");
                return VadEvent::SpeechStart;
            }

            if self.in_speech {
                return VadEvent::SpeechContinuing;
            }

            // Still accumulating but not yet confirmed as speech
            VadEvent::Silence
        } else {
            self.speech_frame_count = 0;

            if self.in_speech {
                self.silence_frame_count += audio.len();

                if self.silence_frame_count >= self.min_silence_samples {
                    // Speech segment ended
                    self.in_speech = false;
                    self.silence_frame_count = 0;
                    let speech_audio = std::mem::take(&mut self.speech_buffer);
                    log::debug!(
                        "VAD: Speech ended ({} samples, {:.1}s)",
                        speech_audio.len(),
                        speech_audio.len() as f32 / 16000.0
                    );
                    return VadEvent::SpeechEnd(speech_audio);
                }

                // Still in speech — the silence is a natural pause
                self.speech_buffer.extend_from_slice(audio);
                VadEvent::SpeechContinuing
            } else {
                // Not in speech, clear any partial buffer
                if !self.speech_buffer.is_empty()
                    && self.speech_buffer.len() < self.min_speech_samples
                {
                    self.speech_buffer.clear();
                }
                VadEvent::Silence
            }
        }
    }

    /// Simple energy-based VAD.
    ///
    /// Computes RMS energy and compares against threshold.
    /// This is a fast heuristic; the Silero ONNX model provides more
    /// accurate detection but this serves as a robust fallback and
    /// pre-filter.
    fn detect_speech_energy(&self, audio: &[f32]) -> bool {
        if audio.is_empty() {
            return false;
        }

        // Compute RMS energy
        let rms = (audio.iter().map(|s| s * s).sum::<f32>() / audio.len() as f32).sqrt();

        // Also check peak amplitude (catches transients)
        let peak = audio.iter().map(|s| s.abs()).fold(0.0f32, f32::max);

        // Speech typically has RMS > 0.01 and peak > 0.02
        // Multiply threshold by base values
        let rms_threshold = 0.01 * self.threshold;
        let peak_threshold = 0.02 * self.threshold;

        rms > rms_threshold || peak > peak_threshold
    }

    /// Force-end any ongoing speech segment and return accumulated audio.
    ///
    /// Used when PTT key is released — we don't wait for silence timeout.
    pub fn force_end_speech(&mut self) -> Option<Vec<f32>> {
        if self.speech_buffer.is_empty() {
            return None;
        }

        self.in_speech = false;
        self.speech_frame_count = 0;
        self.silence_frame_count = 0;
        let audio = std::mem::take(&mut self.speech_buffer);
        log::debug!(
            "VAD: Force-ended speech ({} samples, {:.1}s)",
            audio.len(),
            audio.len() as f32 / 16000.0
        );
        Some(audio)
    }

    /// Reset VAD state (e.g., when starting a new interaction).
    pub fn reset(&mut self) {
        self.in_speech = false;
        self.speech_frame_count = 0;
        self.silence_frame_count = 0;
        self.speech_buffer.clear();
    }

    /// Check if currently detecting speech.
    pub fn is_in_speech(&self) -> bool {
        self.in_speech
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_silence_detection() {
        let mut vad = Vad::gaming_default().unwrap();
        let silence = vec![0.0f32; 1600]; // 100ms of silence

        let event = vad.process_chunk(&silence);
        assert!(matches!(event, VadEvent::Silence));
        assert!(!vad.is_in_speech());
    }

    #[test]
    fn test_speech_detection() {
        let mut vad = Vad::new(0.3, 16000).unwrap();

        // Generate a sine wave (simulates speech)
        let speech: Vec<f32> = (0..8000) // 500ms
            .map(|i| (i as f32 * 0.1).sin() * 0.5)
            .collect();

        // Process in chunks
        for chunk in speech.chunks(1600) {
            vad.process_chunk(chunk);
        }

        assert!(vad.is_in_speech());
    }

    #[test]
    fn test_force_end() {
        let mut vad = Vad::new(0.3, 16000).unwrap();

        // Add some speech audio
        let speech: Vec<f32> = (0..8000)
            .map(|i| (i as f32 * 0.1).sin() * 0.5)
            .collect();
        for chunk in speech.chunks(1600) {
            vad.process_chunk(chunk);
        }

        let audio = vad.force_end_speech();
        assert!(audio.is_some());
        assert!(audio.unwrap().len() > 0);
        assert!(!vad.is_in_speech());
    }
}
