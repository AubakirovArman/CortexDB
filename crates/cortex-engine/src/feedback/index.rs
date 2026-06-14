use std::collections::{BTreeMap, BTreeSet};

use cortex_core::memtable::{MemTable, ReadTxn};
use cortex_core::CellId;

use super::{
    FeedbackScoreReport, FeedbackStats, FEEDBACK_DECAY_WINDOW_SECONDS, FEEDBACK_FULL_VOTE_BONUS,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct FeedbackIndex {
    records: BTreeMap<CellId, FeedbackRecord>,
    records_by_source: BTreeMap<CellId, BTreeSet<CellId>>,
    raw_scores: BTreeMap<CellId, i32>,
    stats: FeedbackStats,
}

impl FeedbackIndex {
    pub(crate) fn from_memtable(memtable: &MemTable, txn: ReadTxn) -> Self {
        let mut index = Self::default();
        for version in memtable.visible_iter(txn) {
            index.apply_record(version.cell_id, feedback_record(&version.payload));
        }
        index
    }

    pub(crate) fn apply_record(&mut self, cell_id: CellId, record: Option<FeedbackRecord>) {
        self.remove_record(cell_id);
        if let Some(record) = record {
            self.apply_delta(record, 1);
            self.records_by_source
                .entry(record.source_cell_id)
                .or_default()
                .insert(cell_id);
            self.records.insert(cell_id, record);
        }
    }

    pub(crate) fn record_from_payload(payload: &[u8]) -> Option<FeedbackRecord> {
        feedback_record(payload)
    }

    pub(crate) fn apply_tombstone(&mut self, cell_id: CellId) {
        self.remove_record(cell_id);
    }

    pub(crate) fn scores(&self) -> BTreeMap<CellId, i32> {
        self.raw_scores.clone()
    }

    pub(crate) fn scores_at(&self, now_unix_seconds: u64) -> BTreeMap<CellId, i32> {
        self.scores_for_cells_at(self.records_by_source.keys().copied(), now_unix_seconds)
    }

    pub(crate) fn scores_for_cells_at<I>(
        &self,
        cell_ids: I,
        now_unix_seconds: u64,
    ) -> BTreeMap<CellId, i32>
    where
        I: IntoIterator<Item = CellId>,
    {
        let mut scores = BTreeMap::new();
        for cell_id in cell_ids {
            scores.insert(cell_id, self.score_for_cell_at(cell_id, now_unix_seconds));
        }
        scores
    }

    pub(crate) fn score_for_cell_at(&self, cell_id: CellId, now_unix_seconds: u64) -> i32 {
        self.records_by_source
            .get(&cell_id)
            .into_iter()
            .flat_map(|record_ids| record_ids.iter())
            .filter_map(|record_id| self.records.get(record_id))
            .map(|record| decayed_feedback_contribution(record, now_unix_seconds))
            .sum()
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
        self.stats.clone()
    }

    fn remove_record(&mut self, cell_id: CellId) {
        let Some(record) = self.records.remove(&cell_id) else {
            return;
        };
        self.apply_delta(record, -1);
        if let Some(record_ids) = self.records_by_source.get_mut(&record.source_cell_id) {
            record_ids.remove(&cell_id);
            if record_ids.is_empty() {
                self.records_by_source.remove(&record.source_cell_id);
                self.raw_scores.remove(&record.source_cell_id);
            }
        }
    }

    fn apply_delta(&mut self, record: FeedbackRecord, multiplier: i32) {
        let sign = if record.useful { 1 } else { -1 };
        let delta = sign * multiplier;
        let score = self.raw_scores.entry(record.source_cell_id).or_default();
        *score += delta;

        self.stats.total = adjust_u32(self.stats.total, multiplier);
        let source_stats = self
            .stats
            .by_source_cell
            .entry(record.source_cell_id)
            .or_default();
        if record.useful {
            self.stats.useful = adjust_u32(self.stats.useful, multiplier);
            source_stats.useful = adjust_u32(source_stats.useful, multiplier);
        } else {
            self.stats.not_useful = adjust_u32(self.stats.not_useful, multiplier);
            source_stats.not_useful = adjust_u32(source_stats.not_useful, multiplier);
        }
        source_stats.score += delta;
        if source_stats.useful == 0 && source_stats.not_useful == 0 {
            self.stats.by_source_cell.remove(&record.source_cell_id);
        }
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

fn adjust_u32(value: u32, delta: i32) -> u32 {
    if delta >= 0 {
        value.saturating_add(delta as u32)
    } else {
        value.saturating_sub(delta.unsigned_abs())
    }
}
