use std::collections::{BTreeMap, BTreeSet};

use cortex_core::CellDescriptor;
use cortex_storage::manifest::{
    ManifestCount, ManifestSegmentStats, ManifestTermDocumentFrequency,
};
use cortex_storage::segment::SegmentCellRef;

use crate::query::CellMetadata;

const TOP_TERM_LIMIT: usize = 32;

pub(crate) fn segment_stats_from_cell_refs(
    segment_id: u64,
    cells: &[SegmentCellRef<'_>],
) -> ManifestSegmentStats {
    let mut stats = SegmentStatsBuilder::new(segment_id, cells.len() as u64);
    for cell in cells {
        if cell.deleted_seq.is_some() {
            continue;
        }
        let metadata = cell_metadata(cell);
        stats.add_metadata(&metadata);
    }
    stats.finish()
}

fn cell_metadata(cell: &SegmentCellRef<'_>) -> CellMetadata {
    if let Some(descriptor) = cell
        .descriptor
        .as_deref()
        .and_then(CellDescriptor::decode_section_v1)
    {
        CellMetadata::from_payload_with_descriptor(cell.payload, &descriptor)
    } else {
        CellMetadata::from_payload(cell.payload)
    }
}

struct SegmentStatsBuilder {
    segment_id: u64,
    row_count: u64,
    min_created_unix_seconds: Option<u64>,
    max_created_unix_seconds: Option<u64>,
    scope_counts: BTreeMap<String, u64>,
    status_counts: BTreeMap<String, u64>,
    type_counts: BTreeMap<String, u64>,
    term_document_frequencies: BTreeMap<String, u64>,
}

impl SegmentStatsBuilder {
    fn new(segment_id: u64, row_count: u64) -> Self {
        Self {
            segment_id,
            row_count,
            min_created_unix_seconds: None,
            max_created_unix_seconds: None,
            scope_counts: BTreeMap::new(),
            status_counts: BTreeMap::new(),
            type_counts: BTreeMap::new(),
            term_document_frequencies: BTreeMap::new(),
        }
    }

    fn add_metadata(&mut self, metadata: &CellMetadata) {
        increment(&mut self.scope_counts, &metadata.scope);
        increment(&mut self.status_counts, &metadata.status);
        increment(&mut self.type_counts, &metadata.cell_type);
        if let Some(created) = metadata.created_unix_seconds {
            self.min_created_unix_seconds = Some(
                self.min_created_unix_seconds
                    .map_or(created, |existing| existing.min(created)),
            );
            self.max_created_unix_seconds = Some(
                self.max_created_unix_seconds
                    .map_or(created, |existing| existing.max(created)),
            );
        }

        let unique_terms = metadata.terms.iter().collect::<BTreeSet<_>>();
        for term in unique_terms {
            increment(&mut self.term_document_frequencies, term);
        }
    }

    fn finish(self) -> ManifestSegmentStats {
        let mut top_terms = self
            .term_document_frequencies
            .into_iter()
            .map(|(term, document_frequency)| ManifestTermDocumentFrequency {
                term,
                document_frequency,
            })
            .collect::<Vec<_>>();
        top_terms.sort_by(|left, right| {
            right
                .document_frequency
                .cmp(&left.document_frequency)
                .then_with(|| left.term.cmp(&right.term))
        });
        top_terms.truncate(TOP_TERM_LIMIT);

        ManifestSegmentStats {
            segment_id: self.segment_id,
            row_count: self.row_count,
            min_created_unix_seconds: self.min_created_unix_seconds,
            max_created_unix_seconds: self.max_created_unix_seconds,
            scope_counts: counts_from_map(self.scope_counts),
            status_counts: counts_from_map(self.status_counts),
            type_counts: counts_from_map(self.type_counts),
            top_terms,
        }
    }
}

fn increment(counts: &mut BTreeMap<String, u64>, key: &str) {
    *counts.entry(key.to_owned()).or_default() += 1;
}

fn counts_from_map(counts: BTreeMap<String, u64>) -> Vec<ManifestCount> {
    counts
        .into_iter()
        .map(|(key, count)| ManifestCount { key, count })
        .collect()
}

#[cfg(test)]
mod tests {
    use cortex_core::KnowledgeCellType;
    use cortex_storage::segment::SegmentCellRef;

    use super::*;

    #[test]
    fn segment_stats_count_metadata_created_range_and_top_terms() {
        let first = descriptor("project:a", "ready", KnowledgeCellType::Fact, Some(10));
        let second = descriptor(
            "project:a",
            "draft",
            KnowledgeCellType::DocumentBlock,
            Some(30),
        );
        let third = descriptor("project:b", "ready", KnowledgeCellType::Fact, Some(20));
        let cells = vec![
            cell(1, Some(first), b"budget runway budget"),
            cell(2, Some(second), b"runway schedule"),
            cell(3, Some(third), b"budget finance"),
            SegmentCellRef {
                candidate_id: 4,
                cell_id: 4,
                created_seq: 4,
                deleted_seq: Some(4),
                descriptor: None,
                payload: &[],
            },
        ];

        let stats = segment_stats_from_cell_refs(7, &cells);

        assert_eq!(stats.segment_id, 7);
        assert_eq!(stats.row_count, 4);
        assert_eq!(stats.min_created_unix_seconds, Some(10));
        assert_eq!(stats.max_created_unix_seconds, Some(30));
        assert_eq!(count_for(&stats.scope_counts, "project:a"), Some(2));
        assert_eq!(count_for(&stats.scope_counts, "project:b"), Some(1));
        assert_eq!(count_for(&stats.status_counts, "ready"), Some(2));
        assert_eq!(count_for(&stats.status_counts, "draft"), Some(1));
        assert_eq!(count_for(&stats.type_counts, "fact"), Some(2));
        assert_eq!(count_for(&stats.type_counts, "document_block"), Some(1));
        assert_eq!(stats.top_terms[0].term, "budget");
        assert_eq!(stats.top_terms[0].document_frequency, 2);
        assert_eq!(stats.top_terms[1].term, "runway");
        assert_eq!(stats.top_terms[1].document_frequency, 2);
    }

    #[test]
    fn segment_stats_falls_back_to_payload_metadata_without_descriptor() {
        let payload =
            b"scope=legacy\nstatus=ready\ntype=fact\ncreated_unix_seconds=44\n\nlegacy body";
        let cells = [cell(1, None, payload)];

        let stats = segment_stats_from_cell_refs(9, &cells);

        assert_eq!(count_for(&stats.scope_counts, "legacy"), Some(1));
        assert_eq!(stats.min_created_unix_seconds, Some(44));
        assert_eq!(stats.max_created_unix_seconds, Some(44));
    }

    fn descriptor(
        scope: &str,
        status: &str,
        cell_type: KnowledgeCellType,
        created_unix_seconds: Option<u64>,
    ) -> Vec<u8> {
        CellDescriptor {
            scope: scope.to_owned(),
            status: status.to_owned(),
            cell_type,
            created_unix_seconds,
            ..CellDescriptor::default()
        }
        .encode_section_v1()
    }

    fn cell<'a>(
        candidate_id: u32,
        descriptor: Option<Vec<u8>>,
        payload: &'a [u8],
    ) -> SegmentCellRef<'a> {
        SegmentCellRef {
            candidate_id,
            cell_id: u64::from(candidate_id),
            created_seq: u64::from(candidate_id),
            deleted_seq: None,
            descriptor,
            payload,
        }
    }

    fn count_for(counts: &[ManifestCount], key: &str) -> Option<u64> {
        counts
            .iter()
            .find(|count| count.key == key)
            .map(|count| count.count)
    }
}
