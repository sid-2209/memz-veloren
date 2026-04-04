//! Voice Dialogue System for Veloren integration.
//!
//! SOTA overhaul: Event-driven architecture using the new VoicePipeline.
//! Bridges the Veloren game engine's ECS/HUD with the async voice pipeline.
//!
//! The game thread sends commands and polls for events each frame.
//! All heavy processing (STT, LLM, TTS) happens on a separate thread.

use memz_voice::{
    LlmConfig, SttConfig, TtsConfig, VoiceEvent, VoicePipeline, VoicePipelineConfig, VoiceState,
    llm::NpcContext,
    voice_profile::VoiceProfile,
};

/// Voice dialogue state for the Veloren integration layer.
pub struct VoiceSystem {
    pipeline: Option<VoicePipeline>,
    active_npc_id: Option<u64>,
    active_npc_name: Option<String>,
    active_npc_position: Option<[f32; 3]>,
    display_state: VoiceDisplayState,
    last_transcription: Option<String>,
    last_response: Option<String>,
    is_initialized: bool,
}

/// UI display state for the voice indicator.
#[derive(Debug, Clone, PartialEq)]
pub enum VoiceDisplayState {
    Hidden,
    Listening,
    Processing,
    Thinking,
    Speaking,
    Error(String),
}

/// NPC data extracted from Veloren's ECS for voice context.
#[derive(Debug, Clone)]
pub struct VelorenNpcData {
    pub entity_id: u64,
    pub name: String,
    pub profession: String,
    pub location: String,
    pub faction: String,
    pub personality_description: String,
    pub mood: String,
    pub player_sentiment: String,
    pub extraversion: f32,
    pub neuroticism: f32,
    pub position: [f32; 3],
    /// Profession-specific knowledge injected into the LLM system prompt.
    pub knowledge: String,
}

impl VoiceSystem {
    pub fn new() -> Self {
        log::info!("VoiceSystem created (lazy initialization)");
        Self {
            pipeline: None,
            active_npc_id: None,
            active_npc_name: None,
            active_npc_position: None,
            display_state: VoiceDisplayState::Hidden,
            last_transcription: None,
            last_response: None,
            is_initialized: false,
        }
    }

    /// Ensure the pipeline is initialized with auto-discovered model paths.
    fn ensure_pipeline(&mut self) -> Result<(), String> {
        if self.pipeline.is_some() {
            return Ok(());
        }

        log::info!("Initializing voice pipeline...");

        let config = build_pipeline_config();

        match VoicePipeline::new(config) {
            Ok(pipeline) => {
                self.pipeline = Some(pipeline);
                self.is_initialized = true;
                log::info!("Voice pipeline initialized successfully");
                Ok(())
            }
            Err(e) => {
                log::error!("Failed to initialize voice pipeline: {}", e);
                Err(e.to_string())
            }
        }
    }

    /// Start a voice interaction with an NPC (player pressed PTT key).
    pub fn start_interaction(&mut self, npc: &VelorenNpcData) -> Result<(), String> {
        self.ensure_pipeline()?;

        let pipeline = self.pipeline.as_mut().unwrap();

        if pipeline.is_active() {
            log::warn!("Already in a voice interaction — ignoring");
            return Ok(());
        }

        log::info!(
            "Starting voice interaction with NPC: {} ({})",
            npc.name,
            npc.profession
        );

        let npc_context = NpcContext {
            name: npc.name.clone(),
            profession: npc.profession.clone(),
            location: npc.location.clone(),
            faction: npc.faction.clone(),
            personality: npc.personality_description.clone(),
            mood: npc.mood.clone(),
            knowledge: npc.knowledge.clone(),
            player_sentiment: npc.player_sentiment.clone(),
        };

        let voice_profile = VoiceProfile::from_npc(
            &npc.profession,
            npc.extraversion,
            npc.neuroticism,
        );

        self.active_npc_id = Some(npc.entity_id);
        self.active_npc_name = Some(npc.name.clone());
        self.active_npc_position = Some(npc.position);
        self.last_transcription = None;
        self.last_response = None;

        pipeline
            .start_recording(npc.entity_id, npc_context, voice_profile)
            .map_err(|e| e.to_string())?;

        self.display_state = VoiceDisplayState::Listening;
        Ok(())
    }

    /// Stop recording (player released PTT key).
    pub fn stop_recording(&mut self) -> Result<(), String> {
        if let Some(pipeline) = &mut self.pipeline {
            pipeline.stop_recording().map_err(|e| e.to_string())?;
            self.display_state = VoiceDisplayState::Processing;
        }
        Ok(())
    }

    /// Cancel the current voice interaction.
    pub fn cancel_interaction(&mut self) {
        if let Some(pipeline) = &mut self.pipeline {
            let _ = pipeline.cancel();
        }
        self.active_npc_id = None;
        self.active_npc_name = None;
        self.active_npc_position = None;
        self.display_state = VoiceDisplayState::Hidden;
    }

    /// Update the voice system (call every frame).
    pub fn update(&mut self) -> Vec<VoiceGameEvent> {
        let mut game_events = Vec::new();

        let pipeline = match &mut self.pipeline {
            Some(p) => p,
            None => return game_events,
        };

        for event in pipeline.drain_events() {
            match event {
                VoiceEvent::StateChanged(state) => {
                    self.display_state = match &state {
                        VoiceState::Idle => VoiceDisplayState::Hidden,
                        VoiceState::Listening => VoiceDisplayState::Listening,
                        VoiceState::Transcribing => VoiceDisplayState::Processing,
                        VoiceState::Thinking => VoiceDisplayState::Thinking,
                        VoiceState::Synthesizing => VoiceDisplayState::Thinking,
                        VoiceState::Speaking => VoiceDisplayState::Speaking,
                        VoiceState::Error(msg) => VoiceDisplayState::Error(msg.clone()),
                    };
                    if state == VoiceState::Idle {
                        self.active_npc_id = None;
                        self.active_npc_name = None;
                        self.active_npc_position = None;
                        game_events.push(VoiceGameEvent::InteractionComplete);
                    }
                }
                VoiceEvent::Transcription(text) => {
                    log::info!("Player said: \"{}\"", text);
                    self.last_transcription = Some(text.clone());
                    game_events.push(VoiceGameEvent::PlayerTranscription(text));
                }
                VoiceEvent::ResponseText(text) => {
                    log::info!("NPC response: \"{}\"", text);
                    self.last_response = Some(text.clone());
                    game_events.push(VoiceGameEvent::NpcResponseText {
                        npc_id: self.active_npc_id.unwrap_or(0),
                        npc_name: self
                            .active_npc_name
                            .clone()
                            .unwrap_or_else(|| "NPC".to_string()),
                        text,
                    });
                }
                VoiceEvent::AudioChunk(audio) => {
                    game_events.push(VoiceGameEvent::NpcAudioChunk {
                        npc_id: self.active_npc_id.unwrap_or(0),
                        position: self.active_npc_position.unwrap_or([0.0; 3]),
                        audio,
                    });
                }
                VoiceEvent::AudioComplete => {
                    game_events.push(VoiceGameEvent::NpcAudioComplete {
                        npc_id: self.active_npc_id.unwrap_or(0),
                    });
                }
                VoiceEvent::Error(msg) => {
                    log::error!("Voice error: {}", msg);
                    self.display_state = VoiceDisplayState::Error(msg.clone());
                    game_events.push(VoiceGameEvent::Error(msg));
                }
            }
        }

        game_events
    }

    pub fn display_state(&self) -> &VoiceDisplayState { &self.display_state }
    pub fn last_transcription(&self) -> Option<&str> { self.last_transcription.as_deref() }
    pub fn last_response(&self) -> Option<&str> { self.last_response.as_deref() }
    pub fn is_active(&self) -> bool { self.pipeline.as_ref().map_or(false, |p| p.is_active()) }
    pub fn active_npc(&self) -> Option<u64> { self.active_npc_id }
    pub fn is_ready(&self) -> bool { self.is_initialized }
}

impl Default for VoiceSystem {
    fn default() -> Self { Self::new() }
}

/// Events produced by the voice system for the game to handle.
#[derive(Debug, Clone)]
pub enum VoiceGameEvent {
    PlayerTranscription(String),
    NpcResponseText {
        npc_id: u64,
        npc_name: String,
        text: String,
    },
    NpcAudioChunk { npc_id: u64, position: [f32; 3], audio: Vec<f32> },
    NpcAudioComplete { npc_id: u64 },
    InteractionComplete,
    Error(String),
}

// ═══════════════════════════════════════════════════════════════════════
// Pipeline configuration: auto-discover models
// ═══════════════════════════════════════════════════════════════════════

/// Build a VoicePipelineConfig with auto-discovered model paths.
///
/// Searches common locations for the Whisper model. The Blitz TTS server,
/// optional Kokoro fallback, and Ollama LLM server are expected to run on localhost.
fn build_pipeline_config() -> VoicePipelineConfig {
    let whisper_model = find_model_file("whisper-tiny.en.bin");
    let ollama_url = "http://localhost:11434".to_string();
    let blitz_url = "http://localhost:8890".to_string();
    let kokoro_url = "http://localhost:8880".to_string();

    if whisper_model.is_empty() {
        log::warn!(
            "Whisper model not found. Voice input (STT) will not work.\n\
             Download with:\n\
             curl -L -o models/whisper-tiny.en.bin \\\n\
             https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin"
        );
    } else {
        log::info!("STT model: {}", whisper_model);
    }

    log::info!("LLM endpoint: {}", ollama_url);
    log::info!("Primary TTS endpoint: {} (Blitz TTS)", blitz_url);
    log::info!("Fallback TTS endpoint: {} (Kokoro, optional)", kokoro_url);

    VoicePipelineConfig {
        stt: SttConfig {
            model_path: whisper_model,
            language: "en".to_string(),
            vad_threshold: 0.1, // Low threshold — PTT fallback handles quiet mics (AirPods etc.)
            use_gpu: true,
        },
        tts: TtsConfig {
            blitz_server_url: blitz_url,
            model_path: String::new(),
            kokoro_server_url: kokoro_url,
            sample_rate: 24000,
            default_voice: VoiceProfile::default(),
            allow_macos_say_fallback: false,
        },
        llm: LlmConfig {
            ollama_url,
            model_name: "llama3.2:3b".to_string(), // 3b: dramatically better dialogue quality
            temperature: 0.8,
            max_tokens: 80,  // 1-2 sentences for natural voice pacing
            seed: None,
            context_size: 4096,
        },
        allow_live_partial_responses: false,
        stream_response_audio: false,
    }
}

/// Search common locations for a voice model file.
///
/// Searches relative to the current working directory (which for
/// `cargo run` in the veloren directory is `veloren/`) and also
/// absolute paths for common development setups.
fn find_model_file(filename: &str) -> String {
    let search_paths = [
        // Dev: running from project root
        format!("models/{}", filename),
        // Dev: running from veloren/ subdirectory
        format!("../models/{}", filename),
        // Dev: running from veloren/voxygen
        format!("../../models/{}", filename),
        // Dev: from binary location (3 levels up from target/debug)
        format!("../../../models/{}", filename),
    ];

    for path in &search_paths {
        if std::path::Path::new(path).exists() {
            // Canonicalize to get absolute path
            if let Ok(abs) = std::fs::canonicalize(path) {
                let abs_str = abs.to_string_lossy().to_string();
                log::info!("Found model: {} → {}", filename, abs_str);
                return abs_str;
            }
            return path.clone();
        }
    }

    // Try relative to the executable
    if let Ok(exe_path) = std::env::current_exe() {
        let mut dir = exe_path;
        for _ in 0..5 {
            dir = match dir.parent() {
                Some(p) => p.to_path_buf(),
                None => break,
            };
            let candidate = dir.join("models").join(filename);
            if candidate.exists() {
                let path_str = candidate.to_string_lossy().to_string();
                log::info!("Found model (exe-relative): {} → {}", filename, path_str);
                return path_str;
            }
        }
    }

    // Try HOME directory
    if let Ok(home) = std::env::var("HOME") {
        let home_path = std::path::Path::new(&home)
            .join(".local/share/veloren-memz/models")
            .join(filename);
        if home_path.exists() {
            return home_path.to_string_lossy().to_string();
        }
    }

    log::warn!("Model file not found: {}", filename);
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_system_creation() {
        let system = VoiceSystem::new();
        assert!(!system.is_active());
        assert_eq!(system.display_state(), &VoiceDisplayState::Hidden);
    }

    #[test]
    fn test_npc_data() {
        let npc = VelorenNpcData {
            entity_id: 42,
            name: "Thorin".into(),
            profession: "Blacksmith".into(),
            location: "Ironforge".into(),
            faction: "Mountain Clan".into(),
            personality_description: "Gruff but kind".into(),
            mood: "content".into(),
            player_sentiment: "neutral".into(),
            extraversion: 0.3,
            neuroticism: 0.2,
            position: [100.0, 200.0, 50.0],
            knowledge: String::new(),
        };
        assert_eq!(npc.name, "Thorin");
        assert!(npc.extraversion < 0.5);
    }

    #[test]
    fn test_find_model_file() {
        // Should not panic — just return empty string if not found
        let result = find_model_file("nonexistent-model.bin");
        // Either found or empty, never panics
        let _ = result;
    }
}
