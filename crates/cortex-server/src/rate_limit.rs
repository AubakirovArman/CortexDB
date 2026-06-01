use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(60);

#[derive(Clone, Debug)]
pub(crate) struct GlobalRateLimit {
    state: Arc<Mutex<RollingBudgetState>>,
}

impl GlobalRateLimit {
    pub(crate) fn new(limit: u64) -> Self {
        Self {
            state: Arc::new(Mutex::new(RollingBudgetState::new(limit))),
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
    states: Arc<Mutex<BTreeMap<String, PrincipalQuotaState>>>,
}

impl PrincipalRateLimits {
    pub(crate) fn allow(&self, principal_id: &str, limit: u64) -> bool {
        let Ok(mut states) = self.states.lock() else {
            return false;
        };
        states
            .entry(principal_id.to_owned())
            .or_default()
            .requests
            .allow_with_limit(limit, Instant::now())
    }

    pub(crate) fn allow_body_bytes(&self, principal_id: &str, limit: u64, bytes: u64) -> bool {
        let Ok(mut states) = self.states.lock() else {
            return false;
        };
        states
            .entry(principal_id.to_owned())
            .or_default()
            .body_bytes
            .allow_amount_with_limit(limit, bytes, Instant::now())
    }

    pub(crate) fn acquire_queue(
        &self,
        principal_id: &str,
        limit: u64,
    ) -> Option<PrincipalQueuePermit> {
        let Ok(mut states) = self.states.lock() else {
            return None;
        };
        let state = states.entry(principal_id.to_owned()).or_default();
        if state.queue_in_flight >= limit {
            return None;
        }
        state.queue_in_flight += 1;
        Some(PrincipalQueuePermit {
            limits: self.clone(),
            principal_id: principal_id.to_owned(),
        })
    }

    fn release_queue(&self, principal_id: &str) {
        let Ok(mut states) = self.states.lock() else {
            return;
        };
        let Some(state) = states.get_mut(principal_id) else {
            return;
        };
        state.queue_in_flight = state.queue_in_flight.saturating_sub(1);
    }
}

pub(crate) struct PrincipalQueuePermit {
    limits: PrincipalRateLimits,
    principal_id: String,
}

impl Drop for PrincipalQueuePermit {
    fn drop(&mut self) {
        self.limits.release_queue(&self.principal_id);
    }
}

#[derive(Debug, Default)]
struct PrincipalQuotaState {
    requests: RollingBudgetState,
    body_bytes: RollingBudgetState,
    queue_in_flight: u64,
}

#[derive(Debug)]
struct RollingBudgetState {
    limit: u64,
    window_started: Instant,
    used: u64,
}

impl RollingBudgetState {
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
        self.allow_amount_with_limit(limit, 1, now)
    }

    fn allow_amount_with_limit(&mut self, limit: u64, amount: u64, now: Instant) -> bool {
        self.limit = limit;
        if now.duration_since(self.window_started) >= RATE_LIMIT_WINDOW {
            self.window_started = now;
            self.used = 0;
        }
        if amount > self.limit || self.used.saturating_add(amount) > self.limit {
            return false;
        }
        self.used += amount;
        true
    }
}

impl Default for RollingBudgetState {
    fn default() -> Self {
        Self::new(0)
    }
}

#[cfg(test)]
mod tests {
    use super::PrincipalRateLimits;

    #[test]
    fn body_byte_budget_counts_bytes_per_principal() {
        let limits = PrincipalRateLimits::default();

        assert!(limits.allow_body_bytes("a", 5, 3));
        assert!(limits.allow_body_bytes("a", 5, 2));
        assert!(!limits.allow_body_bytes("a", 5, 1));
        assert!(limits.allow_body_bytes("b", 5, 5));
    }

    #[test]
    fn queue_budget_rejects_when_in_flight_limit_is_reached() {
        let limits = PrincipalRateLimits::default();

        let first = limits.acquire_queue("a", 1).expect("first permit");
        assert!(limits.acquire_queue("a", 1).is_none());
        assert!(limits.acquire_queue("b", 1).is_some());

        drop(first);
        assert!(limits.acquire_queue("a", 1).is_some());
    }
}
