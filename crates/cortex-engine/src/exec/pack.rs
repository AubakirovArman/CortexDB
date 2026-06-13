use std::time::Instant;

use std::collections::BTreeMap;

use cortex_aql::AgentView;
use cortex_core::CellId;

use crate::context::{ContextPack, ContextPackBuilder, ContextPackOptions};
use crate::database::RetrievedCell;

use super::trace::{elapsed_nanos, PhysicalOp, PhysicalOperatorTrace};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExplainCollector {
    operators: Vec<PhysicalOperatorTrace>,
}

impl ExplainCollector {
    pub fn push(&mut self, trace: PhysicalOperatorTrace) {
        self.operators.push(trace);
    }

    pub fn last_output_count(&self) -> usize {
        self.operators
            .last()
            .map(|operator| operator.output_count)
            .unwrap_or_default()
    }

    pub fn into_traces(self) -> Vec<PhysicalOperatorTrace> {
        self.operators
    }
}

pub struct PackExecution {
    pub pack: ContextPack,
    pub trace: PhysicalOperatorTrace,
    pub budget_filled: bool,
}

pub struct PackOp<'a> {
    cells: Option<Vec<RetrievedCell>>,
    token_budget_tokens: u32,
    citations_required: bool,
    options: &'a ContextPackOptions,
    query: &'a str,
    feedback_scores: &'a BTreeMap<CellId, i32>,
    access_view: Option<&'a AgentView>,
    trace: PhysicalOperatorTrace,
    budget_filled: bool,
}

impl<'a> PackOp<'a> {
    pub fn new(
        cells: Vec<RetrievedCell>,
        token_budget_tokens: u32,
        citations_required: bool,
        options: &'a ContextPackOptions,
        query: &'a str,
        feedback_scores: &'a BTreeMap<CellId, i32>,
        access_view: Option<&'a AgentView>,
    ) -> Self {
        let input_count = cells.len();
        Self {
            cells: Some(cells),
            token_budget_tokens,
            citations_required,
            options,
            query,
            feedback_scores,
            access_view,
            trace: PhysicalOperatorTrace {
                name: "PackOp".to_owned(),
                input_count,
                output_count: 0,
                elapsed_nanos: 0,
            },
            budget_filled: false,
        }
    }

    pub fn execute(
        cells: Vec<RetrievedCell>,
        token_budget_tokens: u32,
        citations_required: bool,
        options: &'a ContextPackOptions,
        query: &'a str,
        feedback_scores: &'a BTreeMap<CellId, i32>,
        access_view: Option<&'a AgentView>,
    ) -> PackExecution {
        let mut op = Self::new(
            cells,
            token_budget_tokens,
            citations_required,
            options,
            query,
            feedback_scores,
            access_view,
        );
        let pack = op.next().unwrap_or_else(|| {
            ContextPackBuilder::new(
                token_budget_tokens,
                citations_required,
                options,
                query,
                feedback_scores,
                access_view,
            )
            .build_from_retrieved(Vec::new())
        });
        PackExecution {
            trace: op.trace(),
            budget_filled: op.budget_filled(),
            pack,
        }
    }

    pub fn budget_filled(&self) -> bool {
        self.budget_filled
    }
}

impl PhysicalOp for PackOp<'_> {
    type Item = ContextPack;

    fn next(&mut self) -> Option<Self::Item> {
        let cells = self.cells.take()?;
        let started = Instant::now();
        let pack = ContextPackBuilder::new(
            self.token_budget_tokens,
            self.citations_required,
            self.options,
            self.query,
            self.feedback_scores,
            self.access_view,
        )
        .build_from_retrieved(cells);
        self.budget_filled = pack_budget_filled(&pack);
        self.trace.output_count = pack.cells.len();
        self.trace.elapsed_nanos = elapsed_nanos(started);
        Some(pack)
    }

    fn trace(&self) -> PhysicalOperatorTrace {
        self.trace.clone()
    }
}

fn pack_budget_filled(pack: &ContextPack) -> bool {
    pack.token_budget_tokens > 0 && pack.estimated_tokens >= pack.token_budget_tokens
}
