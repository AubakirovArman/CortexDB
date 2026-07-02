use std::collections::BTreeMap;

use cortex_aql::AgentView;
use cortex_core::CellId;

use crate::context::grounding::{ground_answer, AnswerGroundingOptions, AnswerGroundingReport};
use crate::context::{ContextPack, ContextPackAnomaly, ContextPackBuilder, ContextPackOptions};
use crate::database::RetrievedCell;

impl ContextPack {
    pub fn ground_answer(
        &self,
        answer: &str,
        options: AnswerGroundingOptions,
    ) -> AnswerGroundingReport {
        ground_answer(&self.cells, answer, options)
    }

    pub fn with_grounding_report(mut self, report: AnswerGroundingReport) -> Self {
        self.grounding_report = Some(report);
        self
    }

    pub fn with_grounded_answer(mut self, answer: &str, options: AnswerGroundingOptions) -> Self {
        self.grounding_report = Some(self.ground_answer(answer, options));
        self
    }

    pub fn from_retrieved(
        cells: Vec<RetrievedCell>,
        token_budget_tokens: u32,
        citations_required: bool,
    ) -> Self {
        Self::from_retrieved_with_options(
            cells,
            token_budget_tokens,
            citations_required,
            &ContextPackOptions::default(),
            "",
        )
    }

    pub fn from_retrieved_with_options(
        cells: Vec<RetrievedCell>,
        token_budget_tokens: u32,
        citations_required: bool,
        options: &ContextPackOptions,
        query: &str,
    ) -> Self {
        Self::from_retrieved_with_feedback_options(
            cells,
            token_budget_tokens,
            citations_required,
            options,
            query,
            &BTreeMap::new(),
        )
    }

    pub fn from_retrieved_with_feedback_options(
        cells: Vec<RetrievedCell>,
        token_budget_tokens: u32,
        citations_required: bool,
        options: &ContextPackOptions,
        query: &str,
        feedback_scores: &BTreeMap<CellId, i32>,
    ) -> Self {
        Self::from_retrieved_with_feedback_options_and_view(
            cells,
            token_budget_tokens,
            citations_required,
            options,
            query,
            feedback_scores,
            None,
        )
    }

    pub fn from_retrieved_with_feedback_options_and_view(
        cells: Vec<RetrievedCell>,
        token_budget_tokens: u32,
        citations_required: bool,
        options: &ContextPackOptions,
        query: &str,
        feedback_scores: &BTreeMap<CellId, i32>,
        access_view: Option<&AgentView>,
    ) -> Self {
        Self::from_retrieved_with_feedback_options_view_and_anomalies(
            cells,
            token_budget_tokens,
            citations_required,
            options,
            query,
            feedback_scores,
            access_view,
            Vec::new(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_retrieved_with_feedback_options_view_and_anomalies(
        cells: Vec<RetrievedCell>,
        token_budget_tokens: u32,
        citations_required: bool,
        options: &ContextPackOptions,
        query: &str,
        feedback_scores: &BTreeMap<CellId, i32>,
        access_view: Option<&AgentView>,
        initial_anomalies: Vec<ContextPackAnomaly>,
    ) -> Self {
        ContextPackBuilder::new(
            token_budget_tokens,
            citations_required,
            options,
            query,
            feedback_scores,
            access_view,
        )
        .with_initial_anomalies(initial_anomalies)
        .build_from_retrieved(cells)
    }
}
