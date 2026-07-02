//! Embedder adapter seam (A2.1).
//!
//! The engine stays network-free: it defines the [`Embedder`] trait as the
//! integration point for turning cell/query text into fixed-point `i16` vectors,
//! and a caller (e.g. the server, which owns the HTTP client) injects a
//! network-backed implementation. [`DeterministicTestEmbedder`] is an offline,
//! reproducible backend for hermetic gates — identical input always yields the
//! identical vector, so embedding-dependent tests stay deterministic.

use crate::error::EngineResult;

/// A backend that embeds text into fixed-point (`i16`) vectors.
///
/// Implementations must preserve input order (`embed_batch(texts)[i]` is the
/// vector for `texts[i]`) and return vectors of length [`Embedder::dimension`].
pub trait Embedder: Send + Sync {
    /// Embedding dimensionality produced by this backend.
    fn dimension(&self) -> usize;

    /// Embed a batch of texts into `i16` vectors, one per input, in order.
    fn embed_batch(&self, texts: &[&str]) -> EngineResult<Vec<Vec<i16>>>;

    /// Convenience helper for embedding a single text.
    fn embed(&self, text: &str) -> EngineResult<Vec<i16>> {
        Ok(self
            .embed_batch(&[text])?
            .into_iter()
            .next()
            .unwrap_or_default())
    }
}

/// Deterministic, offline embedder for hermetic tests and gates.
///
/// Each text maps to a stable vector derived from a content hash, so identical
/// text yields identical vectors while different text (almost surely) differs.
/// It performs no I/O and never fails.
#[derive(Clone, Copy, Debug)]
pub struct DeterministicTestEmbedder {
    dimension: usize,
}

impl DeterministicTestEmbedder {
    /// Creates a deterministic embedder producing vectors of `dimension` lanes
    /// (clamped to at least 1).
    pub fn new(dimension: usize) -> Self {
        Self {
            dimension: dimension.max(1),
        }
    }
}

impl Embedder for DeterministicTestEmbedder {
    fn dimension(&self) -> usize {
        self.dimension
    }

    fn embed_batch(&self, texts: &[&str]) -> EngineResult<Vec<Vec<i16>>> {
        Ok(texts
            .iter()
            .map(|text| deterministic_vector(text, self.dimension))
            .collect())
    }
}

fn deterministic_vector(text: &str, dimension: usize) -> Vec<i16> {
    // Seed a splitmix64 stream from an FNV-1a64 hash of the text so the same
    // text always produces the same lanes across runs and platforms.
    let mut state = fnv1a64(text.as_bytes());
    (0..dimension)
        .map(|_| {
            state = splitmix64(state);
            // Take the high 16 bits for a well-mixed value across the i16 range.
            (state >> 48) as u16 as i16
        })
        .collect()
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325;
    for &byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    // Avoid a zero seed so an empty string still yields a mixed stream.
    hash | 1
}

fn splitmix64(state: u64) -> u64 {
    let mut z = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimension_is_reported_and_clamped() {
        assert_eq!(DeterministicTestEmbedder::new(8).dimension(), 8);
        assert_eq!(DeterministicTestEmbedder::new(0).dimension(), 1);
    }

    #[test]
    fn embedding_is_deterministic_and_correctly_sized() {
        let embedder = DeterministicTestEmbedder::new(16);
        let batch = embedder
            .embed_batch(&["solar plant budget", "wind farm"])
            .unwrap();
        assert_eq!(batch.len(), 2);
        assert_eq!(batch[0].len(), 16);
        // Same text -> identical vector.
        assert_eq!(batch[0], embedder.embed("solar plant budget").unwrap());
        // Different text -> different vector.
        assert_ne!(batch[0], batch[1]);
        // A real (non-degenerate) vector.
        assert!(batch[0].iter().any(|&value| value != 0));
    }

    #[test]
    fn empty_batch_yields_empty_result() {
        let embedder = DeterministicTestEmbedder::new(4);
        assert!(embedder.embed_batch(&[]).unwrap().is_empty());
    }
}
