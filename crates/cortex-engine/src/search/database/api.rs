use cortex_aql::{AgentView, BoundRetrievePlan};

use crate::database::Database;
use crate::error::EngineResult;

use super::super::ann::AnnSearchPolicy;
use super::super::{SearchMode, SearchQuery, SearchReranker};
use super::allowed::allowed_cells_from_bound_retrieve_plan;
use super::ranking::{
    rerank_database_results, search_rerank_candidate_limit, search_rerank_result_limit,
};
use super::{DatabaseSearchOutcome, DatabaseSearchResult, SearchLimit};

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

    pub fn search_vector(
        &self,
        vector: &[i16],
        view: &AgentView,
        limit: SearchLimit,
    ) -> EngineResult<Vec<DatabaseSearchResult>> {
        self.search_cells(
            SearchQuery {
                text: "",
                vector: Some(vector),
                limit: limit.0,
                mode: SearchMode::Vector,
            },
            view,
        )
    }

    pub fn search_vector_with_report(
        &self,
        vector: &[i16],
        view: &AgentView,
        limit: SearchLimit,
    ) -> EngineResult<DatabaseSearchOutcome> {
        self.search_vector_with_report_with_policy(vector, view, limit, AnnSearchPolicy::default())
    }

    pub fn search_vector_with_report_with_policy(
        &self,
        vector: &[i16],
        view: &AgentView,
        limit: SearchLimit,
        policy: AnnSearchPolicy,
    ) -> EngineResult<DatabaseSearchOutcome> {
        self.search_cells_with_report_with_policy(
            SearchQuery {
                text: "",
                vector: Some(vector),
                limit: limit.0,
                mode: SearchMode::Vector,
            },
            view,
            Some(policy),
        )
    }

    pub fn search_vector_exact(
        &self,
        vector: &[i16],
        view: &AgentView,
        limit: SearchLimit,
    ) -> EngineResult<Vec<DatabaseSearchResult>> {
        self.search_cells(
            SearchQuery {
                text: "",
                vector: Some(vector),
                limit: limit.0,
                mode: SearchMode::VectorExact,
            },
            view,
        )
    }

    pub fn search_cells(
        &self,
        query: SearchQuery<'_>,
        view: &AgentView,
    ) -> EngineResult<Vec<DatabaseSearchResult>> {
        Ok(self.search_cells_with_report(query, view)?.results)
    }

    pub fn search_cells_with_reranker(
        &self,
        query: SearchQuery<'_>,
        view: &AgentView,
        reranker: &dyn SearchReranker,
    ) -> EngineResult<Vec<DatabaseSearchResult>> {
        let mut results = self
            .search_cells_with_report(
                SearchQuery {
                    limit: search_rerank_candidate_limit(query),
                    ..query
                },
                view,
            )?
            .results;
        rerank_database_results(&mut results, query, reranker);
        results.truncate(search_rerank_result_limit(query));
        Ok(results)
    }

    pub fn search_cells_with_report(
        &self,
        query: SearchQuery<'_>,
        view: &AgentView,
    ) -> EngineResult<DatabaseSearchOutcome> {
        self.search_cells_with_report_with_policy(query, view, None)
    }

    #[doc(hidden)]
    pub fn search_cells_with_bound_retrieve_plan(
        &self,
        query: SearchQuery<'_>,
        view: &AgentView,
        plan: &BoundRetrievePlan,
    ) -> EngineResult<DatabaseSearchOutcome> {
        let allowed_cells = allowed_cells_from_bound_retrieve_plan(self, view, plan)?;
        self.search_cells_with_report_with_policy_and_allowed_cells(
            query,
            view,
            None,
            Some(&allowed_cells),
        )
    }

    pub fn search_diagnostics(&self, query: &str) -> EngineResult<String> {
        let terms = crate::search::tokenize(query);
        Ok(format!(
            "query_terms_count={} terms=[{}]",
            terms.len(),
            terms.join(", ")
        ))
    }
}
