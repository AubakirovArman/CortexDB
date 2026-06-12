use std::time::Instant;

use crate::context::{ContextPack, ContextPackOptions};
use crate::database::RetrievedCell;

use super::trace::{elapsed_nanos, PhysicalOperatorTrace};

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
}

pub struct PackOp;

impl PackOp {
    pub fn execute(
        cells: Vec<RetrievedCell>,
        token_budget_tokens: u32,
        citations_required: bool,
        options: &ContextPackOptions,
        query: &str,
        feedback_scores: &std::collections::BTreeMap<cortex_core::CellId, i32>,
        access_view: Option<&cortex_aql::AgentView>,
    ) -> PackExecution {
        let input_count = cells.len();
        let started = Instant::now();
        let pack = ContextPack::from_retrieved_with_feedback_options_and_view(
            cells,
            token_budget_tokens,
            citations_required,
            options,
            query,
            feedback_scores,
            access_view,
        );
        PackExecution {
            trace: PhysicalOperatorTrace {
                name: "PackOp".to_owned(),
                input_count,
                output_count: pack.cells.len(),
                elapsed_nanos: elapsed_nanos(started),
            },
            pack,
        }
    }
}
