use std::collections::{BTreeMap, BTreeSet};

pub(crate) mod cache;
mod candidates;
mod catalog;
mod explain;
pub(crate) mod metadata;
mod metadata_validation;
mod provider;
mod render;

use cortex_aql::{AgentView, BitmapHandle, BoundPlan, BrainId};
use cortex_core::memtable::CellVersion;
use cortex_core::{CellId, CommitSeq};
use cortex_storage::indexes::{BitmapIndex, LexicalIndex};
use cortex_storage::segment::SegmentCell;

use crate::database::{Database, RetrievedCell};
use crate::error::{EngineError, EngineResult};
pub use cache::AqlQueryCacheStats;
use candidates::{
    candidate_from_ordinal, increment_candidate, next_candidate_after, reverse_candidate_map,
    validate_candidate,
};
pub use explain::{AqlCandidateCounts, AqlExplainFilter, AqlExplainReport};
use metadata::{
    cell_type_handle, cell_type_id, memory_type_handle, scope_handle, status_handle, status_id,
};
pub use metadata::{scope_id, CellMetadata};
pub use provider::EngineAqlProvider;

const DEFAULT_BRAIN: BrainId = BrainId(1);

#[derive(Clone, Debug, Default)]
pub struct EngineAqlIndex {
    pub bitmaps: BTreeMap<BitmapHandle, BTreeSet<u32>>,
    pub lexical: BTreeMap<String, BTreeSet<u32>>,
    pub lexical_doc_lengths: BTreeMap<u32, u32>,
    pub lexical_term_frequencies: BTreeMap<String, BTreeMap<u32, u32>>,
    pub universe: BTreeSet<u32>,
    pub candidate_to_cell: BTreeMap<u32, CellId>,
    pub cell_to_candidate: BTreeMap<CellId, u32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CandidateId(pub u32);

impl Database {
    pub fn aql_index(&self) -> EngineResult<EngineAqlIndex> {
        self.try_aql_index()
    }

    pub fn try_aql_index(&self) -> EngineResult<EngineAqlIndex> {
        let checkpoint_seq = CommitSeq(self.manifest().checkpoint_seq);
        let changed = self.memtable.changed_cell_ids_after(checkpoint_seq);
        if self.manifest().live_segments.is_empty() {
            return EngineAqlIndex::try_from_versions(&self.snapshot_versions());
        }
        let persisted = self.persisted_index_state()?;
        EngineAqlIndex::from_persisted(
            persisted.bitmap,
            persisted.lexical,
            persisted.candidate_to_cell,
            &self.snapshot_versions(),
            &changed,
        )
    }

    pub fn retrieve_aql(&self, aql: &str, view: &AgentView) -> EngineResult<Vec<RetrievedCell>> {
        let (cached, index) = self.bind_aql_cached(aql, view)?;
        if cached.statement_kind != cache::AqlStatementKind::Retrieve {
            return Err(EngineError::InvalidOperation);
        }
        match cached.bound_plan {
            BoundPlan::Retrieve(plan) => {
                let provider = EngineAqlProvider::new(index, view);
                self.retrieve_cells(&plan, &provider)
            }
            _ => Err(EngineError::InvalidOperation),
        }
    }
}

impl EngineAqlIndex {
    pub fn try_from_versions(versions: &[CellVersion]) -> EngineResult<Self> {
        let mut sorted = versions.iter().collect::<Vec<_>>();
        sorted.sort_by_key(|version| version.cell_id);
        let cells = sorted
            .into_iter()
            .enumerate()
            .map(|(index, version)| {
                Ok((
                    candidate_from_ordinal(index)?,
                    version.payload.as_slice(),
                    version.cell_id,
                ))
            })
            .collect::<EngineResult<Vec<_>>>()?;
        Self::try_from_cells(cells)
    }

    pub fn try_from_segment_cells(cells: &[SegmentCell]) -> EngineResult<Self> {
        Self::try_from_cells(cells.iter().filter_map(|cell| {
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
    ) -> EngineResult<Self> {
        let cell_to_candidate = reverse_candidate_map(&candidate_to_cell)?;
        let changed_candidates = changed
            .iter()
            .filter_map(|cell_id| cell_to_candidate.get(cell_id).copied())
            .collect::<BTreeSet<_>>();
        let changed_cell_candidates = changed
            .iter()
            .filter_map(|cell_id| {
                cell_to_candidate
                    .get(cell_id)
                    .map(|candidate| (*cell_id, *candidate))
            })
            .collect::<BTreeMap<_, _>>();
        let mut index = Self {
            bitmaps: bitmap
                .bitmaps
                .into_iter()
                .map(|(handle, values)| (BitmapHandle(handle), values))
                .collect(),
            lexical: lexical.terms,
            lexical_doc_lengths: lexical.doc_lengths,
            lexical_term_frequencies: lexical.term_frequencies,
            universe: BTreeSet::new(),
            candidate_to_cell,
            cell_to_candidate,
        };
        index.remove_candidates(&changed_candidates);
        let mut next_candidate = None;
        let mut changed_current = Vec::new();
        for version in current
            .iter()
            .filter(|version| changed.contains(&version.cell_id))
        {
            let candidate = if let Some(candidate) = changed_cell_candidates.get(&version.cell_id) {
                *candidate
            } else {
                let value = *next_candidate.get_or_insert(next_candidate_after(
                    index.candidate_to_cell.keys().copied(),
                )?);
                next_candidate = Some(increment_candidate(value)?);
                value
            };
            changed_current.push((candidate, version.payload.as_slice(), version.cell_id));
        }
        index.extend_cells(changed_current)?;
        index.rebuild_universe();
        Ok(index)
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
            doc_lengths: self.lexical_doc_lengths.clone(),
            term_frequencies: self.lexical_term_frequencies.clone(),
        }
    }

    fn try_from_cells<'a>(
        cells: impl IntoIterator<Item = (u32, &'a [u8], CellId)>,
    ) -> EngineResult<Self> {
        let mut index = Self::default();
        index.extend_cells(cells)?;
        Ok(index)
    }

    fn extend_cells<'a>(
        &mut self,
        cells: impl IntoIterator<Item = (u32, &'a [u8], CellId)>,
    ) -> EngineResult<()> {
        for (candidate, payload, cell_id) in cells {
            validate_candidate(candidate)?;
            if let Some(mapped) = self.candidate_to_cell.get(&candidate) {
                if *mapped != cell_id {
                    return Err(EngineError::StorageInvariant(format!(
                        "candidate {candidate} maps to multiple cells"
                    )));
                }
            }
            if let Some(mapped) = self.cell_to_candidate.get(&cell_id) {
                if *mapped != candidate {
                    return Err(EngineError::StorageInvariant(format!(
                        "cell {} maps to multiple candidates",
                        cell_id.0
                    )));
                }
            }
            let metadata = CellMetadata::from_payload(payload);
            self.candidate_to_cell.insert(candidate, cell_id);
            self.cell_to_candidate.insert(cell_id, candidate);
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
            let weighted_terms = metadata.weighted_lexical_terms();
            let doc_length = weighted_terms.values().copied().sum::<u32>().max(1);
            self.lexical_doc_lengths.insert(candidate, doc_length);
            for (term, frequency) in weighted_terms {
                self.lexical
                    .entry(term.clone())
                    .or_default()
                    .insert(candidate);
                self.lexical_term_frequencies
                    .entry(term)
                    .or_default()
                    .insert(candidate, frequency);
            }
        }
        Ok(())
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
        self.lexical_doc_lengths
            .retain(|candidate, _| !candidates.contains(candidate));
        for values in self.lexical_term_frequencies.values_mut() {
            values.retain(|candidate, _| !candidates.contains(candidate));
        }
        self.lexical_term_frequencies
            .retain(|_, values| !values.is_empty());
        self.candidate_to_cell
            .retain(|candidate, _| !candidates.contains(candidate));
        self.cell_to_candidate
            .retain(|_, candidate| !candidates.contains(candidate));
    }

    fn rebuild_universe(&mut self) {
        self.universe = self.candidate_to_cell.keys().copied().collect();
    }
}
