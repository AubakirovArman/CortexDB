use cortex_aql::AqlCatalog;
use cortex_storage::manifest::{
    ManifestCount, ManifestSegment, ManifestSegmentStats, ManifestTermDocumentFrequency,
    StorageManifest,
};

use super::*;

#[test]
fn statistics_estimates_live_scope_status_type_and_rows() {
    let manifest = StorageManifest {
        live_segments: vec![
            ManifestSegment {
                id: 1,
                generation: 1,
                checkpoint_seq: 10,
                cell_count: 3,
            },
            ManifestSegment {
                id: 2,
                generation: 2,
                checkpoint_seq: 20,
                cell_count: 2,
            },
        ],
        segment_stats: vec![
            stats(
                1,
                3,
                &[("project:a", 2), ("project:b", 1)],
                &[("ready", 1), ("draft", 2)],
                &[("fact", 3)],
                &[("budget", 2)],
            ),
            stats(
                2,
                2,
                &[("project:a", 1)],
                &[("ready", 2)],
                &[("document_block", 2)],
                &[("budget", 1)],
            ),
            stats(9, 100, &[("retired", 100)], &[], &[], &[("budget", 100)]),
        ],
        ..StorageManifest::default()
    };
    let statistics = DatabaseStatistics::new(&manifest);

    assert_eq!(statistics.live_segment_row_count(), 5);
    assert_eq!(statistics.estimate_scope_cardinality("project:a"), Some(3));
    assert_eq!(statistics.estimate_scope_cardinality("missing"), Some(0));
    assert_eq!(statistics.estimate_status_cardinality("ready"), Some(3));
    assert_eq!(statistics.estimate_cell_type_cardinality("fact"), Some(3));
    assert_eq!(
        statistics.estimate_term_document_frequency("budget"),
        Some(3)
    );
}

#[test]
fn stats_catalog_estimates_bitmap_handles_without_materialized_bitmaps() {
    let manifest = StorageManifest {
        live_segments: vec![ManifestSegment {
            id: 1,
            generation: 1,
            checkpoint_seq: 10,
            cell_count: 3,
        }],
        segment_stats: vec![stats(
            1,
            3,
            &[("project:a", 2)],
            &[("ready", 1)],
            &[("fact", 3)],
            &[],
        )],
        ..StorageManifest::default()
    };
    let catalog = EngineAqlStatsCatalog::new(DatabaseStatistics::new(&manifest));

    assert_eq!(
        catalog.bitmap_estimated_cardinality(
            DEFAULT_BRAIN,
            catalog
                .scope_bitmap(DEFAULT_BRAIN, scope_id("project:a"))
                .unwrap()
        ),
        Some(2)
    );
    assert_eq!(
        catalog.bitmap_estimated_cardinality(
            DEFAULT_BRAIN,
            catalog
                .status_bitmap(DEFAULT_BRAIN, status_id("ready"))
                .unwrap()
        ),
        Some(1)
    );
    assert_eq!(
        catalog.bitmap_estimated_cardinality(
            DEFAULT_BRAIN,
            catalog
                .cell_type_bitmap(DEFAULT_BRAIN, cell_type_id("fact"))
                .unwrap()
        ),
        Some(3)
    );
    assert_eq!(
        catalog.bitmap_estimated_cardinality(
            DEFAULT_BRAIN,
            catalog
                .memory_type_bitmap(DEFAULT_BRAIN, MemoryType::Decision)
                .unwrap()
        ),
        None
    );
}

fn stats(
    segment_id: u64,
    row_count: u64,
    scopes: &[(&str, u64)],
    statuses: &[(&str, u64)],
    types: &[(&str, u64)],
    terms: &[(&str, u64)],
) -> ManifestSegmentStats {
    ManifestSegmentStats {
        segment_id,
        row_count,
        scope_counts: counts(scopes),
        status_counts: counts(statuses),
        type_counts: counts(types),
        top_terms: terms
            .iter()
            .map(|(term, document_frequency)| ManifestTermDocumentFrequency {
                term: (*term).to_owned(),
                document_frequency: *document_frequency,
            })
            .collect(),
        ..ManifestSegmentStats::default()
    }
}

fn counts(values: &[(&str, u64)]) -> Vec<ManifestCount> {
    values
        .iter()
        .map(|(key, count)| ManifestCount {
            key: (*key).to_owned(),
            count: *count,
        })
        .collect()
}
