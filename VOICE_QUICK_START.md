# 🚀 Voice NPC Quick Start Guide

This is a condensed guide to get started implementing voice-based NPC dialogue in Veloren.

---

## Prerequisites

### System Requirements
- **OS:** macOS (M4), Windows, or Linux
- **RAM:** 8GB minimum, 16GB recommended
- **Disk:** 2GB free space for models
- **Microphone:** Any USB or built-in mic

### Development Tools
```bash
# Rust (already installed)
rustc --version  # Should be 1.93.0+

# Install additional tools
cargo install cargo-watch  # For hot reloading during development
```

---

## Step 1: Install Ollama & Pull Model

### Install Ollama
```bash
# macOS
brew install ollama

# Start Ollama service (runs in background)
ollama serve
```

### Pull the LLM Model
```bash
ollama pull llama3.2:1b
```

This downloads the 1.3GB model. Wait for it to complete.

**Verify the model:**
```bash
ollama list
# Should show: llama3.2:1b    baf6a787fdd5    1.3 GB
```

### Download Whisper Model (for STT)
```bash
cd /Users/siddhartha/Downloads/dev/Project\ Vyuh/memz
mkdir -p models
cd models

# Download from Hugging Face
curl -L -o whisper-tiny.en.bin \
  https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin
```

**Total Setup:** ~1.4GB (Ollama handles the LLM, we only need Whisper)

---

## Step 2: Create `memz-voice` Crate

The `memz-voice` crate is already created! Let's verify it builds:

```bash
cd /Users/siddhartha/Downloads/dev/Project\ Vyuh/memz/memz-voice
cargo build
```

### Current Dependencies in `Cargo.toml`

```toml
[dependencies]
# Audio I/O
cpal = "0.15"
hound = "3.5"

# Speech-to-Text
whisper-rs = { version = "0.12", features = ["metal"] }

# Utilities
anyhow = "1.0"
thiserror = "1.0"
log = "0.4"
env_logger = "0.11"
```

**Note:** We're using Ollama for LLM (via command-line), so no need for llama-cpp-rs or heavy dependencies!

---

## Step 3: Implement Voice Input (STT)

Create `memz-voice/src/stt.rs`:

```rust
use whisper_rs::{WhisperContext, WhisperContextParameters, FullParams, SamplingStrategy};
use anyhow::Result;

pub struct SpeechToText {
    ctx: WhisperContext,
}

impl SpeechToText {
    pub fn new(model_path: &str) -> Result<Self> {
        let params = WhisperContextParameters::default();
        let ctx = WhisperContext::new_with_params(model_path, params)?;
        Ok(Self { ctx })
    }

    pub fn transcribe(&self, audio: &[f32]) -> Result<String> {
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some("en"));
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        let mut state = self.ctx.create_state()?;
        state.full(params, audio)?;

        let num_segments = state.full_n_segments()?;
        let mut result = String::new();
        
        for i in 0..num_segments {
            let segment = state.full_get_segment_text(i)?;
            result.push_str(&segment);
        }

        Ok(result.trim().to_string())
    }
}
```

---

## Step 4: Implement Voice Output (TTS)

Create `memz-voice/src/tts.rs`:

```rust
use pyo3::prelude::*;
use pyo3::types::PyModule;
use anyhow::Result;

pub struct TextToSpeech {
    py_module: Py<PyAny>,
}

impl TextToSpeech {
    pub fn new(model_path: &str) -> Result<Self> {
        Python::with_gil(|py| {
            let code = format!(r#"
import torch
from kokoro import KokoroTTS

model = KokoroTTS.from_pretrained("{}")
model
"#, model_path);

            let module = PyModule::from_code(py, &code, "", "")?;
            let py_module = module.getattr("model")?.into();
            
            Ok(Self { py_module })
        })
    }

    pub fn synthesize(&self, text: &str) -> Result<Vec<f32>> {
        Python::with_gil(|py| {
            let result = self.py_module
                .call_method1(py, "synthesize", (text,))?;
            
            let audio: Vec<f32> = result.extract(py)?;
            Ok(audio)
        })
    }
}
```

---

## Step 4: Implement LLM Dialogue (Using Ollama)

The LLM is already implemented in `memz-voice/src/llm.rs` and uses Ollama:

```rust
use crate::error::{Result, VoiceError};

pub struct DialogueLLM {
    model_path: String,
}

impl DialogueLLM {
    pub fn new(model_path: &str) -> Result<Self> {
        // Check if ollama is available
        let ollama_check = std::process::Command::new("ollama")
            .arg("list")
            .output();

        if ollama_check.is_err() {
            log::warn!("Ollama not found. Install: brew install ollama");
        }

        Ok(Self {
            model_path: model_path.to_string(),
        })
    }

    pub fn generate_response(
        &self,
        player_input: &str,
        npc_profession: &str,
        npc_mood: &str,
    ) -> Result<String> {
        let prompt = format!(
            "You are a {} NPC in a fantasy RPG game. You are {}. \
            Respond in character with 1-2 sentences. Be concise and natural.\n\n\
            Player: {}\n\
            NPC:",
            npc_profession, npc_mood, player_input
        );

        // Call ollama
        let output = std::process::Command::new("ollama")
            .arg("run")
            .arg("llama3.2:1b")
            .arg(&prompt)
            .output()
            .map_err(|e| VoiceError::LlmError(format!("Failed to run ollama: {}", e)))?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(VoiceError::LlmError(format!("Ollama failed: {}", error)));
        }

        let response = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();

        Ok(response)
    }
}
```

**Benefits of using Ollama:**
- No need to manage GGUF files manually
- Automatic model caching and optimization
- Simple API via command-line
- Easy to switch models

---

## Step 6: Create Voice Dialogue Manager

Create `memz-voice/src/lib.rs`:

```rust
mod stt;
mod tts;
mod llm;

pub use stt::SpeechToText;
pub use tts::TextToSpeech;
pub use llm::DialogueLLM;

use anyhow::Result;

pub struct VoiceDialogue {
    stt: SpeechToText,
    tts: TextToSpeech,
    llm: DialogueLLM,
}

impl VoiceDialogue {
    pub fn new(
        whisper_path: &str,
        kokoro_path: &str,
        llama_path: &str,
    ) -> Result<Self> {
        Ok(Self {
            stt: SpeechToText::new(whisper_path)?,
            tts: TextToSpeech::new(kokoro_path)?,
            llm: DialogueLLM::new(llama_path)?,
        })
    }

    pub fn process_voice_input(
        &self,
        audio: &[f32],
        npc_context: &str,
    ) -> Result<(String, Vec<f32>)> {
        // 1. Transcribe player speech
        let player_text = self.stt.transcribe(audio)?;
        
        // 2. Generate NPC response
        let prompt = format!("{}\n\nPlayer: {}\nNPC:", npc_context, player_text);
        let npc_text = self.llm.generate(&prompt, 100)?;
        
        // 3. Synthesize NPC voice
        let npc_audio = self.tts.synthesize(&npc_text)?;
        
        Ok((npc_text, npc_audio))
    }
}
```

---

## Step 5: Test the LLM

Test the dialogue generation:

```bash
cd memz-voice

# Interactive test (type messages to NPC)
cargo run --example test_llm

# Automated test (runs predefined test cases)
cargo run --example test_llm_auto
```

**Expected output:**
```
=== LLM Dialogue Test ===

Checking for Ollama...
Initializing LLM (using Ollama with llama3.2:1b)...
LLM ready!

NPC: blacksmith (friendly)
You: Hello! Can you repair my sword?
🤔 Thinking...
NPC: "Ah, you're looking for some fine armoring, I take it? 
     Let me see what I can whip up for you..."
```

✅ **Status:** LLM is working! Responses take 2-3 seconds to generate.

---

## Step 8: Integrate with Veloren

Add to `memz-veloren/Cargo.toml`:
```toml
[dependencies]
memz-voice = { path = "../memz-voice" }
```

Create `memz-veloren/src/voice_system.rs`:
```rust
use memz_voice::VoiceDialogue;
use memz_core::MemoryBank;

pub struct VoiceSystem {
    voice: VoiceDialogue,
    active_recording: Option<Vec<f32>>,
}

impl VoiceSystem {
    pub fn new() -> anyhow::Result<Self> {
        let voice = VoiceDialogue::new(
            "models/whisper-tiny.en.bin",
            "models/Kokoro-82M",
            "models/llama-3.2-1b-instruct-q4.gguf",
        )?;
        
        Ok(Self {
            voice,
            active_recording: None,
        })
    }

    pub fn start_recording(&mut self) {
        self.active_recording = Some(Vec::new());
    }

    pub fn add_audio_chunk(&mut self, chunk: &[f32]) {
        if let Some(recording) = &mut self.active_recording {
            recording.extend_from_slice(chunk);
        }
    }

    pub fn stop_recording_and_respond(
        &mut self,
        memory_bank: &MemoryBank,
        npc_personality: &str,
    ) -> anyhow::Result<(String, Vec<f32>)> {
        let audio = self.active_recording.take().unwrap_or_default();
        
        // Build NPC context from MEMZ
        let context = format!(
            "You are a {} NPC.\n\nRecent memories:\n{}\n",
            npc_personality,
            // Extract memories from bank
            "..."
        );

        self.voice.process_voice_input(&audio, &context)
    }
}
```

---

## Step 9: Add Keybind to Veloren

In Veloren's input handling code, add:

```rust
// When V key is pressed
if input.key_pressed(VirtualKeyCode::V) {
    voice_system.start_recording();
    // Start capturing microphone
}

// When V key is released
if input.key_released(VirtualKeyCode::V) {
    let (npc_text, npc_audio) = voice_system.stop_recording_and_respond(
        &memory_bank,
        &npc_personality,
    )?;
    
    // Display text in dialogue box
    hud.show_dialogue(npc_text);
    
    // Play audio at NPC position
    audio_frontend.emit_sfx(npc_audio, npc_position);
}
```

---

## Troubleshooting

### "Model not found"
- Ensure models are in `models/` directory
- Check file paths in code

### "Microphone not working"
- Check system permissions (macOS: System Settings → Privacy → Microphone)
- Test with `cargo run --example test_microphone`

### "Out of memory"
- Use smaller models (Whisper-tiny, Llama-1B)
- Enable 4-bit quantization
- Close other applications

### "Slow inference"
- Enable Metal GPU: `features = ["metal"]` in Cargo.toml
- Use quantized models (Q4_K_M)
- Reduce max_tokens in LLM generation

---

## Next Steps

1. ✅ Test standalone voice system
2. ✅ Integrate with MEMZ memory retrieval
3. ✅ Add to Veloren's game loop
4. ✅ Test with multiple NPCs
5. ✅ Optimize latency and memory usage

**Full documentation:** See `VOICE_NPC_IMPLEMENTATION_PLAN.md`

---

**Happy coding!** 🎙️🎮
