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

fn unix_now() -> u64 {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(_) => 0,
    }
}

fn feedback_id_overflow() -> EngineError {
    EngineError::StorageInvariant("feedback cell id space is exhausted".to_owned())
}
