use std::path::PathBuf;

use crate::database::Database;
use crate::error::EngineResult;

use super::ackg::ACKG_FILE_NAME;
use super::types::{GraphEdge, GraphEntity, KnowledgeGraphIndex, ToolCell};
use super::GraphIndexStore;

impl Database {
    /// Build a deterministic graph index from the current snapshot.
    pub fn knowledge_graph_index(&self) -> KnowledgeGraphIndex {
        self.graph_index_store_snapshot().index()
    }

    pub fn knowledge_graph_index_path(&self) -> PathBuf {
        self.root_path.join(ACKG_FILE_NAME)
    }

    pub fn persist_knowledge_graph_index(&self) -> EngineResult<KnowledgeGraphIndex> {
        let index = self.knowledge_graph_index();
        index.write_ackg(self.knowledge_graph_index_path())?;
        Ok(index)
    }

    pub fn read_persisted_knowledge_graph_index(
        &self,
    ) -> EngineResult<Option<KnowledgeGraphIndex>> {
        let path = self.knowledge_graph_index_path();
        if !path.exists() {
            return Ok(None);
        }
        KnowledgeGraphIndex::read_ackg(path).map(Some)
    }

    /// Find Entity cells by exact entity name.
    pub fn graph_entities(&self, entity_name: &str) -> Vec<GraphEntity> {
        self.knowledge_graph_index().entities_named(entity_name)
    }

    /// Find all Relation cells that connect to the given entity name
    /// (as either subject or object).
    pub fn graph_neighbors(&self, entity_name: &str) -> Vec<GraphEdge> {
        self.knowledge_graph_index().neighbors(entity_name)
    }

    /// Find visible cells associated with a source identifier.
    pub fn graph_cells_for_source(&self, source_id: &str) -> Vec<cortex_core::CellId> {
        self.knowledge_graph_index().cells_for_source(source_id)
    }

    /// Find visible relation cells that connect a source to a fact.
    pub fn graph_source_supports_fact_edges(&self) -> Vec<GraphEdge> {
        self.knowledge_graph_index().source_supports_fact_edges()
    }

    /// Find visible relation cells that mark one fact as contradicting another.
    pub fn graph_fact_contradicts_fact_edges(&self) -> Vec<GraphEdge> {
        self.knowledge_graph_index().fact_contradicts_fact_edges()
    }

    /// Find all Tool cells in the database.
    pub fn tool_cells(&self) -> Vec<ToolCell> {
        self.graph_index_store_snapshot().tool_cells()
    }

    fn graph_index_store_snapshot(&self) -> GraphIndexStore {
        self.derived_stores.graph_index_store.clone()
    }
}
