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
use std::collections::BTreeMap;

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
    /// A low-latency preview of the NPC response text while the model is still generating.
    ResponsePreview(String),
    /// A spoken NPC segment that should drive both subtitle timing and audio playback.
    SpokenSegment(SpokenAudioSegment),
    /// All audio chunks for this response have been sent.
    AudioComplete,
    /// An error occurred.
    Error(String),
}

/// A spoken NPC response segment aligned to a concrete piece of generated audio.
#[derive(Debug, Clone)]
pub struct SpokenAudioSegment {
    /// Monotonic segment sequence within a single NPC response.
    pub sequence: u32,
    /// Text that corresponds to this spoken audio segment.
    pub text: String,
    /// PCM samples at 24kHz mono.
    pub audio: Vec<f32>,
    /// Duration of the audio in seconds.
    pub duration_secs: f32,
}

#[derive(Debug)]
enum TtsWorkerCommand {
    Synthesize {
        generation_id: u64,
        sequence: u32,
        text: String,
        voice: VoiceProfile,
    },
}

#[derive(Debug)]
enum TtsWorkerResult {
    SegmentReady {
        generation_id: u64,
        sequence: u32,
        text: String,
        audio: Vec<f32>,
        duration_secs: f32,
    },
    SegmentFailed {
        generation_id: u64,
        sequence: u32,
        error: String,
    },
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
        if self.current_state != VoiceState::Idle {
            return Ok(());
        }

        self.command_tx
            .send(VoiceCommand::StartRecording {
                npc_id,
                npc_context,
                voice_profile,
            })
            .map_err(|e| VoiceError::ChannelError(e.to_string()))?;
        self.current_state = VoiceState::Listening;
        Ok(())
    }

    /// Stop recording (player released PTT).
    pub fn stop_recording(&mut self) -> Result<()> {
        if self.current_state != VoiceState::Listening {
            return Ok(());
        }

        self.command_tx
            .send(VoiceCommand::StopRecording)
            .map_err(|e| VoiceError::ChannelError(e.to_string()))?;
        self.current_state = VoiceState::Transcribing;
        Ok(())
    }

    /// Cancel the current interaction.
    pub fn cancel(&mut self) -> Result<()> {
        self.command_tx
            .send(VoiceCommand::Cancel)
            .map_err(|e| VoiceError::ChannelError(e.to_string()))?;
        self.current_state = VoiceState::Idle;
        Ok(())
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
    let tts = TextToSpeech::new(config.tts.clone())?;
    let mut llm = DialogueLlm::new(config.llm)?;
    let mut conversations = ConversationRegistry::default();
    let (tts_request_tx, tts_request_rx) = crossbeam_channel::unbounded();
    let (tts_result_tx, tts_result_rx) = crossbeam_channel::unbounded();
    let _tts_thread = std::thread::Builder::new()
        .name("voice-tts-worker".to_string())
        .spawn(move || run_tts_worker_loop(tts, tts_request_rx, tts_result_tx))
        .map_err(|e| VoiceError::PipelineError(format!("Failed to spawn TTS worker: {}", e)))?;

    let mut current_npc_id: Option<u64> = None;
    let mut current_context: Option<NpcContext> = None;
    let mut current_voice: Option<VoiceProfile> = None;
    let mut response_generation: u64 = 0;

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
                    if !stt.is_recording() {
                        log::warn!(
                            "Ignoring stop-recording request because the microphone is not recording"
                        );
                        let _ = event_tx.send(VoiceEvent::StateChanged(VoiceState::Idle));
                        continue;
                    }

                    let _ = event_tx.send(VoiceEvent::StateChanged(VoiceState::Transcribing));

                    // STT: Transcribe the final recorded audio
                    match stt.stop_and_transcribe() {
                        Ok(text) if text.is_empty() => {
                            // We received no usable transcript. This may be brief speech,
                            // quiet capture, unclear pronunciation, or a genuine mic issue.
                            log::warn!(
                                "No clear speech could be transcribed from the captured audio"
                            );
                            let _ = event_tx.send(VoiceEvent::Error(
                                "speech_unclear: I couldn't make out clear speech. Hold V while speaking, then release. \
                                 If this keeps happening, check microphone privacy and raise the input volume.".to_string()
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
                                response_generation = response_generation.wrapping_add(1);

                                match generate_response_and_audio(
                                    &mut llm,
                                    &event_tx,
                                    &tts_request_tx,
                                    &tts_result_rx,
                                    response_generation,
                                    npc_ctx,
                                    &text,
                                    history,
                                    &voice,
                                    config.stream_response_audio,
                                ) {
                                    Ok(response) => {
                                        conversations
                                            .get_or_create(npc_id)
                                            .add_exchange(text.clone(), response.clone());
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

                        let voice = current_voice.as_ref().cloned().unwrap_or_default();
                        response_generation = response_generation.wrapping_add(1);

                        match generate_response_and_audio(
                            &mut llm,
                            &event_tx,
                            &tts_request_tx,
                            &tts_result_rx,
                            response_generation,
                            npc_ctx,
                            &text,
                            history,
                            &voice,
                            config.stream_response_audio,
                        ) {
                            Ok(response) => {
                                conversations
                                    .get_or_create(npc_id)
                                    .add_exchange(text, response.clone());
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
    event_tx: &Sender<VoiceEvent>,
    tts_request_tx: &Sender<TtsWorkerCommand>,
    tts_result_rx: &Receiver<TtsWorkerResult>,
    generation_id: u64,
    response: &str,
    voice: &VoiceProfile,
) {
    let sequence = 0;
    if let Err(err) = queue_tts_segment(
        tts_request_tx,
        generation_id,
        sequence,
        response.to_string(),
        voice.clone(),
        event_tx,
    ) {
        log::error!("TTS synthesis failed: {}", err);
        let _ = event_tx.send(VoiceEvent::Error(err.to_string()));
        return;
    }

    let mut pending_segments = BTreeMap::new();
    let mut next_sequence_to_emit = 0;
    match wait_for_tts_results(
        tts_result_rx,
        generation_id,
        1,
        &mut pending_segments,
        &mut next_sequence_to_emit,
        event_tx,
    ) {
        Ok(()) => {
            let _ = event_tx.send(VoiceEvent::AudioComplete);
        }
        Err(e) => {
            log::error!("TTS synthesis failed: {}", e);
            let _ = event_tx.send(VoiceEvent::Error(e.to_string()));
        }
    }
}

fn generate_response_and_audio(
    llm: &mut DialogueLlm,
    event_tx: &Sender<VoiceEvent>,
    tts_request_tx: &Sender<TtsWorkerCommand>,
    tts_result_rx: &Receiver<TtsWorkerResult>,
    generation_id: u64,
    npc_context: &NpcContext,
    player_text: &str,
    history: Option<&conversation::ConversationHistory>,
    voice: &VoiceProfile,
    stream_response_audio: bool,
) -> Result<String> {
    if stream_response_audio {
        generate_streamed_response_and_audio(
            llm,
            event_tx,
            tts_request_tx,
            tts_result_rx,
            generation_id,
            npc_context,
            player_text,
            history,
            voice,
        )
    } else {
        let response = llm.generate_response(npc_context, player_text, history)?;
        let _ = event_tx.send(VoiceEvent::ResponsePreview(response.clone()));
        emit_response_audio(
            event_tx,
            tts_request_tx,
            tts_result_rx,
            generation_id,
            &response,
            voice,
        );
        Ok(response)
    }
}

fn generate_streamed_response_and_audio(
    llm: &mut DialogueLlm,
    event_tx: &Sender<VoiceEvent>,
    tts_request_tx: &Sender<TtsWorkerCommand>,
    tts_result_rx: &Receiver<TtsWorkerResult>,
    generation_id: u64,
    npc_context: &NpcContext,
    player_text: &str,
    history: Option<&conversation::ConversationHistory>,
    voice: &VoiceProfile,
) -> Result<String> {
    let mut emitted_bytes = 0usize;
    let mut last_emitted_text = String::new();
    let mut tts_error: Option<VoiceError> = None;
    let mut pending_segments = BTreeMap::new();
    let mut next_sequence_to_queue = 0u32;
    let mut next_sequence_to_emit = 0u32;
    let mut outstanding_segments = 0usize;

    let response = llm.generate_response_streaming(npc_context, player_text, history, |partial| {
        let partial = partial.trim();
        if partial.is_empty() {
            return true;
        }

        let Some(new_text) = partial.get(emitted_bytes..) else {
            return true;
        };
        let (ready_segments, consumed_bytes) = drain_ready_segments(new_text);
        if last_emitted_text != partial {
            let _ = event_tx.send(VoiceEvent::ResponsePreview(partial.to_string()));
            last_emitted_text.clear();
            last_emitted_text.push_str(partial);
        }

        if ready_segments.is_empty() {
            if let Err(err) = drain_ready_tts_results(
                tts_result_rx,
                generation_id,
                &mut pending_segments,
                &mut next_sequence_to_emit,
                event_tx,
            ) {
                tts_error = Some(err);
                return false;
            }
            return true;
        }

        emitted_bytes += consumed_bytes;

        for segment_text in ready_segments {
            if let Err(err) = queue_tts_segment(
                tts_request_tx,
                generation_id,
                next_sequence_to_queue,
                segment_text,
                voice.clone(),
                event_tx,
            ) {
                tts_error = Some(err);
                return false;
            }
            next_sequence_to_queue = next_sequence_to_queue.saturating_add(1);
            outstanding_segments += 1;
        }

        if let Err(err) = drain_ready_tts_results(
            tts_result_rx,
            generation_id,
            &mut pending_segments,
            &mut next_sequence_to_emit,
            event_tx,
        ) {
            tts_error = Some(err);
            return false;
        }

        true
    })?;

    if let Some(err) = tts_error {
        return Err(err);
    }

    let response = response.trim().to_string();
    if !response.is_empty() && last_emitted_text != response {
        let _ = event_tx.send(VoiceEvent::ResponsePreview(response.clone()));
    }

    if let Some(remaining) = response.get(emitted_bytes..) {
        let remaining = remaining.trim();
        if !remaining.is_empty() {
            queue_tts_segment(
                tts_request_tx,
                generation_id,
                next_sequence_to_queue,
                remaining.to_string(),
                voice.clone(),
                event_tx,
            )?;
            outstanding_segments += 1;
        }
    }

    wait_for_tts_results(
        tts_result_rx,
        generation_id,
        outstanding_segments,
        &mut pending_segments,
        &mut next_sequence_to_emit,
        event_tx,
    )?;
    let _ = event_tx.send(VoiceEvent::AudioComplete);
    Ok(response)
}

fn queue_tts_segment(
    tts_request_tx: &Sender<TtsWorkerCommand>,
    generation_id: u64,
    sequence: u32,
    text: String,
    voice: VoiceProfile,
    event_tx: &Sender<VoiceEvent>,
) -> Result<()> {
    let _ = event_tx.send(VoiceEvent::StateChanged(VoiceState::Synthesizing));
    tts_request_tx
        .send(TtsWorkerCommand::Synthesize {
            generation_id,
            sequence,
            text,
            voice,
        })
        .map_err(|e| VoiceError::ChannelError(e.to_string()))?;
    Ok(())
}

fn run_tts_worker_loop(
    mut tts: TextToSpeech,
    request_rx: Receiver<TtsWorkerCommand>,
    result_tx: Sender<TtsWorkerResult>,
) {
    while let Ok(command) = request_rx.recv() {
        match command {
            TtsWorkerCommand::Synthesize {
                generation_id,
                sequence,
                text,
                voice,
            } => match tts.synthesize(&text, &voice) {
                Ok(audio) => {
                    let duration_secs = audio.len() as f32 / tts.sample_rate() as f32;
                    if result_tx
                        .send(TtsWorkerResult::SegmentReady {
                            generation_id,
                            sequence,
                            text,
                            audio,
                            duration_secs,
                        })
                        .is_err()
                    {
                        break;
                    }
                }
                Err(err) => {
                    if result_tx
                        .send(TtsWorkerResult::SegmentFailed {
                            generation_id,
                            sequence,
                            error: err.to_string(),
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            },
        }
    }
}

fn drain_ready_tts_results(
    tts_result_rx: &Receiver<TtsWorkerResult>,
    generation_id: u64,
    pending_segments: &mut BTreeMap<u32, SpokenAudioSegment>,
    next_sequence_to_emit: &mut u32,
    event_tx: &Sender<VoiceEvent>,
) -> Result<()> {
    loop {
        match tts_result_rx.try_recv() {
            Ok(result) => {
                handle_tts_result(
                    result,
                    generation_id,
                    pending_segments,
                    next_sequence_to_emit,
                    event_tx,
                )?;
            }
            Err(crossbeam_channel::TryRecvError::Empty) => break,
            Err(crossbeam_channel::TryRecvError::Disconnected) => {
                return Err(VoiceError::ChannelError(
                    "TTS worker disconnected unexpectedly".to_string(),
                ));
            }
        }
    }

    Ok(())
}

fn wait_for_tts_results(
    tts_result_rx: &Receiver<TtsWorkerResult>,
    generation_id: u64,
    expected_segments: usize,
    pending_segments: &mut BTreeMap<u32, SpokenAudioSegment>,
    next_sequence_to_emit: &mut u32,
    event_tx: &Sender<VoiceEvent>,
) -> Result<()> {
    while (*next_sequence_to_emit as usize) < expected_segments {
        let result = tts_result_rx
            .recv()
            .map_err(|e| VoiceError::ChannelError(e.to_string()))?;
        handle_tts_result(
            result,
            generation_id,
            pending_segments,
            next_sequence_to_emit,
            event_tx,
        )?;
    }

    Ok(())
}

fn handle_tts_result(
    result: TtsWorkerResult,
    generation_id: u64,
    pending_segments: &mut BTreeMap<u32, SpokenAudioSegment>,
    next_sequence_to_emit: &mut u32,
    event_tx: &Sender<VoiceEvent>,
) -> Result<()> {
    match result {
        TtsWorkerResult::SegmentReady {
            generation_id: result_generation,
            sequence,
            text,
            audio,
            duration_secs,
        } => {
            if result_generation != generation_id {
                return Ok(());
            }
            pending_segments.insert(
                sequence,
                SpokenAudioSegment {
                    sequence,
                    text,
                    audio,
                    duration_secs,
                },
            );
            emit_ready_spoken_segments(pending_segments, next_sequence_to_emit, event_tx);
            Ok(())
        }
        TtsWorkerResult::SegmentFailed {
            generation_id: result_generation,
            sequence,
            error,
        } => {
            if result_generation != generation_id {
                return Ok(());
            }
            Err(VoiceError::TtsError(format!(
                "Failed to synthesize segment {}: {}",
                sequence, error
            )))
        }
    }
}

fn emit_ready_spoken_segments(
    pending_segments: &mut BTreeMap<u32, SpokenAudioSegment>,
    next_sequence_to_emit: &mut u32,
    event_tx: &Sender<VoiceEvent>,
) {
    while let Some(segment) = pending_segments.remove(next_sequence_to_emit) {
        let _ = event_tx.send(VoiceEvent::StateChanged(VoiceState::Speaking));
        let _ = event_tx.send(VoiceEvent::SpokenSegment(segment));
        *next_sequence_to_emit = next_sequence_to_emit.saturating_add(1);
    }
}

fn drain_ready_segments(text: &str) -> (Vec<String>, usize) {
    const MIN_SOFT_BREAK_CHARS: usize = 48;
    const MAX_SEGMENT_CHARS: usize = 140;

    let mut segments = Vec::new();
    let mut start = 0usize;
    let mut consumed = 0usize;
    let mut chars_in_segment = 0usize;
    let mut last_whitespace_end = None;

    for (i, ch) in text.char_indices() {
        chars_in_segment += 1;
        if ch.is_whitespace() {
            last_whitespace_end = Some(i + ch.len_utf8());
        }

        let hard_break = matches!(ch, '.' | '!' | '?');
        let soft_break = matches!(ch, ',' | ';' | ':') && chars_in_segment >= MIN_SOFT_BREAK_CHARS;
        let forced_break = chars_in_segment >= MAX_SEGMENT_CHARS && last_whitespace_end.is_some();

        if !(hard_break || soft_break || forced_break) {
            continue;
        }

        let end = if forced_break {
            last_whitespace_end.unwrap_or(i + ch.len_utf8())
        } else {
            i + ch.len_utf8()
        };
        let segment = text[start..end].trim();
        if !segment.is_empty() {
            segments.push(segment.to_string());
        }
        start = end;
        consumed = end;
        chars_in_segment = 0;
        last_whitespace_end = None;
    }

    (segments, consumed)
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

    #[test]
    fn test_drain_ready_segments_keeps_partial_tail() {
        let (segments, consumed) = drain_ready_segments("Hello traveler. What brings you here");

        assert_eq!(segments, vec!["Hello traveler.".to_string()]);
        assert_eq!(consumed, "Hello traveler.".len());
    }

    #[test]
    fn test_drain_ready_segments_uses_soft_breaks_for_long_clauses() {
        let text =
            "The old mill by the eastern bridge broke again at dawn, and we still have grain waiting for tomorrow";
        let (segments, consumed) = drain_ready_segments(text);

        assert_eq!(
            segments,
            vec!["The old mill by the eastern bridge broke again at dawn,".to_string()]
        );
        assert_eq!(
            consumed,
            "The old mill by the eastern bridge broke again at dawn,".len()
        );
    }
}
