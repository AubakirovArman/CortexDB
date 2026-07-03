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

/// The serving mode a collection is currently in. `Ann` serves budgeted HNSW
/// results (with the windowed recall attached); `ExactDegraded` has fallen back
/// to exact serving after a recall-floor breach and stays there until a rebuild.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuardedServingMode {
    Ann,
    ExactDegraded,
}

impl GuardedServingMode {
    pub fn as_str(self) -> &'static str {
        match self {
            GuardedServingMode::Ann => "ann",
            GuardedServingMode::ExactDegraded => "exact_degraded",
        }
    }
}

/// Per-collection guarded-recall state machine (A3.3). Ties the deterministic
/// sampling decision, the recall window, the sticky degradation verdict, and a
/// monotonic `serving_epoch` together. Every transition is a pure function of
/// recorded observations + the manifest generation — no wall clock, no RNG — so a
/// replay reproduces the same epoch, and the epoch is what enters the signed
/// determinism surface (see `DeterminismHashInput::serving_epoch` +
/// ADR-ann-degradation-receipt-visibility).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuardedRecallState {
    window: RecallWindow,
    generation: u64,
    queries_since_rebuild: u64,
    serving_epoch: u64,
    degraded: bool,
    // Lifetime telemetry (not persisted; reset on restart). `exact_serves` counts
    // queries that paid an exact scan — sampled recall-recomputes + degraded
    // exact-serving — so `exact_serves / queries` is the exact-scan rate A3.3
    // caps at <=15%.
    lifetime_queries: u64,
    lifetime_exact_serves: u64,
}

impl GuardedRecallState {
    /// A fresh collection at `generation`, serving ANN, epoch 0.
    pub fn new(generation: u64) -> Self {
        Self {
            window: RecallWindow::new(),
            generation,
            queries_since_rebuild: 0,
            serving_epoch: 0,
            degraded: false,
            lifetime_queries: 0,
            lifetime_exact_serves: 0,
        }
    }

    /// Reconstruct persisted state (for manifest round-trips). Lifetime telemetry
    /// counters are runtime-only and start at zero.
    pub fn from_parts(
        window: RecallWindow,
        generation: u64,
        queries_since_rebuild: u64,
        serving_epoch: u64,
        degraded: bool,
    ) -> Self {
        Self {
            window,
            generation,
            queries_since_rebuild,
            serving_epoch,
            degraded,
            lifetime_queries: 0,
            lifetime_exact_serves: 0,
        }
    }

    pub fn lifetime_queries(&self) -> u64 {
        self.lifetime_queries
    }
    pub fn lifetime_exact_serves(&self) -> u64 {
        self.lifetime_exact_serves
    }

    /// The exact-scan rate in basis points (`exact_serves * 10000 / queries`), or 0
    /// before any query. A3.3 caps this at 1500 bps (15%) on a healthy index.
    pub fn exact_serve_rate_bps(&self) -> u64 {
        if self.lifetime_queries == 0 {
            return 0;
        }
        self.lifetime_exact_serves.saturating_mul(10_000) / self.lifetime_queries
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn queries_since_rebuild(&self) -> u64 {
        self.queries_since_rebuild
    }
    pub fn serving_epoch(&self) -> u64 {
        self.serving_epoch
    }
    pub fn window(&self) -> &RecallWindow {
        &self.window
    }

    pub fn serving_mode(&self) -> GuardedServingMode {
        if self.degraded {
            GuardedServingMode::ExactDegraded
        } else {
            GuardedServingMode::Ann
        }
    }

    /// Whether this query should recompute exact recall (be sampled). Once
    /// degraded, every query is served exact anyway, so sampling is moot — but the
    /// warm-up/rate decision still governs when the *window* is refreshed.
    pub fn should_sample(&self, query_bytes: &[u8]) -> bool {
        should_sample_recall(query_bytes, self.generation, self.queries_since_rebuild)
    }

    /// Record a sampled query's measured recall. Degradation is **sticky**: once
    /// the windowed minimum drops below `floor_q16` the collection stays in
    /// `ExactDegraded` until a rebuild, and the ANN→exact transition bumps the
    /// serving epoch exactly once.
    pub fn record_sampled(&mut self, recall_q16: u16, floor_q16: u16) {
        self.window.push(recall_q16);
        if !self.degraded && self.window.is_degraded(floor_q16) {
            self.degraded = true;
            self.serving_epoch += 1;
        }
        self.queries_since_rebuild = self.queries_since_rebuild.saturating_add(1);
        // A sampled query always pays an exact scan.
        self.lifetime_queries = self.lifetime_queries.saturating_add(1);
        self.lifetime_exact_serves = self.lifetime_exact_serves.saturating_add(1);
    }

    /// Record an unsampled query. It skips the exact scan when serving ANN, but a
    /// degraded collection serves exact — so count the exact scan only then.
    pub fn record_unsampled(&mut self) {
        self.queries_since_rebuild = self.queries_since_rebuild.saturating_add(1);
        self.lifetime_queries = self.lifetime_queries.saturating_add(1);
        if self.degraded {
            self.lifetime_exact_serves = self.lifetime_exact_serves.saturating_add(1);
        }
    }

    /// A graph rebuild: clear the window and re-arm sampling at the new
    /// generation. If the collection was degraded, this is an exact→ANN
    /// transition and bumps the serving epoch.
    pub fn on_rebuild(&mut self, new_generation: u64) {
        self.window = RecallWindow::new();
        self.queries_since_rebuild = 0;
        self.generation = new_generation;
        if self.degraded {
            self.degraded = false;
            self.serving_epoch += 1;
        }
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

    const FLOOR: u16 = 40_000;

    // Drives the full serving-mode lifecycle: healthy ANN -> sticky degradation on
    // a recall-floor breach (epoch bump) -> rebuild recovery (epoch bump), and
    // asserts the serving epoch only moves on genuine mode transitions.
    #[test]
    fn state_machine_degrades_stickily_and_recovers_on_rebuild() {
        let mut state = GuardedRecallState::new(1);
        assert_eq!(state.serving_mode(), GuardedServingMode::Ann);
        assert_eq!(state.serving_epoch(), 0);

        // Healthy samples: stay ANN, epoch unchanged.
        state.record_sampled(60_000, FLOOR);
        state.record_sampled(55_000, FLOOR);
        assert_eq!(state.serving_mode(), GuardedServingMode::Ann);
        assert_eq!(state.serving_epoch(), 0);

        // A below-floor observation -> degrade (ANN -> exact), epoch bumps once.
        state.record_sampled(30_000, FLOOR);
        assert_eq!(state.serving_mode(), GuardedServingMode::ExactDegraded);
        assert_eq!(state.serving_epoch(), 1);

        // Sticky: further samples (even healthy ones) do NOT re-bump or recover.
        state.record_sampled(65_000, FLOOR);
        state.record_sampled(65_000, FLOOR);
        assert_eq!(state.serving_mode(), GuardedServingMode::ExactDegraded);
        assert_eq!(state.serving_epoch(), 1);

        // A rebuild clears the window and recovers to ANN (exact -> ANN), epoch bumps.
        state.on_rebuild(2);
        assert_eq!(state.serving_mode(), GuardedServingMode::Ann);
        assert_eq!(state.serving_epoch(), 2);
        assert_eq!(state.queries_since_rebuild(), 0);
        assert!(state.window().is_empty());

        // A rebuild while already healthy is not a mode transition: no epoch bump.
        state.on_rebuild(3);
        assert_eq!(state.serving_epoch(), 2);
    }

    #[test]
    fn state_machine_degrades_within_eight_sampled_queries() {
        // A corrupted graph reports low recall; degradation must latch quickly.
        let mut state = GuardedRecallState::new(1);
        let mut sampled = 0;
        for _ in 0..8 {
            state.record_sampled(10_000, FLOOR); // well below floor
            sampled += 1;
            if state.serving_mode() == GuardedServingMode::ExactDegraded {
                break;
            }
        }
        assert_eq!(state.serving_mode(), GuardedServingMode::ExactDegraded);
        assert!(
            sampled <= 8,
            "degradation must latch within 8 sampled queries"
        );
    }

    #[test]
    fn state_machine_is_deterministic_on_replay() {
        // Same observation stream twice -> identical epoch + mode (double-run proof).
        let run = || {
            let mut state = GuardedRecallState::new(5);
            for (i, recall) in [58_000u16, 61_000, 20_000, 63_000, 64_000]
                .iter()
                .enumerate()
            {
                if state.should_sample(format!("q{i}").as_bytes()) {
                    state.record_sampled(*recall, FLOOR);
                } else {
                    state.record_unsampled();
                }
            }
            (
                state.serving_epoch(),
                state.serving_mode(),
                state.queries_since_rebuild(),
            )
        };
        assert_eq!(run(), run());
    }

    // A3.3 DoD: on a healthy index the exact-scan rate stays <= 15% (1500 bps) —
    // the warm-up (32) plus 1-in-8 steady sampling, amortized over enough queries,
    // versus the pre-A3.3 baseline of an exact scan on 100% of queries (10000 bps).
    #[test]
    fn healthy_index_exact_scan_rate_under_15_percent() {
        let mut state = GuardedRecallState::new(1);
        let n = 5_000u64;
        for i in 0..n {
            let query = format!("query-{i}");
            if state.should_sample(query.as_bytes()) {
                state.record_sampled(60_000, FLOOR); // healthy: never degrades
            } else {
                state.record_unsampled();
            }
        }
        assert_eq!(state.lifetime_queries(), n);
        assert_eq!(state.serving_mode(), GuardedServingMode::Ann);
        assert!(
            state.exact_serve_rate_bps() <= 1_500,
            "exact-scan rate {} bps must be <= 1500 (15%)",
            state.exact_serve_rate_bps()
        );
        // A large, real reduction from the exact-every-query baseline (10000 bps).
        assert!(state.exact_serve_rate_bps() < 2_000);
    }

    // A degraded collection serves exact for every query (rate climbs), until a
    // rebuild recovers ANN serving and the rate falls again.
    #[test]
    fn degraded_then_rebuilt_exact_scan_rate_tracks_serving_mode() {
        let mut state = GuardedRecallState::new(1);
        // Warm-up samples all degrade the window immediately.
        for _ in 0..40 {
            state.record_sampled(5_000, FLOOR);
        }
        assert_eq!(state.serving_mode(), GuardedServingMode::ExactDegraded);
        // While degraded, unsampled queries still serve exact.
        for _ in 0..100 {
            state.record_unsampled();
        }
        assert!(
            state.exact_serve_rate_bps() >= 9_000,
            "a fully-degraded collection serves ~all exact"
        );
        // A rebuild recovers ANN serving; subsequent unsampled queries skip exact.
        state.on_rebuild(2);
        for i in 0..5_000 {
            let query = format!("post-rebuild-{i}");
            if state.should_sample(query.as_bytes()) {
                state.record_sampled(60_000, FLOOR);
            } else {
                state.record_unsampled();
            }
        }
        // The healthy post-rebuild window dominates: rate falls back well under 15%
        // over the lifetime is not guaranteed (the degraded prefix counts), but the
        // collection is serving ANN again.
        assert_eq!(state.serving_mode(), GuardedServingMode::Ann);
    }
}
