use cortex_aql::AgentView;

use crate::database::Database;
use crate::error::EngineResult;

use super::access::allowed_candidates;
use super::ann::{evaluate_persisted_ann, AnnEvaluationReport};
use super::database::SearchLimit;

impl Database {
    pub fn evaluate_vector_ann(
        &self,
        vector: &[i16],
        view: &AgentView,
        limit: SearchLimit,
    ) -> EngineResult<Option<AnnEvaluationReport>> {
        if self.manifest().live_segments.is_empty() {
            return Ok(None);
        }
        let checkpoint_seq = cortex_core::CommitSeq(self.manifest().checkpoint_seq);
        if !self
            .memtable
            .changed_cell_ids_after(checkpoint_seq)
            .is_empty()
        {
            return Ok(None);
        }
        let state = self.persisted_index_state()?;
        let allowed = allowed_candidates(&state.bitmap, view);
        let index = self.persisted_vector_index()?;
        let graph = self.persisted_hnsw_graph()?;
        Ok(Some(evaluate_persisted_ann(
            &index.vectors,
            &graph,
            vector,
            &allowed,
            limit.0,
        )))
    }
}
