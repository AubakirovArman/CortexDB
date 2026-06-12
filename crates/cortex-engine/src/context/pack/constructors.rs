use std::collections::BTreeMap;

use cortex_core::CellId;

use crate::context::grounding::{ground_answer, AnswerGroundingOptions, AnswerGroundingReport};
use crate::context::{ContextPack, ContextPackOptions};
use crate::database::RetrievedCell;

impl ContextPack {
    pub fn ground_answer(
        &self,
        answer: &str,
        options: AnswerGroundingOptions,
    ) -> AnswerGroundingReport {
        ground_answer(&self.cells, answer, options)
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
}
