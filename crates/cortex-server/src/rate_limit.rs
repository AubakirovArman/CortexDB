use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);

#[derive(Clone, Debug)]
pub(crate) struct GlobalRateLimit {
    state: Arc<Mutex<RateLimitState>>,
}

impl GlobalRateLimit {
    pub(crate) fn new(limit: u64) -> Self {
        Self {
            state: Arc::new(Mutex::new(RateLimitState::new(limit))),
        }
    }

    pub(crate) fn allow(&self) -> bool {
        let Ok(mut state) = self.state.lock() else {
            return false;
        };
        state.allow(Instant::now())
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct PrincipalRateLimits {
    states: Arc<Mutex<BTreeMap<String, RateLimitState>>>,
}

impl PrincipalRateLimits {
    pub(crate) fn allow(&self, principal_id: &str, limit: u64) -> bool {
        let Ok(mut states) = self.states.lock() else {
            return false;
        };
        states
            .entry(principal_id.to_owned())
            .or_insert_with(|| RateLimitState::new(limit))
            .allow_with_limit(limit, Instant::now())
    }
}

#[derive(Debug)]
struct RateLimitState {
    limit: u64,
    window_started: Instant,
    used: u64,
}

impl RateLimitState {
    fn new(limit: u64) -> Self {
        Self {
            limit,
            window_started: Instant::now(),
            used: 0,
        }
    }

    fn allow(&mut self, now: Instant) -> bool {
        self.allow_with_limit(self.limit, now)
    }

    fn allow_with_limit(&mut self, limit: u64, now: Instant) -> bool {
        self.limit = limit;
        if now.duration_since(self.window_started) >= RATE_LIMIT_WINDOW {
            self.window_started = now;
            self.used = 0;
        }
        if self.used >= self.limit {
            return false;
        }
        self.used += 1;
        true
    }
}
