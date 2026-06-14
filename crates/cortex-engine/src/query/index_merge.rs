use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::BitmapHandle;
use cortex_core::memtable::CellVersion;
use cortex_core::CellId;
use cortex_storage::indexes::{BitmapIndex, LexicalIndex};

use super::candidates::{increment_candidate, next_candidate_after, reverse_candidate_map};
use super::metadata::CellMetadata;
use super::{AqlDeltaIndex, EngineAqlIndex};
use crate::error::EngineResult;
use crate::search::TextAnalyzer;

impl EngineAqlIndex {
    pub fn from_persisted(
        bitmap: BitmapIndex,
        lexical: LexicalIndex,
        candidate_to_cell: BTreeMap<u32, CellId>,
        current: &[CellVersion],
        changed: &[CellId],
    ) -> EngineResult<Self> {
        Self::from_persisted_refs(bitmap, lexical, candidate_to_cell, current.iter(), changed)
    }

    pub(crate) fn from_persisted_refs<'a>(
        bitmap: BitmapIndex,
        lexical: LexicalIndex,
        candidate_to_cell: BTreeMap<u32, CellId>,
        current: impl IntoIterator<Item = &'a CellVersion>,
        changed: &[CellId],
    ) -> EngineResult<Self> {
        Self::from_persisted_refs_with_analyzer(
            bitmap,
            lexical,
            candidate_to_cell,
            current,
            changed,
            &TextAnalyzer::default(),
        )
    }

    pub(crate) fn from_persisted_refs_with_analyzer<'a>(
        bitmap: BitmapIndex,
        lexical: LexicalIndex,
        candidate_to_cell: BTreeMap<u32, CellId>,
        current: impl IntoIterator<Item = &'a CellVersion>,
        changed: &[CellId],
        analyzer: &TextAnalyzer,
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
        let mut index = index_from_persisted(bitmap, lexical, candidate_to_cell, cell_to_candidate);
        index.remove_candidates(&changed_candidates);

        let mut next_candidate = None;
        let mut changed_current = Vec::new();
        for version in current
            .into_iter()
            .filter(|version| changed.contains(&version.cell_id))
        {
            let candidate = candidate_for_changed_cell(
                version.cell_id,
                &changed_cell_candidates,
                &mut next_candidate,
                &index,
            )?;
            changed_current.push((
                candidate,
                CellMetadata::from_payload_with_descriptor(&version.payload, &version.descriptor),
                version.cell_id,
            ));
        }
        index.extend_cells_with_analyzer(changed_current, analyzer)?;
        index.rebuild_universe();
        Ok(index)
    }

    pub(crate) fn from_persisted_delta_with_analyzer(
        bitmap: BitmapIndex,
        lexical: LexicalIndex,
        candidate_to_cell: BTreeMap<u32, CellId>,
        delta: &AqlDeltaIndex,
        analyzer: &TextAnalyzer,
    ) -> EngineResult<Self> {
        let cell_to_candidate = reverse_candidate_map(&candidate_to_cell)?;
        let changed_candidates = delta
            .changed_cell_ids()
            .iter()
            .filter_map(|cell_id| cell_to_candidate.get(cell_id).copied())
            .collect::<BTreeSet<_>>();
        let changed_cell_candidates = delta
            .changed_cell_ids()
            .iter()
            .filter_map(|cell_id| {
                cell_to_candidate
                    .get(cell_id)
                    .map(|candidate| (*cell_id, *candidate))
            })
            .collect::<BTreeMap<_, _>>();
        let mut index = index_from_persisted(bitmap, lexical, candidate_to_cell, cell_to_candidate);
        index.remove_candidates(&changed_candidates);

        let mut next_candidate = None;
        let mut changed_current = Vec::new();
        for (cell_id, metadata) in delta.live_cells() {
            let candidate = candidate_for_changed_cell(
                cell_id,
                &changed_cell_candidates,
                &mut next_candidate,
                &index,
            )?;
            changed_current.push((candidate, metadata.clone(), cell_id));
        }
        index.extend_cells_with_analyzer(changed_current, analyzer)?;
        index.rebuild_universe();
        Ok(index)
    }
}

fn index_from_persisted(
    bitmap: BitmapIndex,
    lexical: LexicalIndex,
    candidate_to_cell: BTreeMap<u32, CellId>,
    cell_to_candidate: BTreeMap<CellId, u32>,
) -> EngineAqlIndex {
    EngineAqlIndex {
        bitmaps: bitmap
            .bitmaps
            .into_iter()
            .map(|(handle, values)| (BitmapHandle(handle), values.iter().collect()))
            .collect(),
        lexical: lexical.terms,
        lexical_doc_lengths: lexical.doc_lengths,
        lexical_term_frequencies: lexical.term_frequencies,
        lexical_field_doc_lengths: lexical.field_doc_lengths,
        lexical_field_term_frequencies: lexical.field_term_frequencies,
        universe: BTreeSet::new(),
        candidate_to_cell,
        cell_to_candidate,
        permission_pruning: Default::default(),
        segment_pruning: Default::default(),
    }
}

fn candidate_for_changed_cell(
    cell_id: CellId,
    changed_cell_candidates: &BTreeMap<CellId, u32>,
    next_candidate: &mut Option<u32>,
    index: &EngineAqlIndex,
) -> EngineResult<u32> {
    if let Some(candidate) = changed_cell_candidates.get(&cell_id) {
        return Ok(*candidate);
    }
    let value = *next_candidate.get_or_insert(next_candidate_after(
        index.candidate_to_cell.keys().copied(),
    )?);
    *next_candidate = Some(increment_candidate(value)?);
    Ok(value)
}
