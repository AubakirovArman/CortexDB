use std::collections::BTreeMap;

use cortex_aql::AgentId;
use cortex_core::{CellId, CommitSeq, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};

use crate::cell_ids::{agent_cell_id_slot, namespaced_agent_cell_id};
use crate::database::Database;
use crate::error::{EngineError, EngineResult};

mod index;

pub(crate) use index::FeedbackIndex;

const FEEDBACK_CELL_NAMESPACE: u64 = 0x9000_0000_0000_0000;
pub const FEEDBACK_DECAY_WINDOW_SECONDS: u64 = 30 * 24 * 60 * 60;
pub const FEEDBACK_FULL_VOTE_BONUS: i32 = 5_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextFeedback {
    pub source_cell_id: CellId,
    pub useful: bool,
    pub note: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoredFeedback {
    pub cell_id: CellId,
    pub commit_seq: CommitSeq,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FeedbackStats {
    pub total: u32,
    pub useful: u32,
    pub not_useful: u32,
    pub by_source_cell: BTreeMap<CellId, FeedbackCellStats>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FeedbackCellStats {
    pub useful: u32,
    pub not_useful: u32,
    pub score: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FeedbackScoreReport {
    pub source_cell_id: CellId,
    pub useful: u32,
    pub not_useful: u32,
    pub raw_score: i32,
    pub decayed_score: i32,
    pub decay_window_seconds: u64,
}

impl Database {
    pub fn record_context_feedback(
        &mut self,
        agent_id: AgentId,
        feedback: ContextFeedback,
    ) -> EngineResult<StoredFeedback> {
        let cell_id = self.next_feedback_cell_id(agent_id)?;
        let cell = KnowledgeCell::new(
            KnowledgeCellMetadata {
                scope: format!("agent:{}", agent_id.0),
                status: "ready".to_owned(),
                cell_type: KnowledgeCellType::Feedback,
                memory_type: None,
                ttl_seconds: None,
                created_unix_seconds: Some(current_unix_seconds()),
                source_trust_q16: None,
                source: Some(format!("cell:{}", feedback.source_cell_id.0)),
            },
            feedback_payload(&feedback),
        );
        let commit_seq = self.put_knowledge_cell(cell_id, cell)?;
        Ok(StoredFeedback {
            cell_id,
            commit_seq,
        })
    }

    fn next_feedback_cell_id(&self, agent_id: AgentId) -> EngineResult<CellId> {
        let agent_slot = agent_cell_id_slot(agent_id).ok_or_else(feedback_id_overflow)?;
        let mut sequence = self
            .current_seq()
            .0
            .checked_add(1)
            .ok_or_else(feedback_id_overflow)?;
        // Probe for a free id, mirroring session allocation
        // (`session.rs::next_session_cell_id`). The sequence is derived from
        // `current_seq`; without this probe a reused sequence would silently
        // overwrite an existing feedback cell instead of allocating a new one.
        let mut attempts = 0u64;
        loop {
            let cell_id = namespaced_agent_cell_id(FEEDBACK_CELL_NAMESPACE, agent_slot, sequence)
                .ok_or_else(feedback_id_overflow)?;
            if self.get_latest_cell_descriptor(cell_id).is_none() {
                return Ok(cell_id);
            }
            attempts = attempts.checked_add(1).ok_or_else(feedback_id_overflow)?;
            if attempts > u64::from(u32::MAX) {
                return Err(feedback_id_overflow());
            }
            sequence = sequence.checked_add(1).ok_or_else(feedback_id_overflow)?;
        }
    }

    pub fn feedback_scores(&self) -> BTreeMap<CellId, i32> {
        self.derived_stores.feedback_index.scores()
    }

    pub fn feedback_scores_at(&self, now_unix_seconds: u64) -> BTreeMap<CellId, i32> {
        self.derived_stores
            .feedback_index
            .scores_at(now_unix_seconds)
    }

    pub fn feedback_scores_for_cells_at<I>(
        &self,
        cell_ids: I,
        now_unix_seconds: u64,
    ) -> BTreeMap<CellId, i32>
    where
        I: IntoIterator<Item = CellId>,
    {
        self.derived_stores
            .feedback_index
            .scores_for_cells_at(cell_ids, now_unix_seconds)
    }

    pub fn feedback_score_for_cell_at(&self, cell_id: CellId, now_unix_seconds: u64) -> i32 {
        self.derived_stores
            .feedback_index
            .score_for_cell_at(cell_id, now_unix_seconds)
    }

    pub fn feedback_score_report_at(&self, now_unix_seconds: u64) -> Vec<FeedbackScoreReport> {
        self.derived_stores
            .feedback_index
            .score_report_at(now_unix_seconds)
    }

    pub fn feedback_stats(&self) -> FeedbackStats {
        self.derived_stores.feedback_index.stats()
    }
}

fn feedback_payload(feedback: &ContextFeedback) -> Vec<u8> {
    let mut text = format!(
        "source_cell_id={}\nuseful={}\n",
        feedback.source_cell_id.0, feedback.useful
    );
    if let Some(note) = &feedback.note {
        text.push_str("note=");
        text.push_str(&sanitize_line(note));
        text.push('\n');
    }
    text.into_bytes()
}

fn sanitize_line(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '\n' | '\r' => ' ',
            other => other,
        })
        .collect()
}

pub(crate) fn current_unix_seconds() -> u64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(_) => 0,
    }
}

fn feedback_id_overflow() -> EngineError {
    EngineError::StorageInvariant("feedback cell id space is exhausted".to_owned())
}
