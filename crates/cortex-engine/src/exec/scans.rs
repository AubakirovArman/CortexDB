use std::time::Instant;

use cortex_aql::{eval_bitmap_program, BoundRetrievePlan};
use cortex_core::memtable::ReadTxn;

use super::trace::{elapsed_nanos, MaterializedOp, PhysicalOp, PhysicalOperatorTrace};
use crate::database::{CandidateResolver, Database, PinnedReadTxn, RetrievedCell};
use crate::error::EngineResult;
use crate::retrieval_quality::cell_version_meets_quality_thresholds;

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
    agent_allowed: cortex_aql::RoaringBitmap,
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
            if self.agent_allowed.contains(candidate) {
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
    _pin: PinnedReadTxn,
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
        let pin = database.pin_read_txn();
        let txn = pin.read_txn();
        Self {
            database,
            provider,
            plan,
            _pin: pin,
            txn,
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
                .and_then(|version| self.database.retrieved_cell_from_version(version).ok());
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
