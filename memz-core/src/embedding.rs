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
// ONNX provider placeholder
// ---------------------------------------------------------------------------

/// Placeholder for the production ONNX-based embedding provider.
///
/// This will use `fastembed-rs` or `ort` (ONNX Runtime for Rust) to
/// load `all-MiniLM-L6-v2` (~80 MB) and generate 384-dimensional
/// embeddings.
///
/// # Implementation plan (Phase 1)
///
/// ```ignore
/// use fastembed::{TextEmbedding, InitOptions, EmbeddingModel};
///
/// let model = TextEmbedding::try_new(InitOptions {
///     model_name: EmbeddingModel::AllMiniLML6V2,
///     show_download_progress: true,
///     ..Default::default()
/// })?;
///
/// let embeddings = model.embed(vec!["Hello, world!"], None)?;
/// ```
///
/// For now, this struct exists to document the intended production path.
/// Add `fastembed = "4"` to `[dependencies]` in Phase 1 and implement.
pub struct OnnxEmbeddingProvider {
    _private: (),
}

impl OnnxEmbeddingProvider {
    /// Create a new ONNX embedding provider.
    ///
    /// # Errors
    ///
    /// Returns an error if the ONNX model cannot be loaded.
    pub fn new(_model_path: &str) -> Result<Self> {
        Err(MemzError::Config(
            "ONNX embedding provider not yet implemented — use StubEmbeddingProvider or \
             RandomEmbeddingProvider for now"
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
