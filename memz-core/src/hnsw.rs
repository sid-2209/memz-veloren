//! HNSW Vector Index — Approximate Nearest-Neighbor Search (§12.4)
//!
//! Wraps `instant-distance` to provide fast cosine-similarity search over
//! memory embeddings.  Used by the retrieval engine when the memory count
//! exceeds the brute-force threshold (default: 100 memories).
//!
//! ## Usage
//!
//! ```rust,no_run
//! # use memz_core::hnsw::HnswIndex;
//! # use memz_core::types::{Embedding, MemoryId};
//! let mut index = HnswIndex::new();
//! index.insert(MemoryId::new(), Embedding(vec![0.1, 0.2, 0.3]));
//! index.insert(MemoryId::new(), Embedding(vec![0.9, 0.8, 0.7]));
//! index.build();
//! let results = index.search(&Embedding(vec![0.1, 0.2, 0.3]), 5);
//! assert!(!results.is_empty());
//! ```

use instant_distance::{Builder, HnswMap, Point, Search};

use crate::types::{Embedding, MemoryId};

// ---------------------------------------------------------------------------
// HnswPoint — adapter from Embedding to instant-distance Point trait
// ---------------------------------------------------------------------------

/// A point in the HNSW index, wrapping an `Embedding` for cosine distance.
#[derive(Clone, Debug)]
struct HnswPoint {
    /// The normalized embedding vector.
    normalized: Vec<f32>,
}

impl HnswPoint {
    /// Create from a raw embedding. Normalizes to unit length for cosine distance.
    fn from_embedding(embedding: &Embedding) -> Self {
        let norm = embedding
            .0
            .iter()
            .map(|x| x * x)
            .sum::<f32>()
            .sqrt()
            .max(f32::EPSILON);
        let normalized: Vec<f32> = embedding.0.iter().map(|x| x / norm).collect();
        Self { normalized }
    }
}

impl Point for HnswPoint {
    /// Cosine distance = 1 - `cosine_similarity`.
    /// Since vectors are pre-normalized, `cosine_similarity` = dot product.
    fn distance(&self, other: &Self) -> f32 {
        if self.normalized.len() != other.normalized.len() {
            return 1.0; // Maximum distance for mismatched dimensions
        }
        let dot: f32 = self
            .normalized
            .iter()
            .zip(other.normalized.iter())
            .map(|(a, b)| a * b)
            .sum();
        (1.0 - dot).max(0.0) // Clamp to [0, 2] → typically [0, 1]
    }
}

// ---------------------------------------------------------------------------
// Search Results
// ---------------------------------------------------------------------------

/// A single search result from the HNSW index.
#[derive(Debug, Clone)]
pub struct HnswResult {
    /// The memory ID of the matching embedding.
    pub memory_id: MemoryId,
    /// Cosine distance (0.0 = identical, 1.0 = orthogonal, 2.0 = opposite).
    pub distance: f32,
    /// Cosine similarity (1.0 - distance), range [-1.0, 1.0].
    pub similarity: f32,
}

// ---------------------------------------------------------------------------
// HnswIndex — incremental insert + batch-build + search
// ---------------------------------------------------------------------------

/// HNSW-based approximate nearest-neighbor index for memory embeddings.
///
/// ## Lifecycle
///
/// 1. **Insert** — Add embeddings with their memory IDs via [`insert`].
/// 2. **Build** — Call [`build`] to construct the HNSW graph (O(N log N)).
/// 3. **Search** — Query nearest neighbors via [`search`].
///
/// Insertions after build require re-building (incremental rebuild is
/// amortized: only rebuild when dirty count exceeds threshold).
pub struct HnswIndex {
    /// Pending (not yet indexed) points.
    pending_points: Vec<HnswPoint>,
    /// Pending values (`MemoryId`).
    pending_values: Vec<MemoryId>,
    /// Built HNSW map (None until `build()` is called).
    map: Option<HnswMap<HnswPoint, MemoryId>>,
    /// Number of inserts since last build.
    dirty_count: usize,
    /// `ef_construction` parameter (higher = more accurate build, slower).
    ef_construction: usize,
    /// `ef_search` parameter (higher = more accurate search, slower).
    ef_search: usize,
    /// Threshold: auto-rebuild if `dirty_count` exceeds this fraction of total.
    auto_rebuild_threshold: f32,
}

impl HnswIndex {
    /// Create a new empty HNSW index with default parameters.
    #[must_use]
    pub fn new() -> Self {
        Self {
            pending_points: Vec::new(),
            pending_values: Vec::new(),
            map: None,
            dirty_count: 0,
            ef_construction: 100,
            ef_search: 50,
            auto_rebuild_threshold: 0.2, // Rebuild when 20% new points
        }
    }

    /// Create with custom HNSW parameters.
    #[must_use]
    pub fn with_params(ef_construction: usize, ef_search: usize) -> Self {
        Self {
            ef_construction,
            ef_search,
            ..Self::new()
        }
    }

    /// Insert a memory embedding into the index.
    ///
    /// The embedding is queued; call [`build`] to index it.
    pub fn insert(&mut self, memory_id: MemoryId, embedding: Embedding) {
        self.pending_points
            .push(HnswPoint::from_embedding(&embedding));
        self.pending_values.push(memory_id);
        self.dirty_count += 1;
    }

    /// Number of points currently in the index (pending + built).
    #[must_use]
    pub fn len(&self) -> usize {
        self.pending_points.len()
    }

    /// Whether the index is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pending_points.is_empty()
    }

    /// Whether the index needs rebuilding (dirty inserts exceed threshold).
    #[must_use]
    pub fn needs_rebuild(&self) -> bool {
        if self.map.is_none() && !self.pending_points.is_empty() {
            return true;
        }
        let total = self.pending_points.len();
        if total == 0 {
            return false;
        }
        (self.dirty_count as f32 / total as f32) > self.auto_rebuild_threshold
    }

    /// Build (or rebuild) the HNSW graph from all pending points.
    ///
    /// This is O(N log N) and should be called during loading or
    /// periodically between frames.
    pub fn build(&mut self) {
        if self.pending_points.is_empty() {
            return;
        }

        let builder = Builder::default()
            .ef_construction(self.ef_construction)
            .ef_search(self.ef_search)
            .seed(42); // Deterministic for reproducibility

        let map = builder.build(
            self.pending_points.clone(),
            self.pending_values.clone(),
        );

        self.map = Some(map);
        self.dirty_count = 0;
    }

    /// Search for the `k` nearest neighbors to the query embedding.
    ///
    /// Returns results sorted by ascending distance (most similar first).
    /// If the index hasn't been built yet, falls back to brute-force.
    #[must_use]
    pub fn search(&self, query: &Embedding, k: usize) -> Vec<HnswResult> {
        let query_point = HnswPoint::from_embedding(query);

        if let Some(map) = &self.map {
            let mut search = Search::default();
            let results: Vec<HnswResult> = map
                .search(&query_point, &mut search)
                .take(k)
                .map(|item| HnswResult {
                    memory_id: *item.value,
                    distance: item.distance,
                    similarity: 1.0 - item.distance,
                })
                .collect();
            results
        } else {
            // Brute-force fallback when not built
            self.brute_force_search(&query_point, k)
        }
    }

    /// Brute-force linear scan (used when index is not yet built).
    fn brute_force_search(&self, query: &HnswPoint, k: usize) -> Vec<HnswResult> {
        let mut scored: Vec<(f32, usize)> = self
            .pending_points
            .iter()
            .enumerate()
            .map(|(i, point)| (query.distance(point), i))
            .collect();

        scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(k);

        scored
            .into_iter()
            .map(|(dist, idx)| HnswResult {
                memory_id: self.pending_values[idx],
                distance: dist,
                similarity: 1.0 - dist,
            })
            .collect()
    }

    /// Remove a memory from the index by ID.
    ///
    /// Note: this marks the entry for removal on next rebuild.
    /// The HNSW graph is not modified in-place (instant-distance is immutable).
    pub fn remove(&mut self, memory_id: MemoryId) {
        let mut i = 0;
        while i < self.pending_values.len() {
            if self.pending_values[i] == memory_id {
                self.pending_values.swap_remove(i);
                self.pending_points.swap_remove(i);
                self.dirty_count += 1;
                // Don't increment i — swapped element now at position i
            } else {
                i += 1;
            }
        }
    }

    /// Clear the entire index.
    pub fn clear(&mut self) {
        self.pending_points.clear();
        self.pending_values.clear();
        self.map = None;
        self.dirty_count = 0;
    }

    /// Get index statistics for debugging.
    #[must_use]
    pub fn stats(&self) -> HnswStats {
        HnswStats {
            total_points: self.pending_points.len(),
            dirty_count: self.dirty_count,
            is_built: self.map.is_some(),
            ef_construction: self.ef_construction,
            ef_search: self.ef_search,
        }
    }
}

impl Default for HnswIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the HNSW index state.
#[derive(Debug, Clone)]
pub struct HnswStats {
    /// Total number of indexed points.
    pub total_points: usize,
    /// Number of insertions since last build.
    pub dirty_count: usize,
    /// Whether the HNSW graph has been built.
    pub is_built: bool,
    /// `ef_construction` parameter.
    pub ef_construction: usize,
    /// `ef_search` parameter.
    pub ef_search: usize,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_embedding(values: &[f32]) -> Embedding {
        Embedding(values.to_vec())
    }

    #[test]
    fn empty_index_returns_no_results() {
        let index = HnswIndex::new();
        let results = index.search(&make_embedding(&[1.0, 0.0, 0.0]), 5);
        assert!(results.is_empty());
    }

    #[test]
    fn insert_and_brute_force_search() {
        let mut index = HnswIndex::new();

        let id1 = MemoryId::new();
        let id2 = MemoryId::new();
        let id3 = MemoryId::new();

        index.insert(id1, make_embedding(&[1.0, 0.0, 0.0]));
        index.insert(id2, make_embedding(&[0.0, 1.0, 0.0]));
        index.insert(id3, make_embedding(&[0.9, 0.1, 0.0]));

        // Without building, should fall back to brute-force
        let results = index.search(&make_embedding(&[1.0, 0.0, 0.0]), 2);
        assert_eq!(results.len(), 2);
        // Most similar should be id1 (exact match) or id3 (close)
        assert!(results[0].similarity > 0.9);
    }

    #[test]
    fn build_and_search() {
        let mut index = HnswIndex::new();

        let ids: Vec<MemoryId> = (0..50).map(|_| MemoryId::new()).collect();
        for (i, &id) in ids.iter().enumerate() {
            let angle = (i as f32 / 50.0) * std::f32::consts::TAU;
            index.insert(id, make_embedding(&[angle.cos(), angle.sin(), 0.0]));
        }

        index.build();
        assert!(!index.needs_rebuild());

        let results = index.search(&make_embedding(&[1.0, 0.0, 0.0]), 5);
        assert_eq!(results.len(), 5);
        // First result should be close to (1, 0, 0)
        assert!(results[0].similarity > 0.95, "Top result sim={}", results[0].similarity);
    }

    #[test]
    fn needs_rebuild_after_inserts() {
        let mut index = HnswIndex::new();
        assert!(!index.needs_rebuild()); // Empty

        index.insert(MemoryId::new(), make_embedding(&[1.0, 0.0]));
        assert!(index.needs_rebuild()); // Never built

        index.build();
        assert!(!index.needs_rebuild());

        // Insert 20% more → should trigger rebuild
        // We have 1 point, so threshold = 0.2 * 2 = 0.4; 1 dirty > 0.4
        index.insert(MemoryId::new(), make_embedding(&[0.0, 1.0]));
        assert!(index.needs_rebuild());
    }

    #[test]
    fn remove_works() {
        let mut index = HnswIndex::new();

        let id1 = MemoryId::new();
        let id2 = MemoryId::new();
        index.insert(id1, make_embedding(&[1.0, 0.0]));
        index.insert(id2, make_embedding(&[0.0, 1.0]));

        assert_eq!(index.len(), 2);
        index.remove(id1);
        assert_eq!(index.len(), 1);
        assert_eq!(index.pending_values[0], id2);
    }

    #[test]
    fn clear_resets_everything() {
        let mut index = HnswIndex::new();
        for _ in 0..10 {
            index.insert(MemoryId::new(), make_embedding(&[1.0, 0.0]));
        }
        index.build();
        assert!(index.stats().is_built);

        index.clear();
        assert!(index.is_empty());
        assert!(!index.stats().is_built);
    }

    #[test]
    fn cosine_distance_identity() {
        let a = HnswPoint::from_embedding(&make_embedding(&[1.0, 0.0, 0.0]));
        let dist = a.distance(&a);
        assert!(dist < 0.001, "Self-distance should be ~0, got {dist}");
    }

    #[test]
    fn cosine_distance_orthogonal() {
        let a = HnswPoint::from_embedding(&make_embedding(&[1.0, 0.0, 0.0]));
        let b = HnswPoint::from_embedding(&make_embedding(&[0.0, 1.0, 0.0]));
        let dist = a.distance(&b);
        assert!(
            (dist - 1.0).abs() < 0.01,
            "Orthogonal vectors should have distance ~1.0, got {dist}"
        );
    }

    #[test]
    fn stats_reports_correctly() {
        let mut index = HnswIndex::with_params(200, 100);
        assert_eq!(index.stats().ef_construction, 200);
        assert_eq!(index.stats().ef_search, 100);
        assert_eq!(index.stats().total_points, 0);
        assert!(!index.stats().is_built);

        index.insert(MemoryId::new(), make_embedding(&[1.0, 0.0]));
        assert_eq!(index.stats().total_points, 1);
        assert_eq!(index.stats().dirty_count, 1);

        index.build();
        assert!(index.stats().is_built);
        assert_eq!(index.stats().dirty_count, 0);
    }

    #[test]
    fn large_index_search() {
        let mut index = HnswIndex::new();

        // Insert 500 random-ish embeddings
        for i in 0..500u32 {
            let v1 = (i as f32 * 0.017).sin();
            let v2 = (i as f32 * 0.031).cos();
            let v3 = (i as f32 * 0.053).sin();
            index.insert(MemoryId::new(), make_embedding(&[v1, v2, v3]));
        }

        index.build();
        let results = index.search(&make_embedding(&[0.5, 0.5, 0.5]), 10);
        assert_eq!(results.len(), 10);

        // Results should be sorted by distance (ascending)
        for window in results.windows(2) {
            assert!(
                window[0].distance <= window[1].distance + 0.001,
                "Results should be sorted by distance"
            );
        }
    }
}
