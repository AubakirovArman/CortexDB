use std::time::Instant;

use cortex_aql::{eval_bitmap_program, BoundRetrievePlan};
use cortex_core::memtable::ReadTxn;

use crate::context::{ContextPack, ContextPackOptions};
use crate::database::{
    cell_version_meets_quality_thresholds, expand_parent_context, rank_retrieved_cells,
    suppress_duplicate_content, CandidateResolver, Database, RetrievedCell,
};
use crate::error::EngineResult;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhysicalOperatorTrace {
    pub name: String,
    pub input_count: usize,
    pub output_count: usize,
    pub elapsed_nanos: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetrieveExecutionReport {
    pub cells: Vec<RetrievedCell>,
    pub operators: Vec<PhysicalOperatorTrace>,
    pub total_elapsed_nanos: u64,
}

pub trait PhysicalOp {
    type Item;

    fn next(&mut self) -> Option<Self::Item>;
    fn trace(&self) -> PhysicalOperatorTrace;
}

pub struct BitmapIndexScan {
    candidates: Vec<u32>,
    cursor: usize,
    trace: PhysicalOperatorTrace,
}

impl BitmapIndexScan {
    pub fn from_plan<P: CandidateResolver>(
        plan: &BoundRetrievePlan,
        provider: &P,
    ) -> EngineResult<Self> {
        let started = Instant::now();
        let candidates = eval_bitmap_program(&plan.bitmap_program, provider)?
            .into_iter()
            .collect::<Vec<_>>();
        Ok(Self {
            candidates,
            cursor: 0,
            trace: PhysicalOperatorTrace {
                name: "BitmapIndexScan".to_owned(),
                input_count: 0,
                output_count: 0,
                elapsed_nanos: elapsed_nanos(started),
            },
        })
    }
}

impl PhysicalOp for BitmapIndexScan {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.candidates.get(self.cursor).copied()?;
        self.cursor += 1;
        self.trace.output_count += 1;
        Some(value)
    }

    fn trace(&self) -> PhysicalOperatorTrace {
        self.trace.clone()
    }
}

pub struct PermissionFilter {
    agent_allowed: std::collections::BTreeSet<u32>,
    candidates: Vec<u32>,
    cursor: usize,
    started: Instant,
    trace: PhysicalOperatorTrace,
}

impl PermissionFilter {
    pub fn new<P: cortex_aql::BitmapProvider>(provider: &P, candidates: Vec<u32>) -> Self {
        let input_count = candidates.len();
        Self {
            agent_allowed: provider.agent_allowed(),
            candidates,
            cursor: 0,
            started: Instant::now(),
            trace: PhysicalOperatorTrace {
                name: "PermissionFilter".to_owned(),
                input_count,
                output_count: 0,
                elapsed_nanos: 0,
            },
        }
    }
}

impl PhysicalOp for PermissionFilter {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(candidate) = self.candidates.get(self.cursor).copied() {
            self.cursor += 1;
            if self.agent_allowed.contains(&candidate) {
                self.trace.output_count += 1;
                return Some(candidate);
            }
        }
        None
    }

    fn trace(&self) -> PhysicalOperatorTrace {
        let mut trace = self.trace.clone();
        trace.elapsed_nanos = elapsed_nanos(self.started);
        trace
    }
}

pub struct QualityFilter<'a, P> {
    database: &'a Database,
    provider: &'a P,
    plan: &'a BoundRetrievePlan,
    txn: ReadTxn,
    candidates: Vec<u32>,
    cursor: usize,
    started: Instant,
    trace: PhysicalOperatorTrace,
}

impl<'a, P: CandidateResolver> QualityFilter<'a, P> {
    pub fn new(
        database: &'a Database,
        provider: &'a P,
        plan: &'a BoundRetrievePlan,
        candidates: Vec<u32>,
    ) -> Self {
        let input_count = candidates.len();
        Self {
            database,
            provider,
            plan,
            txn: database.read_txn(),
            candidates,
            cursor: 0,
            started: Instant::now(),
            trace: PhysicalOperatorTrace {
                name: "QualityFilter".to_owned(),
                input_count,
                output_count: 0,
                elapsed_nanos: 0,
            },
        }
    }
}

impl<P: CandidateResolver> PhysicalOp for QualityFilter<'_, P> {
    type Item = RetrievedCell;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let candidate = self.candidates.get(self.cursor).copied()?;
            self.cursor += 1;
            let cell = self
                .provider
                .cell_id_for_candidate(candidate)
                .and_then(|cell_id| self.database.memtable.read(self.txn, cell_id))
                .filter(|version| {
                    cell_version_meets_quality_thresholds(version, &self.plan.quality_thresholds)
                })
                .map(RetrievedCell::from_version);
            if let Some(cell) = cell {
                self.trace.output_count += 1;
                return Some(cell);
            }
        }
    }

    fn trace(&self) -> PhysicalOperatorTrace {
        let mut trace = self.trace.clone();
        trace.elapsed_nanos = elapsed_nanos(self.started);
        trace
    }
}

pub struct MaterializedOp<T> {
    items: Vec<Option<T>>,
    cursor: usize,
    trace: PhysicalOperatorTrace,
}

impl<T> MaterializedOp<T> {
    fn new(name: &str, input_count: usize, items: Vec<T>, elapsed_nanos: u64) -> Self {
        let output_count = items.len();
        Self {
            items: items.into_iter().map(Some).collect(),
            cursor: 0,
            trace: PhysicalOperatorTrace {
                name: name.to_owned(),
                input_count,
                output_count,
                elapsed_nanos,
            },
        }
    }
}

impl<T> PhysicalOp for MaterializedOp<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.items.get_mut(self.cursor).and_then(Option::take)?;
        self.cursor += 1;
        Some(value)
    }

    fn trace(&self) -> PhysicalOperatorTrace {
        self.trace.clone()
    }
}

pub struct LexicalScan {
    inner: MaterializedOp<u32>,
}

impl LexicalScan {
    pub fn passthrough(candidates: Vec<u32>) -> Self {
        let input_count = candidates.len();
        Self {
            inner: MaterializedOp::new("LexicalScan", input_count, candidates, 0),
        }
    }
}

impl PhysicalOp for LexicalScan {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn trace(&self) -> PhysicalOperatorTrace {
        self.inner.trace()
    }
}

pub struct VectorScan {
    inner: MaterializedOp<u32>,
}

impl VectorScan {
    pub fn passthrough(candidates: Vec<u32>) -> Self {
        let input_count = candidates.len();
        Self {
            inner: MaterializedOp::new("VectorScan", input_count, candidates, 0),
        }
    }
}

impl PhysicalOp for VectorScan {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn trace(&self) -> PhysicalOperatorTrace {
        self.inner.trace()
    }
}

pub struct VerifyOp {
    inner: MaterializedOp<RetrievedCell>,
}

impl VerifyOp {
    pub fn passthrough(cells: Vec<RetrievedCell>) -> Self {
        let input_count = cells.len();
        Self {
            inner: MaterializedOp::new("VerifyOp", input_count, cells, 0),
        }
    }
}

impl PhysicalOp for VerifyOp {
    type Item = RetrievedCell;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn trace(&self) -> PhysicalOperatorTrace {
        self.inner.trace()
    }
}

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

pub fn execute_retrieve<P: CandidateResolver>(
    database: &Database,
    plan: &BoundRetrievePlan,
    provider: &P,
) -> EngineResult<RetrieveExecutionReport> {
    let started_total = Instant::now();
    let mut collector = ExplainCollector::default();

    let mut scan = BitmapIndexScan::from_plan(plan, provider)?;
    let candidates = drain(&mut scan);
    collector.push(scan.trace());

    let mut permission_filter = PermissionFilter::new(provider, candidates);
    let candidates = drain(&mut permission_filter);
    collector.push(permission_filter.trace());

    let mut quality_filter = QualityFilter::new(database, provider, plan, candidates);
    let cells = drain(&mut quality_filter);
    collector.push(quality_filter.trace());

    let started = Instant::now();
    let rank_input_count = cells.len();
    let ranked = rank_retrieved_cells(cells, &plan.task, &plan.weights);
    let mut rank_op =
        MaterializedOp::new("RankOp", rank_input_count, ranked, elapsed_nanos(started));
    let ranked = drain(&mut rank_op);
    collector.push(rank_op.trace());

    let started = Instant::now();
    let deduped = suppress_duplicate_content(ranked);
    let mut dedup_op = MaterializedOp::new(
        "DedupOp",
        collector.last_output_count(),
        deduped,
        elapsed_nanos(started),
    );
    let deduped = drain(&mut dedup_op);
    collector.push(dedup_op.trace());

    let started = Instant::now();
    let expanded = expand_parent_context(deduped);
    let mut parent_op = MaterializedOp::new(
        "ParentExpandOp",
        collector.last_output_count(),
        expanded,
        elapsed_nanos(started),
    );
    let expanded = drain(&mut parent_op);
    collector.push(parent_op.trace());

    let limit = plan.context_policy.candidate_limit as usize;
    let started = Instant::now();
    let limited = expanded.into_iter().take(limit).collect::<Vec<_>>();
    let mut limit_op = MaterializedOp::new(
        "LimitOp",
        collector.last_output_count(),
        limited,
        elapsed_nanos(started),
    );
    let cells = drain(&mut limit_op);
    collector.push(limit_op.trace());

    Ok(RetrieveExecutionReport {
        cells,
        operators: collector.into_traces(),
        total_elapsed_nanos: elapsed_nanos(started_total),
    })
}

fn drain<O: PhysicalOp>(operator: &mut O) -> Vec<O::Item> {
    let mut values = Vec::new();
    while let Some(value) = operator.next() {
        values.push(value);
    }
    values
}

fn elapsed_nanos(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cortex_core::CellId;

    #[test]
    fn materialized_operator_reports_available_output() {
        let mut op = MaterializedOp::new("RankOp", 3, vec![1, 2], 7);

        assert_eq!(drain(&mut op), vec![1, 2]);
        assert_eq!(
            op.trace(),
            PhysicalOperatorTrace {
                name: "RankOp".to_owned(),
                input_count: 3,
                output_count: 2,
                elapsed_nanos: 7
            }
        );
    }

    #[test]
    fn explain_collector_tracks_operator_output_counts() {
        let mut collector = ExplainCollector::default();
        assert_eq!(collector.last_output_count(), 0);

        collector.push(PhysicalOperatorTrace {
            name: "BitmapIndexScan".to_owned(),
            input_count: 0,
            output_count: 2,
            elapsed_nanos: 5,
        });

        assert_eq!(collector.last_output_count(), 2);
        assert_eq!(collector.into_traces().len(), 1);
    }

    #[test]
    fn pack_operator_reports_input_and_selected_cells() {
        let execution = PackOp::execute(
            vec![RetrievedCell::from_payload(
                CellId(1),
                b"source=doc-a\n\nalpha evidence".to_vec(),
            )],
            1_000,
            false,
            &ContextPackOptions::default(),
            "alpha",
            &std::collections::BTreeMap::new(),
            None,
        );

        assert_eq!(execution.trace.name, "PackOp");
        assert_eq!(execution.trace.input_count, 1);
        assert_eq!(execution.trace.output_count, execution.pack.cells.len());
        assert_eq!(execution.pack.cells.len(), 1);
    }
}
