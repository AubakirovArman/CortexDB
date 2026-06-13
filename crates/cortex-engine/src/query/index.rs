use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::BitmapHandle;
use cortex_core::memtable::CellVersion;
use cortex_core::{CellDescriptor, CellId};
use cortex_storage::indexes::{BitmapIndex, LexicalIndex};
use cortex_storage::segment::{SegmentCell, SegmentCellRef};

use super::candidates::{
    candidate_from_ordinal, increment_candidate, next_candidate_after, reverse_candidate_map,
    validate_candidate,
};
use super::metadata::{
    cell_type_handle, cell_type_id, memory_type_handle, scope_handle, scope_id, status_handle,
    status_id, CellMetadata,
};
use super::EngineAqlIndex;
use crate::error::{EngineError, EngineResult};

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
                    CellMetadata::from_payload_with_descriptor(
                        &version.payload,
                        &version.descriptor,
                    ),
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
                CellMetadata::from_payload(&cell.payload),
                CellId(cell.cell_id),
            ))
        }))
    }

    pub fn try_from_segment_cell_refs(cells: &[SegmentCellRef<'_>]) -> EngineResult<Self> {
        Self::try_from_cells(cells.iter().filter_map(|cell| {
            let metadata = cell
                .descriptor
                .as_deref()
                .and_then(CellDescriptor::decode_section_v1)
                .map(|descriptor| {
                    CellMetadata::from_payload_with_descriptor(cell.payload, &descriptor)
                })
                .unwrap_or_else(|| CellMetadata::from_payload(cell.payload));
            cell.deleted_seq.is_none().then_some((
                cell.candidate_id,
                metadata,
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
            lexical_field_doc_lengths: lexical.field_doc_lengths,
            lexical_field_term_frequencies: lexical.field_term_frequencies,
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
            changed_current.push((
                candidate,
                CellMetadata::from_payload_with_descriptor(&version.payload, &version.descriptor),
                version.cell_id,
            ));
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
            field_doc_lengths: self.lexical_field_doc_lengths.clone(),
            field_term_frequencies: self.lexical_field_term_frequencies.clone(),
        }
    }

    fn try_from_cells(
        cells: impl IntoIterator<Item = (u32, CellMetadata, CellId)>,
    ) -> EngineResult<Self> {
        let mut index = Self::default();
        index.extend_cells(cells)?;
        Ok(index)
    }

    fn extend_cells(
        &mut self,
        cells: impl IntoIterator<Item = (u32, CellMetadata, CellId)>,
    ) -> EngineResult<()> {
        for (candidate, metadata, cell_id) in cells {
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
            for (field, terms) in metadata.lexical_field_terms() {
                let field_length = terms.values().copied().sum::<u32>().max(1);
                self.lexical_field_doc_lengths
                    .entry(field.clone())
                    .or_default()
                    .insert(candidate, field_length);
                let field_frequencies = self
                    .lexical_field_term_frequencies
                    .entry(field)
                    .or_default();
                for (term, frequency) in terms {
                    field_frequencies
                        .entry(term)
                        .or_default()
                        .insert(candidate, frequency);
                }
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
        for values in self.lexical_field_doc_lengths.values_mut() {
            values.retain(|candidate, _| !candidates.contains(candidate));
        }
        self.lexical_field_doc_lengths
            .retain(|_, values| !values.is_empty());
        for terms in self.lexical_field_term_frequencies.values_mut() {
            for values in terms.values_mut() {
                values.retain(|candidate, _| !candidates.contains(candidate));
            }
            terms.retain(|_, values| !values.is_empty());
        }
        self.lexical_field_term_frequencies
            .retain(|_, terms| !terms.is_empty());
        self.candidate_to_cell
            .retain(|candidate, _| !candidates.contains(candidate));
        self.cell_to_candidate
            .retain(|_, candidate| !candidates.contains(candidate));
    }

    fn rebuild_universe(&mut self) {
        self.universe = self.candidate_to_cell.keys().copied().collect();
    }
}
