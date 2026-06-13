use cortex_storage::manifest::{
    ManifestCount, ManifestHnswNoFallbackProfile, ManifestHnswProfile, ManifestSegment,
    ManifestSegmentStats, ManifestTermDocumentFrequency, ManifestVectorProfile, StorageManifest,
};

#[test]
fn manifest_hnsw_profile_roundtrips() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("manifest.acm");
    let manifest = StorageManifest {
        hnsw_profile: Some(ManifestHnswProfile {
            max_neighbors: 24,
            ef_search: 192,
            layer_count: 5,
            metric: 1,
            ef_construction: 256,
        }),
        ..StorageManifest::default()
    };

    manifest.store(&path).unwrap();

    assert_eq!(StorageManifest::load(&path).unwrap(), manifest);
}

#[test]
fn manifest_vector_profile_roundtrips() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("manifest.acm");
    let manifest = StorageManifest {
        vector_profile: Some(ManifestVectorProfile {
            dimension: 384,
            metric: 1,
        }),
        ..StorageManifest::default()
    };

    manifest.store(&path).unwrap();

    assert_eq!(StorageManifest::load(&path).unwrap(), manifest);
}

#[test]
fn manifest_hnsw_no_fallback_profile_roundtrips() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("manifest.acm");
    let manifest = StorageManifest {
        hnsw_no_fallback_profile: Some(ManifestHnswNoFallbackProfile {
            rollout_enabled: true,
            min_recall_q16: 65_535,
            require_upper_layers: true,
        }),
        ..StorageManifest::default()
    };

    manifest.store(&path).unwrap();

    assert_eq!(StorageManifest::load(&path).unwrap(), manifest);
}

#[test]
fn manifest_segment_stats_roundtrips() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("manifest.acm");
    let manifest = StorageManifest {
        segment_stats: vec![ManifestSegmentStats {
            segment_id: 7,
            row_count: 100,
            min_created_unix_seconds: Some(10),
            max_created_unix_seconds: Some(99),
            scope_counts: vec![ManifestCount {
                key: "project:alpha".to_owned(),
                count: 80,
            }],
            status_counts: vec![ManifestCount {
                key: "ready".to_owned(),
                count: 90,
            }],
            type_counts: vec![ManifestCount {
                key: "fact".to_owned(),
                count: 70,
            }],
            top_terms: vec![ManifestTermDocumentFrequency {
                term: "budget".to_owned(),
                document_frequency: 42,
            }],
        }],
        ..StorageManifest::default()
    };

    manifest.store(&path).unwrap();

    assert_eq!(StorageManifest::load(&path).unwrap(), manifest);
}

#[test]
fn manifest_segment_stats_helpers_replace_and_retire_by_segment_id() {
    let mut manifest = StorageManifest {
        live_segments: vec![
            ManifestSegment {
                id: 1,
                generation: 1,
                checkpoint_seq: 10,
                cell_count: 10,
            },
            ManifestSegment {
                id: 2,
                generation: 2,
                checkpoint_seq: 20,
                cell_count: 10,
            },
        ],
        ..StorageManifest::default()
    };

    manifest.set_segment_stats(ManifestSegmentStats {
        segment_id: 2,
        row_count: 5,
        ..ManifestSegmentStats::default()
    });
    manifest.set_segment_stats(ManifestSegmentStats {
        segment_id: 2,
        row_count: 9,
        ..ManifestSegmentStats::default()
    });

    assert_eq!(manifest.segment_stats.len(), 1);
    assert_eq!(manifest.stats_for_segment(2).unwrap().row_count, 9);

    manifest.replace_segments(
        vec![
            manifest.live_segments[0].clone(),
            manifest.live_segments[1].clone(),
        ],
        ManifestSegment {
            id: 3,
            generation: 3,
            checkpoint_seq: 20,
            cell_count: 20,
        },
    );

    assert!(manifest.stats_for_segment(2).is_none());
}

#[test]
fn manifest_segment_stats_are_retired_by_full_compaction() {
    let mut manifest = StorageManifest {
        live_segments: vec![
            ManifestSegment {
                id: 1,
                generation: 1,
                checkpoint_seq: 10,
                cell_count: 10,
            },
            ManifestSegment {
                id: 2,
                generation: 2,
                checkpoint_seq: 20,
                cell_count: 10,
            },
        ],
        ..StorageManifest::default()
    };
    manifest.set_segment_stats(ManifestSegmentStats {
        segment_id: 1,
        row_count: 10,
        ..ManifestSegmentStats::default()
    });
    manifest.set_segment_stats(ManifestSegmentStats {
        segment_id: 2,
        row_count: 10,
        ..ManifestSegmentStats::default()
    });

    manifest.compact_to_segment(ManifestSegment {
        id: 3,
        generation: 3,
        checkpoint_seq: 20,
        cell_count: 20,
    });

    assert!(manifest.stats_for_segment(1).is_none());
    assert!(manifest.stats_for_segment(2).is_none());
    assert!(manifest.segment_stats.is_empty());
}

#[test]
fn manifest_rejects_zero_vector_dimension() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("manifest.acm");
    let manifest = StorageManifest {
        vector_profile: Some(ManifestVectorProfile {
            dimension: 1,
            metric: 0,
        }),
        ..StorageManifest::default()
    };
    manifest.store(&path).unwrap();
    let mut bytes = std::fs::read(&path).unwrap();
    let body_len = bytes.len() - 4;
    let vector_magic = bytes
        .windows(4)
        .position(|window| window == b"VECM")
        .unwrap();
    bytes[vector_magic + 4..vector_magic + 8].copy_from_slice(&0u32.to_le_bytes());
    bytes.truncate(body_len);
    let checksum = crc32c::crc32c(&bytes);
    bytes.extend_from_slice(&checksum.to_le_bytes());
    std::fs::write(&path, bytes).unwrap();

    assert!(StorageManifest::load(&path).is_err());
}
