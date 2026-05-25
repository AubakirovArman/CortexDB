use std::collections::{BTreeMap, BTreeSet};

mod metadata;

use cortex_aql::{
    parse_aql, AgentView, AqlCatalog, Binder, BitmapHandle, BitmapProvider, BoundPlan, BrainId,
    CellTypeId, MemoryType, ScopeId, StatusId,
};
use cortex_core::memtable::CellVersion;
use cortex_core::{CellId, CommitSeq};
use cortex_storage::indexes::{BitmapIndex, LexicalIndex};
use cortex_storage::segment::SegmentCell;

use crate::database::{CandidateResolver, Database, RetrievedCell};
use crate::error::{EngineError, EngineResult};
use metadata::{
    cell_type_handle, cell_type_id, memory_type_handle, scope_handle, status_handle, status_id,
};
pub use metadata::{scope_id, CellMetadata};

const DEFAULT_BRAIN: BrainId = BrainId(1);

#[derive(Clone, Debug, Default)]
pub struct EngineAqlIndex {
    pub bitmaps: BTreeMap<BitmapHandle, BTreeSet<u32>>,
    pub lexical: BTreeMap<String, BTreeSet<u32>>,
    pub universe: BTreeSet<u32>,
    pub candidate_to_cell: BTreeMap<u32, CellId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CandidateId(pub u32);

impl Database {
    pub fn aql_index(&self) -> EngineAqlIndex {
        self.try_aql_index()
            .unwrap_or_else(|_| EngineAqlIndex::from_versions(&self.snapshot_versions()))
    }

    pub fn try_aql_index(&self) -> EngineResult<EngineAqlIndex> {
        let checkpoint_seq = CommitSeq(self.manifest().checkpoint_seq);
        let changed = self.memtable.changed_cell_ids_after(checkpoint_seq);
        if self.manifest().live_segments.is_empty() {
            return Ok(EngineAqlIndex::from_versions(&self.snapshot_versions()));
        }
        let persisted = self.persisted_index_state()?;
        Ok(EngineAqlIndex::from_persisted(
            persisted.bitmap,
            persisted.lexical,
            persisted.candidate_to_cell,
            &self.snapshot_versions(),
            &changed,
        ))
    }

    pub fn retrieve_aql(&self, aql: &str, view: &AgentView) -> EngineResult<Vec<RetrievedCell>> {
        let statement = parse_aql(aql).map_err(|error| EngineError::AqlParse(error.to_string()))?;
        let index = self.try_aql_index()?;
        let bound = Binder::new(&index, view).bind_statement(&statement)?;
        match bound {
            BoundPlan::Retrieve(plan) => self.retrieve_cells(&plan, &index),
            _ => Err(EngineError::InvalidOperation),
        }
    }
}

impl EngineAqlIndex {
    pub fn from_versions(versions: &[CellVersion]) -> Self {
        let mut sorted = versions.iter().collect::<Vec<_>>();
        sorted.sort_by_key(|version| version.cell_id);
        Self::from_cells(sorted.into_iter().enumerate().map(|(index, version)| {
            let candidate = u32::try_from(index + 1).expect("candidate id overflow");
            (candidate, version.payload.as_slice(), version.cell_id)
        }))
    }

    pub fn from_segment_cells(cells: &[SegmentCell]) -> Self {
        Self::from_cells(cells.iter().filter_map(|cell| {
            cell.deleted_seq.is_none().then_some((
                cell.candidate_id,
                cell.payload.as_slice(),
                CellId(cell.cell_id),
            ))
        }))
    }

    pub fn from_persisted(
        bitmap: BitmapIndex,
        lexical: LexicalIndex,
        candidate_to_cell: BTreeMap<u32, CellId>,
        current: &[CellVersion],
        changed: &[CellId],
    ) -> Self {
        let changed_candidates = changed
            .iter()
            .filter_map(|cell_id| candidate_for_cell(&candidate_to_cell, *cell_id))
            .collect::<BTreeSet<_>>();
        let mut index = Self {
            bitmaps: bitmap
                .bitmaps
                .into_iter()
                .map(|(handle, values)| (BitmapHandle(handle), values))
                .collect(),
            lexical: lexical.terms,
            universe: BTreeSet::new(),
            candidate_to_cell,
        };
        index.remove_candidates(&changed_candidates);
        let mut next_candidate = index
            .candidate_to_cell
            .keys()
            .next_back()
            .copied()
            .unwrap_or(0)
            .saturating_add(1);
        let changed_current = current
            .iter()
            .filter(|version| changed.contains(&version.cell_id))
            .map(|version| {
                let candidate = candidate_for_cell(&index.candidate_to_cell, version.cell_id)
                    .unwrap_or_else(|| {
                        let value = next_candidate;
                        next_candidate = next_candidate.saturating_add(1);
                        value
                    });
                (candidate, version.payload.as_slice(), version.cell_id)
            })
            .collect::<Vec<_>>();
        index.extend_cells(changed_current);
        index.rebuild_universe();
        index
    }

    pub fn bitmap_index(&self) -> BitmapIndex {
        BitmapIndex {
            bitmaps: self
                .bitmaps
                .iter()
                .map(|(handle, values)| (handle.0, values.clone()))
                .collect(),
        }
    }

    pub fn lexical_index(&self) -> LexicalIndex {
        LexicalIndex {
            terms: self.lexical.clone(),
        }
    }

    fn from_cells<'a>(cells: impl IntoIterator<Item = (u32, &'a [u8], CellId)>) -> Self {
        let mut index = Self::default();
        index.extend_cells(cells);
        index
    }

    fn extend_cells<'a>(&mut self, cells: impl IntoIterator<Item = (u32, &'a [u8], CellId)>) {
        for (candidate, payload, cell_id) in cells {
            let metadata = CellMetadata::from_payload(payload);
            self.candidate_to_cell.insert(candidate, cell_id);
            self.universe.insert(candidate);
            self.push(scope_handle(scope_id(&metadata.scope)), candidate);
            self.push(status_handle(status_id(&metadata.status)), candidate);
            self.push(
                cell_type_handle(cell_type_id(&metadata.cell_type)),
                candidate,
            );
            if let Some(memory_type) = metadata.memory_type {
                self.push(memory_type_handle(memory_type), candidate);
            }
            self.push(BitmapHandle(cell_id.0), candidate);
            for term in metadata.terms {
                self.lexical.entry(term).or_default().insert(candidate);
            }
        }
    }

    fn push(&mut self, handle: BitmapHandle, candidate: u32) {
        self.bitmaps.entry(handle).or_default().insert(candidate);
    }

    fn remove_candidates(&mut self, candidates: &BTreeSet<u32>) {
        for values in self.bitmaps.values_mut() {
            values.retain(|candidate| !candidates.contains(candidate));
        }
        self.bitmaps.retain(|_, values| !values.is_empty());
        for values in self.lexical.values_mut() {
            values.retain(|candidate| !candidates.contains(candidate));
        }
        self.lexical.retain(|_, values| !values.is_empty());
        self.candidate_to_cell
            .retain(|candidate, _| !candidates.contains(candidate));
    }

    fn rebuild_universe(&mut self) {
        self.universe = self
            .bitmaps
            .values()
            .flat_map(|values| values.iter().copied())
            .collect();
    }
}

fn candidate_for_cell(candidate_to_cell: &BTreeMap<u32, CellId>, cell_id: CellId) -> Option<u32> {
    candidate_to_cell
        .iter()
        .find_map(|(candidate, mapped)| (*mapped == cell_id).then_some(*candidate))
}

impl AqlCatalog for EngineAqlIndex {
    fn resolve_brain(&self, _name: &str) -> Option<BrainId> {
        Some(DEFAULT_BRAIN)
    }

    fn resolve_scope(&self, _brain: BrainId, name: &str) -> Option<ScopeId> {
        Some(scope_id(name))
    }

    fn resolve_status(&self, _brain: BrainId, status: &str) -> Option<StatusId> {
        Some(status_id(status))
    }

    fn resolve_cell_type(&self, _brain: BrainId, cell_type: &str) -> Option<CellTypeId> {
        Some(cell_type_id(cell_type))
    }

    fn scope_bitmap(&self, _brain: BrainId, scope: ScopeId) -> Option<BitmapHandle> {
        Some(scope_handle(scope))
    }

    fn status_bitmap(&self, _brain: BrainId, status: StatusId) -> Option<BitmapHandle> {
        Some(status_handle(status))
    }

    fn cell_type_bitmap(&self, _brain: BrainId, cell_type: CellTypeId) -> Option<BitmapHandle> {
        Some(cell_type_handle(cell_type))
    }

    fn memory_type_bitmap(&self, _brain: BrainId, memory_type: MemoryType) -> Option<BitmapHandle> {
        Some(memory_type_handle(memory_type))
    }

    fn field_is_filterable(&self, _brain: BrainId, field: &str) -> bool {
        matches!(
            field,
            "space" | "scope" | "status" | "type" | "cell_type" | "memory_type"
        )
    }

    fn bitmap_estimated_cardinality(&self, _brain: BrainId, handle: BitmapHandle) -> Option<u64> {
        self.bitmaps.get(&handle).map(|values| values.len() as u64)
    }
}

impl BitmapProvider for EngineAqlIndex {
    fn bitmap(&self, handle: BitmapHandle) -> Option<BTreeSet<u32>> {
        Some(self.bitmaps.get(&handle).cloned().unwrap_or_default())
    }

    fn agent_allowed(&self) -> BTreeSet<u32> {
        self.universe.clone()
    }

    fn live(&self) -> BTreeSet<u32> {
        self.universe.clone()
    }

    fn universe(&self) -> BTreeSet<u32> {
        self.universe.clone()
    }
}

impl CandidateResolver for EngineAqlIndex {
    fn cell_id_for_candidate(&self, candidate: u32) -> Option<CellId> {
        self.candidate_to_cell.get(&candidate).copied()
    }
}
