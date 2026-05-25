use std::collections::BTreeMap;

use cortex_aql::AgentId;
use cortex_core::{CellId, CommitSeq, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};

use crate::database::Database;
use crate::error::{EngineError, EngineResult};

const FEEDBACK_CELL_NAMESPACE: u64 = 0x9000_0000_0000_0000;

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
                created_unix_seconds: Some(unix_now()),
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
        let agent_bits = (agent_id.0 & 0x0fff_ffff) << 32;
        let sequence = self
            .current_seq()
            .0
            .checked_add(1)
            .ok_or_else(feedback_id_overflow)?;
        Ok(CellId(
            FEEDBACK_CELL_NAMESPACE | agent_bits | (sequence & 0xffff_ffff),
        ))
    }

    pub fn feedback_scores(&self) -> BTreeMap<CellId, i32> {
        let mut scores = BTreeMap::<CellId, i32>::new();
        for version in self.snapshot_versions() {
            let Some((source_cell_id, useful)) = feedback_target(&version.payload) else {
                continue;
            };
            let delta = if useful { 1 } else { -1 };
            *scores.entry(source_cell_id).or_default() += delta;
        }
        scores
    }

    pub fn feedback_stats(&self) -> FeedbackStats {
        let mut stats = FeedbackStats::default();
        for version in self.snapshot_versions() {
            let Some((source_cell_id, useful)) = feedback_target(&version.payload) else {
                continue;
            };
            stats.total += 1;
            let cell_stats = stats.by_source_cell.entry(source_cell_id).or_default();
            if useful {
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

fn feedback_target(payload: &[u8]) -> Option<(CellId, bool)> {
    let text = String::from_utf8_lossy(payload);
    let mut is_feedback = false;
    let mut source_cell_id = None;
    let mut useful = None;
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
        }
    }
    is_feedback.then_some((source_cell_id?, useful?))
}

fn unix_now() -> u64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(_) => 0,
    }
}

fn feedback_id_overflow() -> EngineError {
    EngineError::StorageInvariant("feedback cell id space is exhausted".to_owned())
}
