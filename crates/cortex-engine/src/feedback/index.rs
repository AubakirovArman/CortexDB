use std::collections::BTreeMap;

use cortex_core::memtable::{MemTable, ReadTxn};
use cortex_core::CellId;

use super::{
    FeedbackScoreReport, FeedbackStats, FEEDBACK_DECAY_WINDOW_SECONDS, FEEDBACK_FULL_VOTE_BONUS,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct FeedbackIndex {
    records: BTreeMap<CellId, FeedbackRecord>,
}

impl FeedbackIndex {
    pub(crate) fn from_memtable(memtable: &MemTable, txn: ReadTxn) -> Self {
        let records = memtable
            .visible_iter(txn)
            .filter_map(|version| {
                feedback_record(&version.payload).map(|record| (version.cell_id, record))
            })
            .collect();
        Self { records }
    }

    pub(crate) fn apply_record(&mut self, cell_id: CellId, record: Option<FeedbackRecord>) {
        if let Some(record) = record {
            self.records.insert(cell_id, record);
        } else {
            self.records.remove(&cell_id);
        }
    }

    pub(crate) fn record_from_payload(payload: &[u8]) -> Option<FeedbackRecord> {
        feedback_record(payload)
    }

    pub(crate) fn apply_tombstone(&mut self, cell_id: CellId) {
        self.records.remove(&cell_id);
    }

    pub(crate) fn scores(&self) -> BTreeMap<CellId, i32> {
        let mut scores = BTreeMap::<CellId, i32>::new();
        for record in self.records.values() {
            let delta = if record.useful { 1 } else { -1 };
            *scores.entry(record.source_cell_id).or_default() += delta;
        }
        scores
    }

    pub(crate) fn scores_at(&self, now_unix_seconds: u64) -> BTreeMap<CellId, i32> {
        let mut scores = BTreeMap::<CellId, i32>::new();
        for record in self.records.values() {
            *scores.entry(record.source_cell_id).or_default() +=
                decayed_feedback_contribution(record, now_unix_seconds);
        }
        scores
    }

    pub(crate) fn score_report_at(&self, now_unix_seconds: u64) -> Vec<FeedbackScoreReport> {
        let mut reports = BTreeMap::<CellId, FeedbackScoreReport>::new();
        for record in self.records.values() {
            let report =
                reports
                    .entry(record.source_cell_id)
                    .or_insert_with(|| FeedbackScoreReport {
                        source_cell_id: record.source_cell_id,
                        useful: 0,
                        not_useful: 0,
                        raw_score: 0,
                        decayed_score: 0,
                        decay_window_seconds: FEEDBACK_DECAY_WINDOW_SECONDS,
                    });
            if record.useful {
                report.useful += 1;
                report.raw_score += 1;
            } else {
                report.not_useful += 1;
                report.raw_score -= 1;
            }
            report.decayed_score += decayed_feedback_contribution(record, now_unix_seconds);
        }
        reports.into_values().collect()
    }

    pub(crate) fn stats(&self) -> FeedbackStats {
        let mut stats = FeedbackStats::default();
        for record in self.records.values() {
            stats.total += 1;
            let cell_stats = stats
                .by_source_cell
                .entry(record.source_cell_id)
                .or_default();
            if record.useful {
                stats.useful += 1;
                cell_stats.useful += 1;
                cell_stats.score += 1;
            } else {
                stats.not_useful += 1;
                cell_stats.not_useful += 1;
                cell_stats.score -= 1;
            }
        }
        stats
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FeedbackRecord {
    source_cell_id: CellId,
    useful: bool,
    created_unix_seconds: Option<u64>,
}

pub(crate) fn feedback_record(payload: &[u8]) -> Option<FeedbackRecord> {
    let text = String::from_utf8_lossy(payload);
    let mut is_feedback = false;
    let mut source_cell_id = None;
    let mut useful = None;
    let mut created_unix_seconds = None;
    for line in text.lines() {
        if line.trim() == "type=feedback" {
            is_feedback = true;
        } else if let Some(value) = line.strip_prefix("source_cell_id=") {
            source_cell_id = value.trim().parse::<u64>().ok().map(CellId);
        } else if let Some(value) = line.strip_prefix("useful=") {
            useful = match value.trim() {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            };
        } else if let Some(value) = line.strip_prefix("created_unix_seconds=") {
            created_unix_seconds = value.trim().parse::<u64>().ok();
        }
    }
    is_feedback.then_some(FeedbackRecord {
        source_cell_id: source_cell_id?,
        useful: useful?,
        created_unix_seconds,
    })
}

fn decayed_feedback_contribution(record: &FeedbackRecord, now_unix_seconds: u64) -> i32 {
    let sign = if record.useful { 1 } else { -1 };
    let Some(created) = record.created_unix_seconds else {
        return sign * FEEDBACK_FULL_VOTE_BONUS;
    };
    let age = now_unix_seconds.saturating_sub(created);
    if age >= FEEDBACK_DECAY_WINDOW_SECONDS {
        return 0;
    }
    let remaining = FEEDBACK_DECAY_WINDOW_SECONDS - age;
    let magnitude = ((i128::from(FEEDBACK_FULL_VOTE_BONUS) * i128::from(remaining))
        + i128::from(FEEDBACK_DECAY_WINDOW_SECONDS / 2))
        / i128::from(FEEDBACK_DECAY_WINDOW_SECONDS);
    sign * i32::try_from(magnitude).unwrap_or(FEEDBACK_FULL_VOTE_BONUS)
}
