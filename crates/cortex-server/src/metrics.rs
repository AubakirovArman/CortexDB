use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use crate::responses::LatencyHistogramResponse;

#[derive(Clone, Debug)]
pub(crate) struct LatencyHistogram {
    count: Arc<AtomicU64>,
    sum_ms: Arc<AtomicU64>,
    le_10_ms: Arc<AtomicU64>,
    le_50_ms: Arc<AtomicU64>,
    le_100_ms: Arc<AtomicU64>,
    le_500_ms: Arc<AtomicU64>,
    le_1000_ms: Arc<AtomicU64>,
    gt_1000_ms: Arc<AtomicU64>,
}

impl LatencyHistogram {
    pub(crate) fn new() -> Self {
        Self {
            count: Arc::new(AtomicU64::new(0)),
            sum_ms: Arc::new(AtomicU64::new(0)),
            le_10_ms: Arc::new(AtomicU64::new(0)),
            le_50_ms: Arc::new(AtomicU64::new(0)),
            le_100_ms: Arc::new(AtomicU64::new(0)),
            le_500_ms: Arc::new(AtomicU64::new(0)),
            le_1000_ms: Arc::new(AtomicU64::new(0)),
            gt_1000_ms: Arc::new(AtomicU64::new(0)),
        }
    }

    pub(crate) fn observe_ms(&self, latency_ms: u64) {
        self.count.fetch_add(1, Ordering::Relaxed);
        self.sum_ms.fetch_add(latency_ms, Ordering::Relaxed);
        if latency_ms <= 10 {
            self.le_10_ms.fetch_add(1, Ordering::Relaxed);
        }
        if latency_ms <= 50 {
            self.le_50_ms.fetch_add(1, Ordering::Relaxed);
        }
        if latency_ms <= 100 {
            self.le_100_ms.fetch_add(1, Ordering::Relaxed);
        }
        if latency_ms <= 500 {
            self.le_500_ms.fetch_add(1, Ordering::Relaxed);
        }
        if latency_ms <= 1000 {
            self.le_1000_ms.fetch_add(1, Ordering::Relaxed);
        } else {
            self.gt_1000_ms.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub(crate) fn snapshot(&self) -> LatencyHistogramResponse {
        LatencyHistogramResponse {
            count: self.count.load(Ordering::Relaxed),
            sum_ms: self.sum_ms.load(Ordering::Relaxed),
            le_10_ms: self.le_10_ms.load(Ordering::Relaxed),
            le_50_ms: self.le_50_ms.load(Ordering::Relaxed),
            le_100_ms: self.le_100_ms.load(Ordering::Relaxed),
            le_500_ms: self.le_500_ms.load(Ordering::Relaxed),
            le_1000_ms: self.le_1000_ms.load(Ordering::Relaxed),
            gt_1000_ms: self.gt_1000_ms.load(Ordering::Relaxed),
        }
    }
}

impl Default for LatencyHistogram {
    fn default() -> Self {
        Self::new()
    }
}
