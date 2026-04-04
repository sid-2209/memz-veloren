use thiserror::Error;

#[derive(Error, Debug)]
pub enum VoiceError {
    #[error("Speech-to-text error: {0}")]
    SttError(String),

    #[error("Text-to-speech error: {0}")]
    TtsError(String),

    #[error("LLM inference error: {0}")]
    LlmError(String),

    #[error("Audio device error: {0}")]
    AudioError(String),

    #[error("Model loading error: {0}")]
    ModelError(String),

    #[error("Voice activity detection error: {0}")]
    VadError(String),

    #[error("Audio resampling error: {0}")]
    ResampleError(String),

    #[error("Pipeline state error: {0}")]
    PipelineError(String),

    #[error("Model not found at path: {0}")]
    ModelNotFound(String),

    #[error("Channel communication error: {0}")]
    ChannelError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Other error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, VoiceError>;
