use cortex_core::{CellId, CommitSeq, KnowledgeCell};

use crate::database::Database;
use crate::error::EngineResult;

impl Database {
    pub fn put_knowledge_cell(
        &mut self,
        cell_id: CellId,
        cell: KnowledgeCell,
    ) -> EngineResult<CommitSeq> {
        self.put_cell(cell_id, cell.encode_payload())
    }
}
