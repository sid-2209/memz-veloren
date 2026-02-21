//! Vector embedding abstraction layer.
//!
//! Provides a trait-based interface for generating text embeddings
//! used by the retrieval engine for semantic similarity search.
//!
//! The production implementation uses ONNX Runtime (via `fastembed-rs`)
//! with the `all-MiniLM-L6-v2` model.  A stub implementation is
//! provided for tests and for the "Ultra-Low" hardware profile.

use crate::error::{MemzError, Result};
use crate::types::Embedding;

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Generate vector embeddings from text.
///
/// Implementations must be `Send + Sync` for use from async contexts.
pub trait EmbeddingProvider: Send + Sync {
    /// Embed a single text string.
    ///
    /// Returns a vector of `embedding_dimensions()` floats.
    ///
    /// # Errors
    ///
    /// Returns [`MemzError::Serialization`] if the model fails to
    /// produce an embedding.
    fn embed(&self, text: &str) -> Result<Embedding>;

    /// Embed a batch of texts.
    ///
    /// Default implementation calls `embed` in a loop.  High-throughput
    /// providers should override this with a native batch API.
    ///
    /// # Errors
    ///
    /// Returns an error if any embedding in the batch fails.
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>> {
        texts.iter().map(|t| self.embed(t)).collect()
    }

    /// The dimensionality of embeddings produced by this provider.
    fn dimensions(&self) -> usize;

    /// A human-readable name for the model (e.g. `"all-MiniLM-L6-v2"`).
    fn model_name(&self) -> &str;
}

// ---------------------------------------------------------------------------
// Cosine similarity
// ---------------------------------------------------------------------------

/// Compute the cosine similarity between two embedding vectors.
///
/// Returns a value in \[-1.0, 1.0\].  Returns `0.0` if either vector
/// has zero magnitude (edge case guard).
#[must_use]
pub fn cosine_similarity(a: &Embedding, b: &Embedding) -> f32 {
    if a.0.len() != b.0.len() {
        return 0.0;
    }

    let mut dot = 0.0_f32;
    let mut mag_a = 0.0_f32;
    let mut mag_b = 0.0_f32;

    for (x, y) in a.0.iter().zip(b.0.iter()) {
        dot += x * y;
        mag_a += x * x;
        mag_b += y * y;
    }

    let denom = mag_a.sqrt() * mag_b.sqrt();
    if denom < f32::EPSILON {
        return 0.0;
    }

    dot / denom
}

// ---------------------------------------------------------------------------
// Stub / Zero-cost provider (for tests & ultra-low hardware)
// ---------------------------------------------------------------------------

/// A stub embedding provider that returns zero-vectors.
///
/// This is used for:
/// - Unit tests that don't need real embeddings
/// - The "Ultra-Low" hardware profile (keyword-match fallback)
/// - Development/debugging
pub struct StubEmbeddingProvider {
    dims: usize,
}

impl StubEmbeddingProvider {
    /// Create a new stub provider with the given dimensionality.
    #[must_use]
    pub fn new(dimensions: usize) -> Self {
        Self { dims: dimensions }
    }
}

impl Default for StubEmbeddingProvider {
    fn default() -> Self {
        Self::new(384)
    }
}

impl EmbeddingProvider for StubEmbeddingProvider {
    fn embed(&self, _text: &str) -> Result<Embedding> {
        Ok(Embedding(vec![0.0; self.dims]))
    }

    fn dimensions(&self) -> usize {
        self.dims
    }

    fn model_name(&self) -> &str {
        "stub-zero-vector"
    }
}

// ---------------------------------------------------------------------------
// Normalized random provider (for integration testing)
// ---------------------------------------------------------------------------

/// An embedding provider that returns random unit-length vectors.
///
/// Useful for integration tests that need non-zero, diverse embeddings
/// without loading a real model.
pub struct RandomEmbeddingProvider {
    dims: usize,
}

impl RandomEmbeddingProvider {
    /// Create a new random provider.
    #[must_use]
    pub fn new(dimensions: usize) -> Self {
        Self { dims: dimensions }
    }
}

impl EmbeddingProvider for RandomEmbeddingProvider {
    fn embed(&self, _text: &str) -> Result<Embedding> {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let raw: Vec<f32> = (0..self.dims).map(|_| rng.gen_range(-1.0..1.0)).collect();

        // L2-normalize
        let mag: f32 = raw.iter().map(|x| x * x).sum::<f32>().sqrt();
        if mag < f32::EPSILON {
            return Ok(Embedding(vec![0.0; self.dims]));
        }
        let normed: Vec<f32> = raw.iter().map(|x| x / mag).collect();
        Ok(Embedding(normed))
    }

    fn dimensions(&self) -> usize {
        self.dims
    }

    fn model_name(&self) -> &str {
        "random-unit-vector"
    }
}

// ---------------------------------------------------------------------------
// ONNX-based embedding provider (feature-gated)
// ---------------------------------------------------------------------------

/// Production ONNX-based embedding provider using `fastembed-rs`.
///
/// Uses the `all-MiniLM-L6-v2` model (~80 MB) to generate 384-dimensional
/// embeddings suitable for semantic retrieval.
///
/// # Feature gate
///
/// This provider is only available when the `onnx` cargo feature is enabled:
///
/// ```toml
/// memz-core = { path = "../memz-core", features = ["onnx"] }
/// ```
///
/// # Example (when feature enabled)
///
/// ```ignore
/// use memz_core::embedding::{OnnxEmbeddingProvider, EmbeddingProvider};
///
/// let provider = OnnxEmbeddingProvider::new(None)?;
/// let emb = provider.embed("The trader sold rare gems")?;
/// assert_eq!(emb.0.len(), 384);
/// ```
#[cfg(feature = "onnx")]
pub struct OnnxEmbeddingProvider {
    model: fastembed::TextEmbedding,
    dims: usize,
    model_name_str: String,
}

#[cfg(feature = "onnx")]
impl OnnxEmbeddingProvider {
    /// Create a new ONNX embedding provider.
    ///
    /// If `model` is `None`, defaults to `AllMiniLML6V2` (384-dim).
    ///
    /// The model weights are downloaded on first use and cached in a
    /// platform-specific cache directory.
    ///
    /// # Errors
    ///
    /// Returns [`MemzError::Config`] if the ONNX model cannot be loaded.
    pub fn new(model: Option<fastembed::EmbeddingModel>) -> Result<Self> {
        let model_enum = model.unwrap_or(fastembed::EmbeddingModel::AllMiniLML6V2);
        let model_name_str = format!("{model_enum:?}");

        let init_options = fastembed::InitOptions::new(model_enum)
            .with_show_download_progress(true);

        let text_embedding = fastembed::TextEmbedding::try_new(init_options)
            .map_err(|e| MemzError::Config(format!("Failed to load ONNX model: {e}")))?;

        // Probe dimensionality with a test embedding
        let probe = text_embedding
            .embed(vec!["probe"], None)
            .map_err(|e| MemzError::Config(format!("Probe embedding failed: {e}")))?;

        let dims = probe
            .first()
            .map(|v| v.len())
            .unwrap_or(384);

        Ok(Self {
            model: text_embedding,
            dims,
            model_name_str,
        })
    }
}

#[cfg(feature = "onnx")]
impl EmbeddingProvider for OnnxEmbeddingProvider {
    fn embed(&self, text: &str) -> Result<Embedding> {
        let results = self
            .model
            .embed(vec![text], None)
            .map_err(|e| MemzError::Serialization(format!("ONNX embed failed: {e}")))?;

        results
            .into_iter()
            .next()
            .map(Embedding)
            .ok_or_else(|| MemzError::Serialization("ONNX returned empty result".to_string()))
    }

    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let owned: Vec<String> = texts.iter().map(|s| (*s).to_string());
        let results = self
            .model
            .embed(owned.collect(), None)
            .map_err(|e| MemzError::Serialization(format!("ONNX batch embed failed: {e}")))?;

        Ok(results.into_iter().map(Embedding).collect())
    }

    fn dimensions(&self) -> usize {
        self.dims
    }

    fn model_name(&self) -> &str {
        &self.model_name_str
    }
}

// ---------------------------------------------------------------------------
// Stub ONNX provider (when feature not enabled)
// ---------------------------------------------------------------------------

/// Placeholder for the ONNX-based embedding provider.
///
/// Enable the `onnx` feature to use the real implementation:
///
/// ```toml
/// memz-core = { path = "../memz-core", features = ["onnx"] }
/// ```
#[cfg(not(feature = "onnx"))]
pub struct OnnxEmbeddingProvider {
    _private: (),
}

#[cfg(not(feature = "onnx"))]
impl OnnxEmbeddingProvider {
    /// Create a new ONNX embedding provider.
    ///
    /// # Errors
    ///
    /// Always returns an error when the `onnx` feature is not enabled.
    pub fn new(_model_path: &str) -> Result<Self> {
        Err(MemzError::Config(
            "ONNX embedding provider requires the `onnx` feature — \
             compile with `cargo build --features onnx`, or use \
             StubEmbeddingProvider / RandomEmbeddingProvider"
                .to_string(),
        ))
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_identical_vectors() {
        let a = Embedding(vec![1.0, 0.0, 0.0]);
        let b = Embedding(vec![1.0, 0.0, 0.0]);
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_orthogonal_vectors() {
        let a = Embedding(vec![1.0, 0.0]);
        let b = Embedding(vec![0.0, 1.0]);
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 1e-6);
    }

    #[test]
    fn cosine_opposite_vectors() {
        let a = Embedding(vec![1.0, 0.0]);
        let b = Embedding(vec![-1.0, 0.0]);
        let sim = cosine_similarity(&a, &b);
        assert!((sim - (-1.0)).abs() < 1e-6);
    }

    #[test]
    fn cosine_mismatched_dimensions() {
        let a = Embedding(vec![1.0, 0.0]);
        let b = Embedding(vec![1.0, 0.0, 0.0]);
        assert_eq!(cosine_similarity(&a, &b), 0.0);
    }

    #[test]
    fn stub_provider_returns_zeros() {
        let provider = StubEmbeddingProvider::new(4);
        let emb = provider.embed("hello").expect("embed");
        assert_eq!(emb.0.len(), 4);
        assert!(emb.0.iter().all(|&x| x == 0.0));
    }

    #[test]
    fn random_provider_returns_unit_vectors() {
        let provider = RandomEmbeddingProvider::new(64);
        let emb = provider.embed("hello").expect("embed");
        assert_eq!(emb.0.len(), 64);
        let mag: f32 = emb.0.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((mag - 1.0).abs() < 0.01, "expected unit vector, got magnitude {mag}");
    }

    #[test]
    fn batch_embed_works() {
        let provider = StubEmbeddingProvider::new(8);
        let texts = vec!["hello", "world", "test"];
        let results = provider.embed_batch(&texts).expect("batch");
        assert_eq!(results.len(), 3);
    }
}
