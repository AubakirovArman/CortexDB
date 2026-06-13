use std::collections::BTreeSet;

use cortex_aql::BitmapHandle;
use cortex_core::memtable::CellVersion;
use cortex_core::{CellDescriptor, CellId};
use cortex_storage::indexes::{BitmapIndex, LexicalIndex};
use cortex_storage::segment::{SegmentCell, SegmentCellRef};

use super::candidates::{candidate_from_ordinal, validate_candidate};
use super::metadata::{
    cell_type_handle, cell_type_id, memory_type_handle, scope_handle, scope_id, status_handle,
    status_id, CellMetadata,
};
use super::{AqlDeltaIndex, EngineAqlIndex};
use crate::error::{EngineError, EngineResult};

impl EngineAqlIndex {
    pub fn try_from_versions(versions: &[CellVersion]) -> EngineResult<Self> {
        Self::try_from_version_refs(versions.iter())
    }

    pub(crate) fn try_from_version_refs<'a>(
        versions: impl IntoIterator<Item = &'a CellVersion>,
    ) -> EngineResult<Self> {
        let mut sorted = versions.into_iter().collect::<Vec<_>>();
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

    pub(crate) fn try_from_delta(delta: &AqlDeltaIndex) -> EngineResult<Self> {
        let cells = delta
            .live_cells()
            .enumerate()
            .map(|(index, (cell_id, metadata))| {
                Ok((candidate_from_ordinal(index)?, metadata.clone(), cell_id))
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

    pub(super) fn extend_cells(
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

    pub(super) fn remove_candidates(&mut self, candidates: &BTreeSet<u32>) {
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

    pub(super) fn rebuild_universe(&mut self) {
        self.universe = self.candidate_to_cell.keys().copied().collect();
    }
}

#[cfg(test)]
mod tests {
    use cortex_core::{CellId, CommitSeq};

    use super::*;

    #[test]
    fn borrowed_version_index_matches_owned_version_index() {
        let versions = vec![
            CellVersion::new(
                CellId(2),
                CommitSeq(1),
                b"scope=project:alpha\nstatus=ready\ntype=fact\n\nbeta term".to_vec(),
                0,
            ),
            CellVersion::new(
                CellId(1),
                CommitSeq(2),
                b"scope=project:alpha\nstatus=ready\ntype=document_block\n\nalpha term".to_vec(),
                0,
            ),
        ];

        let owned = EngineAqlIndex::try_from_versions(&versions).unwrap();
        let borrowed = EngineAqlIndex::try_from_version_refs(versions.iter()).unwrap();

        assert_eq!(owned.bitmaps, borrowed.bitmaps);
        assert_eq!(owned.lexical, borrowed.lexical);
        assert_eq!(owned.lexical_doc_lengths, borrowed.lexical_doc_lengths);
        assert_eq!(
            owned.lexical_term_frequencies,
            borrowed.lexical_term_frequencies
        );
        assert_eq!(owned.candidate_to_cell, borrowed.candidate_to_cell);
        assert_eq!(owned.cell_to_candidate, borrowed.cell_to_candidate);
        assert_eq!(owned.universe, borrowed.universe);
    }
}
