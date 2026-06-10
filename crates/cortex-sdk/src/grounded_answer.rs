use std::collections::BTreeSet;

use crate::aql::{Aql, AqlBuildError, AqlRetrievalMode};
use crate::types::{
    AnswerGroundingOptionsResponse, AnswerGroundingReportResponse, ContextPackResponse,
    VerificationReportResponse,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroundedAnswerRequest {
    scope: String,
    brain: String,
    question: String,
    mode: Option<AqlRetrievalMode>,
    budget_tokens: Option<u64>,
    candidate_limit: Option<u32>,
    where_clause: Option<String>,
    require_citations: bool,
    grounding_options: AnswerGroundingOptionsResponse,
    verify_answer: bool,
}

impl GroundedAnswerRequest {
    pub fn new(
        scope: impl Into<String>,
        brain: impl Into<String>,
        question: impl Into<String>,
    ) -> Self {
        Self {
            scope: scope.into(),
            brain: brain.into(),
            question: question.into(),
            mode: None,
            budget_tokens: None,
            candidate_limit: None,
            where_clause: None,
            require_citations: true,
            grounding_options: AnswerGroundingOptionsResponse {
                require_citations: true,
                ..AnswerGroundingOptionsResponse::default()
            },
            verify_answer: true,
        }
    }

    pub fn mode(mut self, mode: AqlRetrievalMode) -> Self {
        self.mode = Some(mode);
        self
    }

    pub fn budget_tokens(mut self, budget_tokens: u64) -> Self {
        self.budget_tokens = Some(budget_tokens);
        self
    }

    pub fn limit_candidates(mut self, candidate_limit: u32) -> Self {
        self.candidate_limit = Some(candidate_limit);
        self
    }

    pub fn where_clause(mut self, where_clause: impl Into<String>) -> Self {
        self.where_clause = Some(where_clause.into());
        self
    }

    pub fn require_citations(mut self, require: bool) -> Self {
        self.require_citations = require;
        self.grounding_options.require_citations = require;
        self
    }

    pub fn grounding_options(mut self, options: AnswerGroundingOptionsResponse) -> Self {
        self.grounding_options = options;
        self.require_citations = options.require_citations;
        self
    }

    pub fn verify_answer(mut self, verify: bool) -> Self {
        self.verify_answer = verify;
        self
    }

    pub fn scope(&self) -> &str {
        &self.scope
    }

    pub fn brain(&self) -> &str {
        &self.brain
    }

    pub fn question(&self) -> &str {
        &self.question
    }

    pub fn should_verify_answer(&self) -> bool {
        self.verify_answer
    }

    pub fn grounding_options_value(&self) -> AnswerGroundingOptionsResponse {
        self.grounding_options
    }

    pub fn retrieve_statement(&self) -> Result<String, AqlBuildError> {
        let mut builder = Aql::retrieve_context(&self.question, &self.brain);
        if let Some(mode) = self.mode {
            builder = builder.mode(mode);
        }
        if let Some(budget_tokens) = self.budget_tokens {
            builder = builder.budget_tokens(budget_tokens);
        }
        if let Some(candidate_limit) = self.candidate_limit {
            builder = builder.limit_candidates(candidate_limit);
        }
        if let Some(where_clause) = self.where_clause.as_deref() {
            builder = builder.where_clause(where_clause);
        }
        if self.require_citations {
            builder = builder.require_citations();
        }
        builder.build()
    }

    pub fn verify_statement(&self, answer: &str) -> Result<Option<String>, AqlBuildError> {
        if !self.verify_answer || answer.trim().is_empty() {
            return Ok(None);
        }
        Aql::verify_fact(answer, &self.brain).build().map(Some)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroundedAnswerResponse {
    pub question: String,
    pub answer: String,
    pub retrieve_statement: String,
    pub verify_statement: Option<String>,
    pub context: ContextPackResponse,
    pub grounding: AnswerGroundingReportResponse,
    pub verification: Option<VerificationReportResponse>,
    pub citations: Vec<String>,
    pub used_context_cell_ids: Vec<u64>,
    pub rejected: bool,
}

impl GroundedAnswerResponse {
    pub fn from_context_answer(
        request: &GroundedAnswerRequest,
        retrieve_statement: String,
        verify_statement: Option<String>,
        context: ContextPackResponse,
        answer: String,
        verification: Option<VerificationReportResponse>,
    ) -> Self {
        let grounding =
            context.ground_answer_with_options(&answer, request.grounding_options_value());
        let citations = collect_citations(&context, &grounding);
        let used_context_cell_ids = collect_used_cell_ids(&context, &grounding);
        let rejected = grounding.rejected;
        Self {
            question: request.question().to_owned(),
            answer,
            retrieve_statement,
            verify_statement,
            context,
            grounding,
            verification,
            citations,
            used_context_cell_ids,
            rejected,
        }
    }

    pub fn answer_supported(&self) -> bool {
        self.grounding.answer_supported
    }

    pub fn verification_status(&self) -> Option<&str> {
        self.verification
            .as_ref()
            .map(|value| value.status.as_str())
    }
}

fn collect_citations(
    context: &ContextPackResponse,
    grounding: &AnswerGroundingReportResponse,
) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut citations = Vec::new();
    for citation in grounding
        .spans
        .iter()
        .flat_map(|span| span.citations.iter())
        .chain(
            context
                .cells
                .iter()
                .filter_map(|cell| cell.citation.as_ref()),
        )
    {
        if !citation.trim().is_empty() && seen.insert(citation.clone()) {
            citations.push(citation.clone());
        }
    }
    citations
}

fn collect_used_cell_ids(
    context: &ContextPackResponse,
    grounding: &AnswerGroundingReportResponse,
) -> Vec<u64> {
    let mut seen = BTreeSet::new();
    let mut ids = Vec::new();
    for cell_id in grounding
        .spans
        .iter()
        .flat_map(|span| span.supported_by_cell_ids.iter().copied())
        .chain(context.cells.iter().map(|cell| cell.cell_id))
    {
        if seen.insert(cell_id) {
            ids.push(cell_id);
        }
    }
    ids
}
