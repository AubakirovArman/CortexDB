use super::*;
use cortex_core::memtable::CellVersion;
use cortex_core::{CellId, CommitSeq};

use crate::database::Database;
use crate::error::EngineResult;
use crate::query::CellMetadata;
use crate::search::vector::vector_from_payload;

struct TestEmbeddingProvider {
    vector: Vec<i16>,
    calls: Vec<String>,
    batch_sizes: Vec<usize>,
}

impl EmbeddingBackfillProvider for TestEmbeddingProvider {
    fn embed_text(&mut self, text: &str) -> EngineResult<Vec<i16>> {
        self.calls.push(text.to_owned());
        Ok(self.vector.clone())
    }

    fn embed_text_batch(&mut self, texts: &[String]) -> EngineResult<Vec<Vec<i16>>> {
        self.batch_sizes.push(texts.len());
        texts.iter().map(|text| self.embed_text(text)).collect()
    }
}

fn default_report(jsonl: &str) -> EmbeddingCoverageReport {
    embedding_coverage_report_from_jsonl(
        ["doc-a".to_owned(), "doc-b".to_owned()],
        jsonl,
        EmbeddingCoverageConfig::default(),
    )
}

#[test]
fn complete_embedding_jsonl_is_production_ready() {
    let report = default_report(
        r#"{"doc_id":"doc-a","vector":[1.0,2.0]}
{"doc_id":"doc-b","vector":[3.0,4.0]}
"#,
    );

    assert_eq!(report.coverage_basis_points, 10_000);
    assert_eq!(report.dimension, Some(2));
    assert!(report.production_ready);
}

#[test]
fn missing_duplicate_unexpected_and_invalid_rows_are_reported() {
    let report = default_report(
        r#"{"doc_id":"doc-a","vector":[1.0,2.0]}
{"doc_id":"doc-a","vector":[1.0,2.0]}
{"doc_id":"doc-z","vector":[1.0,2.0]}
{"doc_id":"doc-b","vector":[]}
not-json
"#,
    );

    assert_eq!(report.embedded_items, 1);
    assert_eq!(report.missing_items, 1);
    assert_eq!(report.duplicate_items, 1);
    assert_eq!(report.unexpected_items, 1);
    assert_eq!(report.empty_vector_rows, 1);
    assert_eq!(report.invalid_rows, 1);
    assert!(!report.production_ready);
    assert_eq!(report.missing_ids_sample, vec!["doc-b"]);
    assert_eq!(report.duplicate_ids_sample, vec!["doc-a"]);
    assert_eq!(report.unexpected_ids_sample, vec!["doc-z"]);
}

#[test]
fn dimension_mismatch_blocks_production_ready() {
    let report = embedding_coverage_report_from_jsonl(
        ["doc-a".to_owned()],
        r#"{"doc_id":"doc-a","vector":[1.0,2.0]}"#,
        EmbeddingCoverageConfig {
            expected_dimension: Some(3),
            min_coverage_basis_points: 10_000,
            expected_model: None,
        },
    );

    assert_eq!(report.embedded_items, 0);
    assert_eq!(report.dimension_mismatch_rows, 1);
    assert!(!report.production_ready);
}

#[test]
fn production_ready_uses_configured_coverage_threshold() {
    let expected = (0..200).map(|index| format!("doc-{index:03}"));
    let jsonl = (0..199)
        .map(|index| format!(r#"{{"doc_id":"doc-{index:03}","vector":[1.0]}}"#))
        .collect::<Vec<_>>()
        .join("\n");

    let report =
        embedding_coverage_report_from_jsonl(expected, &jsonl, EmbeddingCoverageConfig::default());

    assert_eq!(report.coverage_basis_points, 9_950);
    assert_eq!(report.missing_items, 1);
    assert!(report.production_ready);
}

#[test]
fn retry_ids_include_missing_and_invalid_vectors() {
    let retry_ids = embedding_retry_ids_from_jsonl(
        ["doc-a".to_owned(), "doc-b".to_owned(), "doc-c".to_owned()],
        r#"{"doc_id":"doc-a","vector":[1.0,2.0]}
{"doc_id":"doc-b","vector":[]}
"#,
        EmbeddingCoverageConfig::default(),
    );

    assert_eq!(retry_ids, vec!["doc-b", "doc-c"]);
}

#[test]
fn stale_model_or_text_hash_are_retried() {
    let expected = [
        EmbeddingExpectedItem {
            doc_id: "doc-a".to_owned(),
            text_hash: Some("hash-a-v2".to_owned()),
        },
        EmbeddingExpectedItem {
            doc_id: "doc-b".to_owned(),
            text_hash: Some("hash-b".to_owned()),
        },
        EmbeddingExpectedItem {
            doc_id: "doc-c".to_owned(),
            text_hash: Some("hash-c".to_owned()),
        },
    ];
    let jsonl = r#"{"doc_id":"doc-a","vector":[1.0],"model":"bge-m3","text_hash":"hash-a-v1"}
{"doc_id":"doc-b","vector":[1.0],"model":"other-model","text_hash":"hash-b"}
{"doc_id":"doc-c","vector":[1.0],"model":"bge-m3","text_hash":"hash-c"}
"#;

    let report = embedding_coverage_report_from_expected_items(
        expected.clone(),
        jsonl,
        EmbeddingCoverageConfig {
            expected_dimension: Some(1),
            min_coverage_basis_points: 10_000,
            expected_model: Some("bge-m3".to_owned()),
        },
    );

    assert_eq!(report.embedded_items, 1);
    assert_eq!(report.stale_items, 2);
    assert_eq!(report.stale_ids_sample, vec!["doc-a", "doc-b"]);
    assert!(!report.production_ready);

    let retry_ids = embedding_retry_ids_from_expected_items(
        expected,
        jsonl,
        EmbeddingCoverageConfig {
            expected_dimension: Some(1),
            min_coverage_basis_points: 10_000,
            expected_model: Some("bge-m3".to_owned()),
        },
    );

    assert_eq!(retry_ids, vec!["doc-a", "doc-b"]);
}

#[test]
fn live_versions_without_vectors_are_embedding_debt() {
    let versions = vec![
        CellVersion::new(CellId(10), CommitSeq(1), b"scope=docs\n\nAlpha".to_vec(), 0),
        CellVersion::new(CellId(11), CommitSeq(2), b"scope=docs\n\nBeta".to_vec(), 0),
    ];

    let report = embedding_debt_report_from_versions(
        &versions,
        EmbeddingCoverageConfig {
            expected_dimension: Some(3),
            min_coverage_basis_points: 10_000,
            expected_model: Some("bge-m3".to_owned()),
        },
    );

    assert_eq!(report.total_items, 2);
    assert_eq!(report.ready_items, 0);
    assert_eq!(report.missing_vector_items, 2);
    assert_eq!(report.debt_sample[0].cell_id, 10);
}

#[test]
fn matching_vector_model_and_text_hash_clear_embedding_debt() {
    let base = b"scope=docs\n\nAlpha";
    let hash = embedding_text_hash(base);
    let payload = format!(
        "scope=docs\nembedding_model=bge-m3\nembedding_text_hash={hash}\nvector=1,2,3\n\nAlpha"
    )
    .into_bytes();
    let versions = vec![CellVersion::new(CellId(10), CommitSeq(1), payload, 0)];

    let report = embedding_debt_report_from_versions(
        &versions,
        EmbeddingCoverageConfig {
            expected_dimension: Some(3),
            min_coverage_basis_points: 10_000,
            expected_model: Some("bge-m3".to_owned()),
        },
    );

    assert_eq!(report.total_items, 1);
    assert_eq!(report.ready_items, 1);
    assert_eq!(report.debt_items, 0);
}

#[test]
fn database_embedding_debt_report_tracks_new_put_cells() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(CellId(10), b"scope=docs\n\nAlpha".to_vec())
        .unwrap();

    let report = db.embedding_debt_report(EmbeddingCoverageConfig {
        expected_dimension: Some(3),
        min_coverage_basis_points: 10_000,
        expected_model: Some("bge-m3".to_owned()),
    });

    assert_eq!(report.total_items, 1);
    assert_eq!(report.missing_vector_items, 1);
    assert_eq!(report.debt_items, 1);
    assert_eq!(db.embedding_expected_manifest()[0].doc_id, "10");
}

#[test]
fn database_backfill_embedding_debt_patches_cells_and_clears_debt() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(CellId(10), b"scope=docs\nstatus=ready\n\nAlpha".to_vec())
        .unwrap();
    let mut provider = TestEmbeddingProvider {
        vector: vec![1, 2, 3],
        calls: Vec::new(),
        batch_sizes: Vec::new(),
    };
    let config = EmbeddingCoverageConfig {
        expected_dimension: Some(3),
        min_coverage_basis_points: 10_000,
        expected_model: Some("bge-m3".to_owned()),
    };

    let report = db
        .backfill_embedding_debt(
            &mut provider,
            EmbeddingBackfillOptions {
                config: config.clone(),
                max_items: None,
            },
        )
        .unwrap();

    assert_eq!(report.debt_items, 1);
    assert_eq!(report.embedded_items, 1);
    assert_eq!(report.failed_items, 0);
    assert_eq!(report.batch_size, 1);
    assert_eq!(report.embedding_batches, 1);
    assert_eq!(report.write_batches, 1);
    assert_eq!(report.max_batch_items, 1);
    assert_eq!(report.final_debt_items, 0);
    assert_eq!(provider.calls, vec!["scope=docs\nstatus=ready\n\nAlpha"]);

    let payload = db.get_latest_cell(CellId(10)).unwrap();
    assert_eq!(vector_from_payload(&payload), Some(vec![1, 2, 3]));
    let metadata = CellMetadata::from_payload(&payload);
    assert_eq!(metadata.scope, "docs");
    assert_eq!(metadata.status, "ready");
    assert_eq!(metadata.body_text, "Alpha");
    assert_eq!(db.embedding_debt_report(config).debt_items, 0);
}

#[test]
fn database_backfill_embedding_debt_batches_patch_writes() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    for index in 1..=5 {
        db.put_cell(
            CellId(index),
            format!("scope=docs\nstatus=ready\n\nDoc {index}").into_bytes(),
        )
        .unwrap();
    }
    let mut provider = TestEmbeddingProvider {
        vector: vec![1, 2, 3],
        calls: Vec::new(),
        batch_sizes: Vec::new(),
    };
    let config = EmbeddingCoverageConfig {
        expected_dimension: Some(3),
        min_coverage_basis_points: 10_000,
        expected_model: Some("bge-m3".to_owned()),
    };

    let report = db
        .backfill_embedding_debt_batched(
            &mut provider,
            EmbeddingBackfillOptions {
                config: config.clone(),
                max_items: None,
            },
            2,
        )
        .unwrap();

    assert_eq!(report.debt_items, 5);
    assert_eq!(report.embedded_items, 5);
    assert_eq!(report.final_debt_items, 0);
    assert_eq!(report.batch_size, 2);
    assert_eq!(report.embedding_batches, 3);
    assert_eq!(report.write_batches, 3);
    assert_eq!(report.max_batch_items, 2);
    assert_eq!(provider.batch_sizes, vec![2, 2, 1]);
    assert_eq!(db.current_seq(), CommitSeq(10));
    assert_eq!(db.storage_stats().unwrap().wal_writer.batches_committed, 8);
    assert_eq!(db.embedding_debt_report(config).debt_items, 0);
}

#[test]
fn database_backfill_embedding_debt_resumes_after_partial_run() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        for index in 1..=4 {
            db.put_cell(
                CellId(index),
                format!("scope=docs\nstatus=ready\n\nDoc {index}").into_bytes(),
            )
            .unwrap();
        }
        let mut provider = TestEmbeddingProvider {
            vector: vec![1, 2, 3],
            calls: Vec::new(),
            batch_sizes: Vec::new(),
        };
        let first = db
            .backfill_embedding_debt_batched(
                &mut provider,
                EmbeddingBackfillOptions {
                    config: test_embedding_config(),
                    max_items: Some(2),
                },
                2,
            )
            .unwrap();
        assert_eq!(first.embedded_items, 2);
        assert_eq!(first.skipped_items, 2);
        assert_eq!(first.final_debt_items, 2);
        db.close().unwrap();
    }

    let mut db = Database::open(dir.path()).unwrap();
    let mut provider = TestEmbeddingProvider {
        vector: vec![1, 2, 3],
        calls: Vec::new(),
        batch_sizes: Vec::new(),
    };
    let second = db
        .backfill_embedding_debt_batched(
            &mut provider,
            EmbeddingBackfillOptions {
                config: test_embedding_config(),
                max_items: None,
            },
            2,
        )
        .unwrap();

    assert_eq!(second.debt_items, 2);
    assert_eq!(second.embedded_items, 2);
    assert_eq!(second.final_debt_items, 0);
    assert_eq!(provider.batch_sizes, vec![2]);
}

fn test_embedding_config() -> EmbeddingCoverageConfig {
    EmbeddingCoverageConfig {
        expected_dimension: Some(3),
        min_coverage_basis_points: 10_000,
        expected_model: Some("bge-m3".to_owned()),
    }
}
