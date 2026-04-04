//! SOTA Voice Dialogue Pipeline for NPC Conversations.
//!
//! Orchestrates the full voice pipeline: microphone capture → VAD →
//! STT → LLM → TTS → audio output. All processing runs on dedicated
//! threads to never block the game's main thread.
//!
//! # Architecture
//!
//! ```text
//! Game Thread                     Voice Thread
//! ──────────                     ────────────
//! VoiceCommand ──(channel)──→    Pipeline processes:
//!                                  1. Start/stop recording
//!                                  2. VAD → STT → LLM → TTS
//! VoiceEvent ←──(channel)───     Returns:
//!                                  - State changes
//!                                  - Transcriptions
//!                                  - Response text + audio
//! ```

pub mod conversation;
pub mod error;
pub mod llm;
pub mod resampler;
pub mod stt;
pub mod tts;
pub mod vad;
pub mod voice_profile;

use conversation::ConversationRegistry;
use crossbeam_channel::{Receiver, Sender};
use error::{Result, VoiceError};
use llm::{DialogueLlm, NpcContext};
use stt::SpeechToText;
use tts::TextToSpeech;
use voice_profile::VoiceProfile;

// Re-exports for external consumers
pub use conversation::Exchange;
pub use llm::{LlmConfig, NpcContext as NpcDialogueContext};
pub use stt::SttConfig;
pub use tts::TtsConfig;
pub use voice_profile::VoiceProfile as NpcVoiceProfile;

/// Current state of the voice dialogue pipeline.
#[derive(Debug, Clone, PartialEq)]
pub enum VoiceState {
    /// No active voice interaction.
    Idle,
    /// Microphone is capturing audio (player is speaking).
    Listening,
    /// Processing recorded audio through STT.
    Transcribing,
    /// LLM is generating the NPC response.
    Thinking,
    /// TTS is synthesizing NPC voice audio.
    Synthesizing,
    /// NPC voice audio is playing back.
    Speaking,
    /// An error occurred (contains description).
    Error(String),
}

/// Commands sent from the game thread to the voice pipeline.
#[derive(Debug)]
pub enum VoiceCommand {
    /// Start recording (player pressed PTT key while near an NPC).
    StartRecording {
        npc_id: u64,
        npc_context: NpcContext,
        voice_profile: VoiceProfile,
    },
    /// Stop recording (player released PTT key).
    StopRecording,
    /// Cancel the current interaction entirely.
    Cancel,
    /// Shut down the voice pipeline.
    Shutdown,
}

/// Events sent from the voice pipeline back to the game thread.
#[derive(Debug, Clone)]
pub enum VoiceEvent {
    /// Pipeline state changed.
    StateChanged(VoiceState),
    /// Player's speech was transcribed.
    Transcription(String),
    /// NPC response text is ready (for dialogue box).
    ResponseText(String),
    /// NPC audio chunk is ready (PCM samples, 24kHz mono).
    AudioChunk(Vec<f32>),
    /// All audio chunks for this response have been sent.
    AudioComplete,
    /// An error occurred.
    Error(String),
}

/// Configuration for the complete voice pipeline.
#[derive(Debug, Clone)]
pub struct VoicePipelineConfig {
    /// STT engine configuration.
    pub stt: SttConfig,
    /// TTS engine configuration.
    pub tts: TtsConfig,
    /// LLM engine configuration.
    pub llm: LlmConfig,
    /// Whether to let VAD emit partial transcriptions while the mic is still open.
    ///
    /// This should stay disabled for push-to-talk gameplay because any NPC speech
    /// emitted before the player releases the key can bleed back into the same
    /// recording and create duplicate/jumbled replies.
    pub allow_live_partial_responses: bool,
    /// Whether to emit the synthesized reply as multiple sentence-level audio chunks.
    ///
    /// Disabled by default for in-game playback because the client currently plays
    /// chunks immediately, which can sound like overlapping/echoing speech.
    pub stream_response_audio: bool,
}

impl Default for VoicePipelineConfig {
    fn default() -> Self {
        Self {
            stt: SttConfig::default(),
            tts: TtsConfig::default(),
            llm: LlmConfig::default(),
            allow_live_partial_responses: false,
            stream_response_audio: false,
        }
    }
}

/// The main voice dialogue pipeline.
///
/// This is the primary interface for the game engine. It uses channels for
/// bidirectional communication between the game thread and voice processing thread.
pub struct VoicePipeline {
    /// Send commands to the voice thread.
    command_tx: Sender<VoiceCommand>,
    /// Receive events from the voice thread.
    event_rx: Receiver<VoiceEvent>,
    /// Current state (mirrors what the voice thread reports).
    current_state: VoiceState,
    /// Handle to the processing thread.
    _thread: Option<std::thread::JoinHandle<()>>,
}

impl VoicePipeline {
    /// Create and start a new voice pipeline.
    ///
    /// This spawns a dedicated processing thread that handles all
    /// voice I/O and AI inference, keeping the game thread free.
    pub fn new(config: VoicePipelineConfig) -> Result<Self> {
        let (command_tx, command_rx) = crossbeam_channel::unbounded();
        let (event_tx, event_rx) = crossbeam_channel::unbounded();

        let thread = std::thread::Builder::new()
            .name("voice-pipeline".to_string())
            .spawn(move || {
                if let Err(e) = run_pipeline_thread(config, command_rx, event_tx.clone()) {
                    log::error!("Voice pipeline thread error: {}", e);
                    let _ = event_tx.send(VoiceEvent::Error(e.to_string()));
                }
            })
            .map_err(|e| VoiceError::PipelineError(format!("Failed to spawn thread: {}", e)))?;

        Ok(Self {
            command_tx,
            event_rx,
            current_state: VoiceState::Idle,
            _thread: Some(thread),
        })
    }

    /// Create a pipeline with default config (for testing/development).
    pub fn with_defaults() -> Result<Self> {
        Self::new(VoicePipelineConfig::default())
    }

    /// Start recording (player pressed PTT near an NPC).
    pub fn start_recording(
        &mut self,
        npc_id: u64,
        npc_context: NpcContext,
        voice_profile: VoiceProfile,
    ) -> Result<()> {
        self.command_tx
            .send(VoiceCommand::StartRecording {
                npc_id,
                npc_context,
                voice_profile,
            })
            .map_err(|e| VoiceError::ChannelError(e.to_string()))
    }

    /// Stop recording (player released PTT).
    pub fn stop_recording(&mut self) -> Result<()> {
        self.command_tx
            .send(VoiceCommand::StopRecording)
            .map_err(|e| VoiceError::ChannelError(e.to_string()))
    }

    /// Cancel the current interaction.
    pub fn cancel(&mut self) -> Result<()> {
        self.command_tx
            .send(VoiceCommand::Cancel)
            .map_err(|e| VoiceError::ChannelError(e.to_string()))
    }

    /// Poll for events from the voice pipeline.
    ///
    /// Call this from the game's update loop (every frame) to receive
    /// state changes, transcriptions, response text, and audio chunks.
    /// Returns `None` if no events are pending.
    pub fn poll_event(&mut self) -> Option<VoiceEvent> {
        match self.event_rx.try_recv() {
            Ok(event) => {
                // Update local state mirror
                if let VoiceEvent::StateChanged(ref state) = event {
                    self.current_state = state.clone();
                }
                Some(event)
            }
            Err(_) => None,
        }
    }

    /// Drain all pending events.
    ///
    /// Useful when you want to process all events in a single frame.
    pub fn drain_events(&mut self) -> Vec<VoiceEvent> {
        let mut events = Vec::new();
        while let Some(event) = self.poll_event() {
            events.push(event);
        }
        events
    }

    /// Get the current pipeline state.
    pub fn state(&self) -> &VoiceState {
        &self.current_state
    }

    /// Check if an interaction is in progress.
    pub fn is_active(&self) -> bool {
        self.current_state != VoiceState::Idle
    }
}

impl Drop for VoicePipeline {
    fn drop(&mut self) {
        let _ = self.command_tx.send(VoiceCommand::Shutdown);
    }
}

/// Main loop for the voice processing thread.
fn run_pipeline_thread(
    config: VoicePipelineConfig,
    command_rx: Receiver<VoiceCommand>,
    event_tx: Sender<VoiceEvent>,
) -> Result<()> {
    log::info!("Voice pipeline thread started");

    // Initialize engines
    let mut stt = match SpeechToText::new(config.stt) {
        Ok(s) => s,
        Err(e) => {
            log::error!("STT initialization failed: {}", e);
            let _ = event_tx.send(VoiceEvent::Error(format!("Voice init failed: {}", e)));
            let _ = event_tx.send(VoiceEvent::StateChanged(VoiceState::Idle));
            return Err(e);
        }
    };
    let mut tts = TextToSpeech::new(config.tts)?;
    let mut llm = DialogueLlm::new(config.llm)?;
    let mut conversations = ConversationRegistry::default();

    let mut current_npc_id: Option<u64> = None;
    let mut current_context: Option<NpcContext> = None;
    let mut current_voice: Option<VoiceProfile> = None;

    loop {
        // Wait for next command, or timeout to poll STT
        let command = if stt.is_recording() {
            match command_rx.recv_timeout(std::time::Duration::from_millis(50)) {
                Ok(cmd) => Some(cmd),
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => None,
                Err(_) => {
                    log::info!("Command channel closed, shutting down");
                    break;
                }
            }
        } else {
            match command_rx.recv() {
                Ok(cmd) => Some(cmd),
                Err(_) => {
                    log::info!("Command channel closed, shutting down");
                    break;
                }
            }
        };

        if let Some(cmd) = command {
            match cmd {
                VoiceCommand::StartRecording {
                    npc_id,
                    npc_context,
                    voice_profile,
                } => {
                    current_npc_id = Some(npc_id);
                    current_context = Some(npc_context);
                    current_voice = Some(voice_profile);

                    let _ = event_tx.send(VoiceEvent::StateChanged(VoiceState::Listening));

                    if let Err(e) = stt.start_recording() {
                        log::error!("Failed to start recording: {}", e);
                        let _ = event_tx.send(VoiceEvent::Error(e.to_string()));
                        let _ = event_tx.send(VoiceEvent::StateChanged(VoiceState::Idle));
                    }
                }

                VoiceCommand::StopRecording => {
                    let _ = event_tx.send(VoiceEvent::StateChanged(VoiceState::Transcribing));

                    // STT: Transcribe the final recorded audio
                    match stt.stop_and_transcribe() {
                        Ok(text) if text.is_empty() => {
                            // Mic is silent or permission denied — tell the user clearly
                            log::warn!("No speech detected — mic may be silent (AirPods A2DP mode?) or permission denied");
                            let _ = event_tx.send(VoiceEvent::Error(
                                "mic_silent: Couldn't hear you. On macOS, check System Settings → Privacy & Security → Microphone. \
                                 If using AirPods, try speaking again or switch to Built-in Microphone.".to_string()
                            ));
                            let _ = event_tx.send(VoiceEvent::StateChanged(VoiceState::Idle));
                        }
                        Ok(text) => {
                            let _ = event_tx.send(VoiceEvent::Transcription(text.clone()));
                            let _ = event_tx.send(VoiceEvent::StateChanged(VoiceState::Thinking));

                            if let (Some(ref npc_ctx), Some(npc_id)) =
                                (&current_context, current_npc_id)
                            {
                                let history = conversations.get(npc_id);
                                let voice = current_voice.as_ref().cloned().unwrap_or_default();

                                match llm.generate_response(npc_ctx, &text, history) {
                                    Ok(response) => {
                                        conversations
                                            .get_or_create(npc_id)
                                            .add_exchange(text.clone(), response.clone());
                                        let _ = event_tx.send(VoiceEvent::ResponseText(response.clone()));
                                        emit_response_audio(
                                            &mut tts,
                                            &event_tx,
                                            &response,
                                            &voice,
                                            config.stream_response_audio,
                                        );
                                    }
                                    Err(e) => {
                                        log::error!("LLM generation failed: {}", e);
                                        let _ = event_tx.send(VoiceEvent::Error(e.to_string()));
                                    }
                                }
                            }

                            let _ = event_tx.send(VoiceEvent::StateChanged(VoiceState::Idle));
                        }
                        Err(e) => {
                            log::error!("Transcription failed: {}", e);
                            let _ = event_tx.send(VoiceEvent::Error(e.to_string()));
                            let _ = event_tx.send(VoiceEvent::StateChanged(VoiceState::Idle));
                        }
                    }
                }

                VoiceCommand::Cancel => {
                    log::info!("Voice interaction cancelled");
                    if stt.is_recording() {
                        let _ = stt.stop_and_transcribe(); // Discard result
                    }
                    current_npc_id = None;
                    current_context = None;
                    current_voice = None;
                    let _ = event_tx.send(VoiceEvent::StateChanged(VoiceState::Idle));
                }

                VoiceCommand::Shutdown => {
                    log::info!("Voice pipeline shutting down");
                    break;
                }
            }
        }

        // Optional live-response mode for continuous listening. This stays off for
        // push-to-talk gameplay because emitting NPC speech before the player has
        // released the key causes acoustic feedback and duplicate replies.
        if config.allow_live_partial_responses && stt.is_recording() {
            match stt.try_transcribe_partial() {
                Ok(Some(text)) => {
                    log::info!("Partial transcription detected pause: {}", text);
                    let _ = event_tx.send(VoiceEvent::Transcription(text.clone()));
                    let _ = event_tx.send(VoiceEvent::StateChanged(VoiceState::Thinking));

                    if let (Some(ref npc_ctx), Some(npc_id)) = (&current_context, current_npc_id) {
                        let history = conversations.get(npc_id);

                        match llm.generate_response(npc_ctx, &text, history) {
                            Ok(response) => {
                                conversations
                                    .get_or_create(npc_id)
                                    .add_exchange(text, response.clone());

                                let _ = event_tx.send(VoiceEvent::ResponseText(response.clone()));
                                let voice = current_voice.as_ref().cloned().unwrap_or_default();
                                emit_response_audio(
                                    &mut tts,
                                    &event_tx,
                                    &response,
                                    &voice,
                                    config.stream_response_audio,
                                );
                            }
                            Err(e) => {
                                log::error!("LLM generation failed: {}", e);
                                let _ = event_tx.send(VoiceEvent::Error(e.to_string()));
                            }
                        }
                    }

                    // Go back to listening if STT is still recording
                    if stt.is_recording() {
                        let _ = event_tx.send(VoiceEvent::StateChanged(VoiceState::Listening));
                    }
                }
                Ok(None) => {}
                Err(e) => {
                    log::error!("Partial transcription failed: {}", e);
                }
            }
        }
    }

    log::info!("Voice pipeline thread exited");
    Ok(())
}

fn emit_response_audio(
    tts: &mut TextToSpeech,
    event_tx: &Sender<VoiceEvent>,
    response: &str,
    voice: &VoiceProfile,
    stream_response_audio: bool,
) {
    let _ = event_tx.send(VoiceEvent::StateChanged(VoiceState::Synthesizing));

    let result = if stream_response_audio {
        let event_tx_clone = event_tx.clone();
        tts.synthesize_streaming(response, voice, |audio_chunk| {
            let _ = event_tx_clone.send(VoiceEvent::StateChanged(VoiceState::Speaking));
            let _ = event_tx_clone.send(VoiceEvent::AudioChunk(audio_chunk));
            true
        })
    } else {
        match tts.synthesize(response, voice) {
            Ok(audio) => {
                let _ = event_tx.send(VoiceEvent::StateChanged(VoiceState::Speaking));
                let _ = event_tx.send(VoiceEvent::AudioChunk(audio));
                Ok(())
            }
            Err(e) => Err(e),
        }
    };

    match result {
        Ok(()) => {
            let _ = event_tx.send(VoiceEvent::AudioComplete);
        }
        Err(e) => {
            log::error!("TTS synthesis failed: {}", e);
            let _ = event_tx.send(VoiceEvent::Error(e.to_string()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_state() {
        assert_ne!(VoiceState::Idle, VoiceState::Listening);
        assert_eq!(VoiceState::Idle, VoiceState::Idle);
    }

    #[test]
    fn test_npc_context_prompt() {
        let ctx = NpcContext {
            name: "Aria".to_string(),
            profession: "Herbalist".to_string(),
            location: "Greenwood".to_string(),
            faction: "Nature's Keepers".to_string(),
            personality: "Gentle, curious, empathetic".to_string(),
            mood: "peaceful".to_string(),
            knowledge: "Knows about local herbs and remedies.".to_string(),
            player_sentiment: "friendly acquaintance".to_string(),
        };

        let prompt = ctx.to_system_prompt();
        assert!(prompt.contains("Aria"));
        assert!(prompt.contains("Herbalist"));
        assert!(prompt.contains("Greenwood"));
        assert!(prompt.contains("Nature's Keepers"));
    }

    #[test]
    fn test_conversation_flow() {
        let mut registry = ConversationRegistry::default();
        let history = registry.get_or_create(42);

        history.add_exchange("Hello!".into(), "Greetings, traveler.".into());
        history.add_exchange("What's your name?".into(), "I am Thorin.".into());

        let ctx = history.to_context_string();
        assert!(ctx.contains("Hello!"));
        assert!(ctx.contains("Thorin"));
    }
}
