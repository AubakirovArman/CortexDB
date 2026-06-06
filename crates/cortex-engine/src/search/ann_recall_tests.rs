//! ANN recall golden fixture tests.

#[cfg(test)]
mod tests {
    use crate::search::{AnnSearchPolicy, SearchLimit, MIN_ANN_RECALL_Q16};
    use crate::{Database, DatabaseOptions, EngineFeatureFlags};
    use cortex_core::{CellId, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};

    fn vector_cell(_id: u64, vector: &[i16]) -> KnowledgeCell {
        let payload = format!(
            "scope=project:test\nstatus=ready\nvector={}\n\nbody",
            vector
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        KnowledgeCell::new(
            KnowledgeCellMetadata {
                scope: "project:test".to_owned(),
                status: "ready".to_owned(),
                cell_type: KnowledgeCellType::Fact,
                memory_type: None,
                ttl_seconds: None,
                created_unix_seconds: None,
                source_trust_q16: None,
                source: None,
            },
            payload,
        )
    }

    #[test]
    fn ann_recall_perfect_with_exact_scan() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = open_hnsw_db(dir.path());

        // Insert 5 cells with simple 3D vectors
        let vectors: Vec<Vec<i16>> = vec![
            vec![1, 0, 0],
            vec![0, 1, 0],
            vec![0, 0, 1],
            vec![1, 1, 0],
            vec![1, 1, 1],
        ];

        for (i, v) in vectors.iter().enumerate() {
            db.put_knowledge_cell(CellId((i + 1) as u64), vector_cell((i + 1) as u64, v))
                .unwrap();
        }

        db.checkpoint().unwrap();

        let report = db
            .evaluate_vector_ann(
                &[1, 1, 0],
                &cortex_aql::AgentView {
                    agent_id: cortex_aql::AgentId(1),
                    label: Some("test".to_owned()),
                    readable_brains: std::collections::BTreeSet::from([cortex_aql::BrainId(1)]),
                    readable_scopes: std::collections::BTreeSet::from([
                        crate::query::metadata::scope_id("project:test"),
                    ]),
                    writable_scopes: std::collections::BTreeSet::new(),
                    allowed_modes: std::collections::BTreeSet::from([
                        cortex_aql::RetrievalMode::Balanced,
                    ]),
                    allowed_memory_types: std::collections::BTreeSet::new(),
                    max_context_budget_tokens: 4000,
                    default_context_budget_tokens: 1000,
                    max_candidate_limit: 100,
                    default_candidate_limit: 20,
                    min_required_confidence_q16: cortex_aql::Q16_ZERO,
                    max_ttl_seconds: Some(3600),
                    allow_remember: false,
                    allow_verify_fact: false,
                    allow_audit_mode: false,
                    require_citations_by_default: false,
                    private_scope: None,
                },
                crate::search::SearchLimit(3),
            )
            .unwrap();

        // Exact scan should always be available after checkpoint
        assert!(
            report.is_some(),
            "ANN evaluation should be available after checkpoint"
        );
        let report = report.unwrap();
        // With 5 vectors and exact top-3, overlap should be perfect (3/3)
        assert_eq!(
            report.overlap_count, 3,
            "exact top-3 should overlap perfectly"
        );
        assert_eq!(
            report.recall_q16,
            u16::MAX,
            "perfect recall should be MAX_Q16"
        );
    }

    #[test]
    fn ann_metrics_after_checkpoint() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = open_hnsw_db(dir.path());

        let vectors: Vec<Vec<i16>> = vec![vec![1, 0], vec![0, 1], vec![1, 1]];
        for (i, v) in vectors.iter().enumerate() {
            db.put_knowledge_cell(CellId((i + 1) as u64), vector_cell((i + 1) as u64, v))
                .unwrap();
        }

        let metrics_before = db.ann_metrics();
        assert_eq!(metrics_before.persisted_segments, 0);
        assert!(!metrics_before.has_checkpoint);

        db.checkpoint().unwrap();

        let metrics_after = db.ann_metrics();
        assert!(metrics_after.has_checkpoint);
        assert_eq!(metrics_after.persisted_segments, 1);
    }

    #[test]
    fn ann_fixture_gate_meets_recall_slo_after_checkpoint() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = open_hnsw_db(dir.path());

        let vectors: Vec<Vec<i16>> =
            vec![vec![10, 0], vec![8, 2], vec![0, 10], vec![1, 9], vec![5, 5]];
        for (i, v) in vectors.iter().enumerate() {
            db.put_knowledge_cell(CellId((i + 1) as u64), vector_cell((i + 1) as u64, v))
                .unwrap();
        }
        db.checkpoint().unwrap();

        let report = db
            .evaluate_vector_ann_with_policy(
                &[10, 0],
                &test_view(),
                SearchLimit(2),
                AnnSearchPolicy {
                    min_recall_q16: Some(MIN_ANN_RECALL_Q16),
                    fallback: true,
                    fallback_scan_cap: None,
                    max_visited_candidates: None,
                    require_slo: true,
                },
            )
            .unwrap()
            .expect("checkpointed ANN evaluation should be available");

        assert!(
            report.recall_q16 >= MIN_ANN_RECALL_Q16,
            "ANN recall fixture gate failed: recall_q16={} min={}",
            report.recall_q16,
            MIN_ANN_RECALL_Q16
        );
        assert!(report.search.production_safe);
        assert!(report.search.slo_violations.is_empty());
    }

    #[test]
    fn ann_slo_gate_reports_budget_violation() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = open_hnsw_db(dir.path());

        for id in 1..=64 {
            db.put_knowledge_cell(CellId(id), vector_cell(id, &[id as i16, (64 - id) as i16]))
                .unwrap();
        }
        db.checkpoint().unwrap();

        let outcome = db
            .search_vector_with_report_with_policy(
                &[64, 0],
                &test_view(),
                SearchLimit(2),
                AnnSearchPolicy {
                    min_recall_q16: Some(MIN_ANN_RECALL_Q16),
                    fallback: true,
                    fallback_scan_cap: None,
                    max_visited_candidates: Some(1),
                    require_slo: true,
                },
            )
            .unwrap();

        let report = outcome.ann_report.expect("ANN report should be emitted");
        assert!(report.fallback_performed);
        assert!(!report.production_safe);
        assert!(report
            .slo_violations
            .iter()
            .any(|violation| violation.as_str() == "visit_budget_exceeded"));
    }

    fn test_view() -> cortex_aql::AgentView {
        cortex_aql::AgentView {
            agent_id: cortex_aql::AgentId(1),
            label: Some("test".to_owned()),
            readable_brains: std::collections::BTreeSet::from([cortex_aql::BrainId(1)]),
            readable_scopes: std::collections::BTreeSet::from([crate::query::metadata::scope_id(
                "project:test",
            )]),
            writable_scopes: std::collections::BTreeSet::new(),
            allowed_modes: std::collections::BTreeSet::from([cortex_aql::RetrievalMode::Balanced]),
            allowed_memory_types: std::collections::BTreeSet::new(),
            max_context_budget_tokens: 4000,
            default_context_budget_tokens: 1000,
            max_candidate_limit: 100,
            default_candidate_limit: 20,
            min_required_confidence_q16: cortex_aql::Q16_ZERO,
            max_ttl_seconds: Some(3600),
            allow_remember: false,
            allow_verify_fact: false,
            allow_audit_mode: false,
            require_citations_by_default: false,
            private_scope: None,
        }
    }

    fn open_hnsw_db(path: &std::path::Path) -> Database {
        Database::open_with_options(
            path,
            DatabaseOptions {
                feature_flags: EngineFeatureFlags::production_safe().with_experimental_hnsw(true),
                ..DatabaseOptions::default()
            },
        )
        .unwrap()
    }
}
