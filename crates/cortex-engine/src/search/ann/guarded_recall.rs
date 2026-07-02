//! A3.3: deterministic sampled guarded-recall.
//!
//! To keep ANN serving honest without paying a full exact scan on every query,
//! recall is measured on a deterministic *sample* of queries and tracked in a
//! bounded window. When the windowed recall drops below a floor, the caller can
//! fall back to exact serving until the next rebuild. Everything here is
//! deterministic — no wall clock, no RNG — so the sampling decision and the
//! degradation verdict are reproducible and (in a follow-on) receipt-visible.

/// Steady-state sample rate: measure recall on 1 in every N queries.
pub const GUARDED_RECALL_SAMPLE_RATE: u64 = 8;
/// Warm-up: always measure the first N queries after a graph rebuild, so a bad
/// rebuild is caught immediately instead of after the sampler happens to fire.
pub const GUARDED_RECALL_WARMUP_QUERIES: u64 = 32;
/// Number of recent recall observations retained in the window.
pub const GUARDED_RECALL_WINDOW: usize = 64;

/// Decides, deterministically and without any wall clock or RNG, whether a query
/// should be sampled for recall measurement. The first `WARMUP_QUERIES` after a
/// rebuild are always sampled; afterwards a query is sampled iff
/// `hash(query_bytes, generation) % SAMPLE_RATE == 0`. `generation` is the
/// manifest generation, so the sample set changes deterministically across
/// rebuilds and is byte-stable within one.
pub fn should_sample_recall(
    query_bytes: &[u8],
    generation: u64,
    queries_since_rebuild: u64,
) -> bool {
    if queries_since_rebuild < GUARDED_RECALL_WARMUP_QUERIES {
        return true;
    }
    sampling_hash(query_bytes, generation).is_multiple_of(GUARDED_RECALL_SAMPLE_RATE)
}

/// FNV-1a-64 over the manifest generation followed by the query bytes.
fn sampling_hash(query_bytes: &[u8], generation: u64) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET;
    for byte in generation.to_le_bytes() {
        hash = (hash ^ u64::from(byte)).wrapping_mul(PRIME);
    }
    for &byte in query_bytes {
        hash = (hash ^ u64::from(byte)).wrapping_mul(PRIME);
    }
    hash
}

/// A bounded ring buffer of recent recall_q16 observations. Deterministic and
/// allocation-free; the windowed minimum drives the degradation decision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecallWindow {
    values: [u16; GUARDED_RECALL_WINDOW],
    len: usize,
    next: usize,
}

impl Default for RecallWindow {
    fn default() -> Self {
        Self {
            values: [u16::MAX; GUARDED_RECALL_WINDOW],
            len: 0,
            next: 0,
        }
    }
}

impl RecallWindow {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records one sampled recall observation, evicting the oldest once full.
    pub fn push(&mut self, recall_q16: u16) {
        self.values[self.next] = recall_q16;
        self.next = (self.next + 1) % GUARDED_RECALL_WINDOW;
        self.len = (self.len + 1).min(GUARDED_RECALL_WINDOW);
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The minimum recall over the current window, or `None` when empty.
    pub fn windowed_min(&self) -> Option<u16> {
        self.values.iter().take(self.len).copied().min()
    }

    /// True when the window holds at least one observation and its minimum recall
    /// is below `floor_q16` — the signal to fall back to exact serving.
    pub fn is_degraded(&self, floor_q16: u16) -> bool {
        self.windowed_min().is_some_and(|min| min < floor_q16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warmup_queries_are_always_sampled() {
        for query_since_rebuild in 0..GUARDED_RECALL_WARMUP_QUERIES {
            assert!(should_sample_recall(b"any query", 7, query_since_rebuild));
        }
    }

    #[test]
    fn steady_state_samples_deterministically_at_rate() {
        let generation = 42;
        let start = GUARDED_RECALL_WARMUP_QUERIES;
        let mut sampled = 0usize;
        let total = 8_000usize;
        for i in 0..total {
            let query = format!("query-{i}");
            if should_sample_recall(query.as_bytes(), generation, start + i as u64) {
                sampled += 1;
            }
        }
        // ~1/8 sampled; allow a generous band around the expectation.
        let expected = total / GUARDED_RECALL_SAMPLE_RATE as usize;
        assert!(
            (sampled as i64 - expected as i64).unsigned_abs() < (expected as u64) / 2,
            "sampled {sampled}, expected ~{expected}"
        );
        // Deterministic: same inputs -> same decision.
        assert_eq!(
            should_sample_recall(b"query-1", generation, start),
            should_sample_recall(b"query-1", generation, start)
        );
    }

    #[test]
    fn generation_changes_the_sample_set() {
        // A rebuild (new generation) reshuffles which queries are sampled.
        let query = b"stable query bytes";
        let start = GUARDED_RECALL_WARMUP_QUERIES;
        let a: Vec<bool> = (0..64)
            .map(|g| should_sample_recall(query, g, start))
            .collect();
        let b: Vec<bool> = (100..164)
            .map(|g| should_sample_recall(query, g, start))
            .collect();
        assert_ne!(a, b, "different generations must not sample identically");
    }

    #[test]
    fn recall_window_tracks_min_and_degradation() {
        let mut window = RecallWindow::new();
        assert!(window.is_empty());
        assert_eq!(window.windowed_min(), None);
        assert!(!window.is_degraded(40_000));

        window.push(60_000);
        window.push(55_000);
        window.push(30_000);
        assert_eq!(window.windowed_min(), Some(30_000));
        assert!(window.is_degraded(40_000), "min 30000 < floor 40000");
        assert!(!window.is_degraded(20_000));
    }

    #[test]
    fn recall_window_evicts_oldest_when_full() {
        let mut window = RecallWindow::new();
        // Fill with a low value, then push it out with high values.
        window.push(1);
        for _ in 0..GUARDED_RECALL_WINDOW {
            window.push(65_000);
        }
        assert_eq!(window.len(), GUARDED_RECALL_WINDOW);
        assert_eq!(
            window.windowed_min(),
            Some(65_000),
            "the low observation must have been evicted"
        );
    }
}
