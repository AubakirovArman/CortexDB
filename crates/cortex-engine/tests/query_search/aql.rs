use super::common::prelude::*;
use super::common::view;
use cortex_aql::AqlCatalog;
use cortex_engine::ExecutionPath;

#[test]
fn retrieve_aql_uses_engine_index_without_mock_provider() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nalpha budget".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=draft\nbeta budget".to_vec(),
    )
    .unwrap();

    let cells = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES;"#,
            &view(scope_id("project:investments")),
        )
        .unwrap();
    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0].cell_id, CellId(1));
}

#[test]
fn retrieve_aql_delta_index_tracks_write_patch_tombstone_checkpoint_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let query = r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES;"#;
    let mut db = Database::open(dir.path()).unwrap();

    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\n\nalpha budget".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=draft\n\nbeta budget".to_vec(),
    )
    .unwrap();
    assert_eq!(retrieve_ids(&db, query), vec![CellId(1)]);

    db.patch_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\n\nbeta budget".to_vec(),
    )
    .unwrap();
    assert_eq!(retrieve_ids(&db, query), vec![CellId(1), CellId(2)]);

    db.tombstone_cell(CellId(1)).unwrap();
    assert_eq!(retrieve_ids(&db, query), vec![CellId(2)]);

    db.checkpoint().unwrap();
    assert_eq!(retrieve_ids(&db, query), vec![CellId(2)]);
    drop(db);

    let db = Database::open(dir.path()).unwrap();
    assert_eq!(retrieve_ids(&db, query), vec![CellId(2)]);
}

#[test]
fn aql_uses_manifest_stats_for_bitmap_estimates_after_checkpoint() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:wide\nstatus=draft\ntype=fact\n\nalpha budget".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:wide\nstatus=draft\ntype=fact\n\nbeta budget".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(3),
        b"scope=project:wide\nstatus=ready\ntype=document_block\n\nbudget gamma".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();

    let statistics = db.statistics();
    assert_eq!(
        statistics.estimate_scope_cardinality("project:wide"),
        Some(3)
    );
    assert_eq!(statistics.estimate_status_cardinality("ready"), Some(1));
    assert_eq!(
        statistics.estimate_cell_type_cardinality("document_block"),
        Some(1)
    );
    assert_eq!(
        statistics.estimate_term_document_frequency("budget"),
        Some(3)
    );

    let report = db
        .explain_retrieve_aql(
            r#"EXPLAIN RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:wide AND status = "ready" LIMIT 10 CANDIDATES;"#,
            &view(scope_id("project:wide")),
        )
        .unwrap();
    let index = db.aql_index().unwrap();
    let brain = BrainId(1);
    let status_handle = index
        .status_bitmap(brain, index.resolve_status(brain, "ready").unwrap())
        .unwrap();
    let scope_handle = index.scope_bitmap(brain, scope_id("project:wide")).unwrap();

    assert_eq!(report.candidate_counts.estimated_after_bitmap, Some(1));
    assert_eq!(report.cost_model.estimated_after_bitmap, Some(1));
    assert_eq!(report.cost_model.selected_path, ExecutionPath::BitmapFirst);
    assert_eq!(report.candidate_counts.after_bitmap, 1);
    let status_op = format!("Push({status_handle:?})");
    let scope_op = format!("Push({scope_handle:?})");
    let status_position = report
        .bitmap_ops
        .iter()
        .position(|op| op == &status_op)
        .unwrap();
    let scope_position = report
        .bitmap_ops
        .iter()
        .position(|op| op == &scope_op)
        .unwrap();
    assert!(status_position < scope_position);
}

#[test]
fn retrieve_aql_lazy_payload_residency_reads_checkpoint_payload_on_demand() {
    let dir = tempfile::tempdir().unwrap();
    let payload =
        b"scope=project:investments\nstatus=ready\ntitle=lazy budget\n\nlazy budget payload body"
            .to_vec();
    let query = r#"RETRIEVE CONTEXT FOR TASK "lazy budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES;"#;

    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), payload.clone()).unwrap();
        db.checkpoint().unwrap();
    }

    let db = Database::open_with_options(
        dir.path(),
        DatabaseOptions {
            payload_residency: PayloadResidency::Lazy,
            ..DatabaseOptions::default()
        },
    )
    .unwrap();

    assert_eq!(db.payload_residency(), PayloadResidency::Lazy);
    assert_eq!(db.storage_stats().unwrap().memtable_payload_bytes, 0);
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), payload);

    let cells = db
        .retrieve_aql(query, &view(scope_id("project:investments")))
        .unwrap();
    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0].cell_id, CellId(1));
    assert_eq!(cells[0].payload, payload);
}

#[test]
fn retrieve_aql_with_allowed_cells_restricts_candidate_pool() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nalpha budget".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\nbeta budget".to_vec(),
    )
    .unwrap();

    let cells = db
        .retrieve_aql_with_allowed_cells(
            r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES;"#,
            &view(scope_id("project:investments")),
            &BTreeSet::from([CellId(2)]),
        )
        .unwrap();

    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0].cell_id, CellId(2));
}

fn retrieve_ids(db: &Database, query: &str) -> Vec<CellId> {
    let mut ids = db
        .retrieve_aql(query, &view(scope_id("project:investments")))
        .unwrap()
        .into_iter()
        .map(|cell| cell.cell_id)
        .collect::<Vec<_>>();
    ids.sort();
    ids
}

#[test]
fn retrieve_aql_uses_descriptor_scope_for_persisted_bitmap_acl() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        KnowledgeCell::new(
            KnowledgeCellMetadata {
                scope: "tenant:private".to_owned(),
                status: "ready".to_owned(),
                cell_type: KnowledgeCellType::Raw,
                ..Default::default()
            },
            b"scope=project:investments\nstatus=ready\n\nhidden spoof budget".to_vec(),
        ),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\n\nvisible budget".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();

    let cells = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES;"#,
            &view(scope_id("project:investments")),
        )
        .unwrap();

    assert_eq!(
        cells.iter().map(|cell| cell.cell_id).collect::<Vec<_>>(),
        vec![CellId(2)]
    );
}

#[test]
fn explain_retrieve_aql_reports_plan_filters_counts_and_mode() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nsource=doc-a\n\nalpha budget".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=draft\nsource=doc-b\n\nbeta budget".to_vec(),
    )
    .unwrap();

    let report = db
        .explain_retrieve_aql(
            r#"EXPLAIN RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
USING MODE balanced WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES;"#,
            &view(scope_id("project:investments")),
        )
        .unwrap();

    assert_eq!(report.task, "budget");
    assert_eq!(report.selected_mode, RetrievalMode::Balanced);
    assert!(!report.logical_plan.policy_complete);
    assert!(report.policy_rewritten_plan.policy_complete);
    assert!(report
        .logical_plan
        .nodes
        .iter()
        .any(|node| node.kind == "scan" && node.permission_predicate.is_none()));
    assert!(report
        .policy_rewritten_plan
        .nodes
        .iter()
        .any(|node| node.kind == "scan"
            && node.permission_predicate.as_deref() == Some("agent_allowed")));
    assert_eq!(report.candidate_counts.universe, 2);
    assert_eq!(report.candidate_counts.agent_allowed, 2);
    assert_eq!(report.candidate_counts.live, 2);
    assert_eq!(report.candidate_counts.after_bitmap, 1);
    assert_eq!(report.candidate_counts.after_quality, 1);
    assert_eq!(report.candidate_counts.returned_limit, 1);
    assert_eq!(report.cost_model.selected_path, ExecutionPath::BitmapFirst);
    assert!(report.cost_model.reason.contains("bitmap"));
    assert_eq!(report.cost_model.estimated_after_bitmap, None);
    assert_eq!(report.cost_model.recommended_candidate_limit, 2);
    assert!(report
        .bitmap_plan
        .contains("BitmapProgram(max_stack_depth="));
    assert!(report.bitmap_ops.iter().any(|op| op == "PushAgentAllowed"));
    assert!(report
        .filters
        .iter()
        .any(|filter| filter.expression.contains("status = \"ready\"")));
}

#[test]
fn explain_analyze_retrieve_aql_reports_operator_counts() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nsource=doc-a\n\nalpha budget".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\nsource=doc-b\n\nbeta budget".to_vec(),
    )
    .unwrap();

    let report = db
        .explain_analyze_retrieve_aql(
            r#"EXPLAIN ANALYZE RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
USING MODE balanced WHERE space = project:investments LIMIT 1 CANDIDATES;"#,
            &view(scope_id("project:investments")),
        )
        .unwrap();
    let trace = report.execution_trace.expect("analyze trace");
    let names = trace
        .operators
        .iter()
        .map(|operator| operator.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(report.candidate_counts.returned_limit, 1);
    assert_eq!(
        names,
        vec![
            "BitmapIndexScan",
            "PermissionFilter",
            "QualityFilter",
            "RankOp",
            "DedupOp",
            "ParentExpandOp",
            "LimitOp",
        ]
    );
    assert_eq!(
        trace
            .operators
            .iter()
            .find(|operator| operator.name == "LimitOp")
            .map(|operator| operator.output_count),
        Some(1)
    );
    assert!(trace.total_elapsed_nanos > 0);
}

#[test]
fn retrieve_aql_preserves_large_cell_ids_after_checkpoint_and_compact() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(
            CellId(u64::MAX),
            b"scope=project:investments\nstatus=ready\nlarge alpha".to_vec(),
        )
        .unwrap();
        db.put_cell(
            CellId(u64::MAX - 1),
            b"scope=project:investments\nstatus=draft\nlarge beta".to_vec(),
        )
        .unwrap();
        db.checkpoint().unwrap();
        db.compact().unwrap();
    }

    let db = Database::open(dir.path()).unwrap();
    let cells = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "large" IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES;"#,
            &view(scope_id("project:investments")),
        )
        .unwrap();
    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0].cell_id, CellId(u64::MAX));
    assert_eq!(
        cells[0].payload,
        b"scope=project:investments\nstatus=ready\nlarge alpha"
    );
}

#[test]
fn persisted_index_overlay_removes_changed_checkpoint_candidates() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nalpha budget".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();
    db.patch_cell(
        CellId(1),
        b"scope=project:investments\nstatus=draft\nalpha budget".to_vec(),
    )
    .unwrap();

    let readable = view(scope_id("project:investments"));
    let ready = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES;"#,
            &readable,
        )
        .unwrap();
    let draft = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "draft" LIMIT 10 CANDIDATES;"#,
            &readable,
        )
        .unwrap();

    assert!(ready.is_empty());
    assert_eq!(draft[0].cell_id, CellId(1));
}

#[test]
fn retrieve_aql_reports_missing_persisted_index() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nalpha budget".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();
    std::fs::remove_file(dir.path().join("segments").join("segment-1.acb")).unwrap();

    let result = db.retrieve_aql(
        r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES;"#,
        &view(scope_id("project:investments")),
    );
    assert!(result.is_err());
}
