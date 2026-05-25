use std::collections::BTreeMap;

use cortex_aql::AgentView;
use cortex_core::CellId;

use crate::database::Database;
use crate::error::{EngineError, EngineResult};
use crate::query::{scope_id, CellMetadata};

use super::{SearchIndexes, SearchMode, SearchQuery};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchLimit(pub usize);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatabaseSearchResult {
    pub cell_id: CellId,
    pub score: u64,
    pub lexical_score: u64,
    pub vector_score: u64,
    pub payload: Vec<u8>,
}

impl Database {
    pub fn search_keyword(
        &self,
        text: &str,
        view: &AgentView,
        limit: SearchLimit,
    ) -> EngineResult<Vec<DatabaseSearchResult>> {
        self.search_cells(
            SearchQuery {
                text,
                vector: None,
                limit: limit.0,
                mode: SearchMode::Keyword,
            },
            view,
        )
    }

    pub fn search_cells(
        &self,
        query: SearchQuery<'_>,
        view: &AgentView,
    ) -> EngineResult<Vec<DatabaseSearchResult>> {
        let mut indexes = SearchIndexes::default();
        let mut cells = BTreeMap::<u32, (CellId, Vec<u8>)>::new();
        for (index, (version, metadata)) in self
            .snapshot_versions()
            .into_iter()
            .filter_map(|version| {
                let metadata = CellMetadata::from_payload(&version.payload);
                view.can_read_scope(scope_id(&metadata.scope))
                    .then_some((version, metadata))
            })
            .enumerate()
        {
            let candidate =
                u32::try_from(index + 1).map_err(|_| EngineError::CandidateIdOverflow)?;
            indexes.add_document(candidate, &metadata.body_text);
            if let Some(vector) = vector_line(&version.payload) {
                indexes.add_vector(candidate, vector);
            }
            cells.insert(candidate, (version.cell_id, version.payload));
        }
        Ok(indexes
            .search(query)
            .into_iter()
            .filter_map(|result| {
                let (cell_id, payload) = cells.remove(&result.cell_id)?;
                Some(DatabaseSearchResult {
                    cell_id,
                    score: result.score,
                    lexical_score: result.lexical_score,
                    vector_score: result.vector_score,
                    payload,
                })
            })
            .collect())
    }
}

fn vector_line(payload: &[u8]) -> Option<Vec<i16>> {
    let text = String::from_utf8_lossy(payload);
    text.lines().find_map(|line| {
        let value = line.trim().strip_prefix("vector=")?;
        let vector = value
            .split([',', ' '])
            .filter(|part| !part.is_empty())
            .map(str::parse::<i16>)
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        (!vector.is_empty()).then_some(vector)
    })
}
