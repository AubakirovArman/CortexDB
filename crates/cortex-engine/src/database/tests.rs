use std::collections::BTreeSet;
use std::time::Instant;

use cortex_aql::{
    AgentId, AgentView, BoundPlan, BrainId, MemoryType, QualityThresholds, RetrievalMode, Q16_ZERO,
};
use cortex_core::memtable::CellVersion;
use cortex_core::{CellId, CommitSeq};

use super::*;
use crate::query::{scope_id, EngineAqlProvider};
use crate::retrieval_quality::cell_version_meets_quality_thresholds;

fn test_view(modes: impl IntoIterator<Item = RetrievalMode>) -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("database-test".to_owned()),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id("default")]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: modes.into_iter().collect(),
        allowed_memory_types: BTreeSet::new(),
        max_context_budget_tokens: 4000,
        default_context_budget_tokens: 1000,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: None,
        allow_remember: false,
        allow_verify_fact: false,
        allow_audit_mode: true,
        require_citations_by_default: false,
        private_scope: None,
    }
}

#[test]
fn retrieve_aql_orders_by_lexical_relevance_before_candidate_id() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"title=unrelated\n\ncommon body without the important term".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"title=needle migration policy\n\ncommon body".to_vec(),
    )
    .unwrap();

    let view = test_view([RetrievalMode::Balanced]);
    let results = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "needle" IN BRAIN default LIMIT 2 CANDIDATES;"#,
            &view,
        )
        .unwrap();

    assert_eq!(results[0].cell_id, CellId(2));
}

#[test]
fn quality_threshold_fast_path_uses_materialized_descriptor() {
    let mut version = CellVersion::new(
        CellId(1),
        CommitSeq(1),
        b"scope=default\nstatus=ready\nsource_trust_q16=60000\n\nbody".to_vec(),
        0,
    );
    version.payload = b"scope=default\nstatus=ready\nsource_trust_q16=1000\n\nbody".to_vec();
    let thresholds = QualityThresholds {
        min_source_trust_q16: 50_000,
        ..QualityThresholds::default()
    };
    assert!(cell_version_meets_quality_thresholds(&version, &thresholds));
}

#[test]
fn retrieve_aql_uses_path_view_for_lexical_relevance() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(CellId(1), b"title=unrelated\n\nrunbook".to_vec())
        .unwrap();
    db.put_cell(
        CellId(2),
        b"path=confluence/payments/runbook\n\ncommon body".to_vec(),
    )
    .unwrap();

    let view = test_view([RetrievalMode::Balanced]);
    let results = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "runbook" IN BRAIN default LIMIT 2 CANDIDATES;"#,
            &view,
        )
        .unwrap();

    assert_eq!(results[0].cell_id, CellId(2));
}

#[test]
fn retrieve_aql_expands_child_hit_with_parent_context() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
            CellId(1),
            b"document_id=doc-alpha\nchunk_id=parent-alpha\nchunk_role=parent\ntitle=Alpha parent\n\nParent context includes owner, deadline, and rollout notes."
                .to_vec(),
        )
        .unwrap();
    db.put_cell(
            CellId(2),
            b"document_id=doc-alpha\nchunk_id=child-alpha-1\nparent_id=parent-alpha\nchunk_role=child\nsection=Risk details\n\nspecific-child-anchor appears here."
                .to_vec(),
        )
        .unwrap();

    let view = test_view([RetrievalMode::Balanced]);
    let results = db
            .retrieve_aql(
                r#"RETRIEVE CONTEXT FOR TASK "specific-child-anchor" IN BRAIN default LIMIT 2 CANDIDATES;"#,
                &view,
            )
            .unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].cell_id, CellId(2));
    assert_eq!(results[1].cell_id, CellId(1));
}

#[test]
fn operator_executor_matches_direct_retrieve_pipeline_and_reports_trace() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
            CellId(1),
            b"document_id=doc-alpha\nchunk_id=parent-alpha\nchunk_role=parent\ncontent_hash=parent\n\nParent context for alpha."
                .to_vec(),
        )
        .unwrap();
    db.put_cell(
            CellId(2),
            b"document_id=doc-alpha\nchunk_id=child-alpha\nparent_id=parent-alpha\nchunk_role=child\ncontent_hash=child\n\nneedle alpha detail"
                .to_vec(),
        )
        .unwrap();
    db.put_cell(
        CellId(3),
        b"content_hash=duplicate\n\nneedle alpha duplicate".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(4),
        b"content_hash=duplicate\ntitle=needle alpha\n\nneedle alpha duplicate".to_vec(),
    )
    .unwrap();

    let view = test_view([RetrievalMode::Balanced]);
    let (cached, index) = db
        .bind_aql_cached(
            r#"RETRIEVE CONTEXT FOR TASK "needle alpha" IN BRAIN default LIMIT 10 CANDIDATES;"#,
            &view,
        )
        .unwrap();
    let BoundPlan::Retrieve(plan) = cached.bound_plan else {
        panic!("expected retrieve plan");
    };
    let provider = EngineAqlProvider::new(index, &view);

    let direct = db.retrieve_cells_direct(&plan, &provider).unwrap();
    let report = db
        .retrieve_cells_with_execution_trace(&plan, &provider)
        .unwrap();

    assert_eq!(report.cells, direct);
    let names = report
        .operators
        .iter()
        .map(|operator| operator.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            "BitmapIndexScan",
            "PermissionFilter",
            "MemoryLifecycleFilter",
            "QualityFilter",
            "RankOp",
            "DedupOp",
            "ParentExpandOp",
            "LimitOp",
        ]
    );
    assert!(report.total_elapsed_nanos > 0);
    assert_eq!(
        report
            .operators
            .iter()
            .find(|operator| operator.name == "LimitOp")
            .map(|operator| operator.output_count),
        Some(report.cells.len())
    );
}

#[test]
fn retrieve_execution_report_captures_permission_denials_without_forbidden_payload() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=default\ntitle=alpha shared\n\nallowed alpha body".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=default\ntitle=alpha shared\n\nsecret-denied-payload-marker".to_vec(),
    )
    .unwrap();

    let view = test_view([RetrievalMode::Balanced]);
    let (cached, index) = db
        .bind_aql_cached(
            r#"RETRIEVE CONTEXT FOR TASK "alpha shared" IN BRAIN default LIMIT 10 CANDIDATES;"#,
            &view,
        )
        .unwrap();
    let BoundPlan::Retrieve(plan) = cached.bound_plan else {
        panic!("expected retrieve plan");
    };
    let allowed_candidates = BTreeSet::from([index
        .cell_to_candidate
        .get(&CellId(1))
        .copied()
        .expect("allowed cell candidate")]);
    let provider =
        EngineAqlProvider::new_with_allowed_candidates(index, &view, &allowed_candidates);

    let report = db
        .retrieve_cells_with_execution_trace(&plan, &provider)
        .unwrap();

    assert_eq!(report.cells.len(), 1);
    assert_eq!(report.cells[0].cell_id, CellId(1));
    assert_eq!(report.captured_access_denials.total_denied, 1);
    assert!(!report.captured_access_denials.truncated);
    assert_eq!(report.captured_access_denials.denials.len(), 1);
    let denial = &report.captured_access_denials.denials[0];
    assert_eq!(denial.policy_version, "agent_view_readable_scope.v1");
    assert_eq!(denial.agent_id, Some(view.agent_id.0));
    assert_eq!(denial.cell_id_hash.len(), 64);
    assert_eq!(denial.evidence_digest.len(), 64);
    assert!(denial
        .reason
        .contains("rejected by AQL agent access filtering"));
    let denial_debug = format!("{:?}", report.captured_access_denials);
    assert!(!denial_debug.contains("secret-denied-payload-marker"));
    assert!(!denial_debug.contains("scope=default"));
    assert!(!denial_debug.contains("CellId(2)"));
}

#[test]
fn retrieve_aql_suppresses_duplicate_content_hashes() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"source_hash=source-a\ncontent_hash=same-content\n\nalpha budget duplicate".to_vec(),
    )
    .unwrap();
    db.put_cell(
            CellId(2),
            b"source_hash=source-b\ncontent_hash=same-content\ntitle=alpha budget\n\nalpha budget duplicate".to_vec(),
        )
        .unwrap();

    let view = test_view([RetrievalMode::Balanced]);
    let results = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "alpha budget" IN BRAIN default LIMIT 10 CANDIDATES;"#,
            &view,
        )
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].cell_id, CellId(2));
}

#[test]
fn audit_mode_uses_trust_weight_in_retrieval_ordering() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"source_trust_q16=1000\n\nbudget policy".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"source_trust_q16=65000\n\nbudget policy".to_vec(),
    )
    .unwrap();

    let view = test_view([RetrievalMode::Audit]);
    let results = db
            .retrieve_aql(
                r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN default USING MODE audit LIMIT 2 CANDIDATES;"#,
                &view,
            )
            .unwrap();

    assert_eq!(results[0].cell_id, CellId(2));
}

#[test]
fn semantic_mode_uses_query_vector_for_retrieval_ordering() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"title=vector alpha\nvector=100,0\n\nshared context".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"title=vector beta\nvector=0,100\n\nshared context".to_vec(),
    )
    .unwrap();

    let view = test_view([RetrievalMode::Semantic]);
    let results = db
            .retrieve_aql(
                r#"RETRIEVE CONTEXT FOR TASK "query_vector=0,100" IN BRAIN default USING MODE semantic LIMIT 2 CANDIDATES;"#,
                &view,
            )
            .unwrap();

    assert_eq!(results[0].cell_id, CellId(2));
}

#[test]
fn semantic_mode_uses_named_view_vectors_for_retrieval_ordering() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"title=body match\nvector=0,100\n\nshared context".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"title=view match\ntitle_vector=100,0\nvector=0,1\n\nshared context".to_vec(),
    )
    .unwrap();

    let view = test_view([RetrievalMode::Semantic]);
    let results = db
            .retrieve_aql(
                r#"RETRIEVE CONTEXT FOR TASK "query_vector=100,0" IN BRAIN default USING MODE semantic LIMIT 2 CANDIDATES;"#,
                &view,
            )
            .unwrap();

    assert_eq!(results[0].cell_id, CellId(2));
}

fn bench_payload(id: u64) -> Vec<u8> {
    format!("scope=bench\nstatus=ready\nsource=bench-{id}\ncell {id} budget ready").into_bytes()
}

fn bench_query() -> &'static str {
    r#"RETRIEVE CONTEXT FOR TASK "bench" IN BRAIN default WHERE space = bench AND status = "ready" LIMIT 100 CANDIDATES;"#
}

fn bench_view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("bench".to_owned()),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id("bench")]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 100_000,
        default_context_budget_tokens: 50_000,
        max_candidate_limit: 1000,
        default_candidate_limit: 100,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3_600),
        allow_remember: false,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: true,
        private_scope: None,
    }
}

#[test]
#[ignore = "run explicitly: cargo test -p cortex-engine --lib --release operator_executor_overhead -- --ignored --nocapture"]
fn operator_executor_overhead_within_ten_percent() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    for id in 1..=1000 {
        db.put_cell(CellId(id), bench_payload(id)).unwrap();
    }
    db.checkpoint().unwrap();

    let (cached, index) = db.bind_aql_cached(bench_query(), &bench_view()).unwrap();
    let BoundPlan::Retrieve(plan) = cached.bound_plan else {
        panic!("expected retrieve plan");
    };
    let provider = EngineAqlProvider::new(index, &bench_view());

    let iterations = 100;
    let mut direct_times = Vec::with_capacity(iterations);
    let mut exec_times = Vec::with_capacity(iterations);

    for _ in 0..iterations {
        let start = Instant::now();
        let _ = db.retrieve_cells_direct(&plan, &provider).unwrap();
        direct_times.push(start.elapsed().as_nanos());
    }
    for _ in 0..iterations {
        let start = Instant::now();
        let _ = db
            .retrieve_cells_with_execution_trace(&plan, &provider)
            .unwrap();
        exec_times.push(start.elapsed().as_nanos());
    }

    direct_times.sort();
    exec_times.sort();
    let direct_median = direct_times[iterations / 2];
    let exec_median = exec_times[iterations / 2];
    let ratio = exec_median as f64 / direct_median as f64;
    eprintln!(
        "direct median: {} ns, exec median: {} ns, ratio: {:.3}",
        direct_median, exec_median, ratio
    );
    assert!(
        ratio <= 1.10,
        "executor overhead exceeds 10%: ratio = {ratio}"
    );
}

#[test]
fn pin_read_txn_registers_and_unregisters() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    let pin = db.pin_read_txn();
    let seq = pin.read_txn().read_seq;
    assert_eq!(db.active_read_pins.lock().unwrap().get(&seq), Some(&1));
    assert_eq!(db.gc_horizon(), seq);
    drop(pin);
    assert!(db.active_read_pins.lock().unwrap().is_empty());
    assert_eq!(db.gc_horizon(), db.current_seq());
}

#[test]
fn gc_horizon_is_oldest_active_pin() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(CellId(1), b"v1".to_vec()).unwrap();
    let pin1 = db.pin_read_txn();
    let seq1 = pin1.read_txn().read_seq;
    db.put_cell(CellId(2), b"v2".to_vec()).unwrap();
    let pin2 = db.pin_read_txn();
    let seq2 = pin2.read_txn().read_seq;
    db.put_cell(CellId(3), b"v3".to_vec()).unwrap();
    assert_eq!(db.gc_horizon(), seq1);
    drop(pin1);
    assert_eq!(db.gc_horizon(), seq2);
    drop(pin2);
    assert_eq!(db.gc_horizon(), db.current_seq());
}

#[test]
fn pinned_snapshot_preserves_old_version_across_compact() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(CellId(1), b"v1".to_vec()).unwrap();
    let pin = db.pin_read_txn();
    let txn = pin.read_txn();
    db.patch_cell(CellId(1), b"v2".to_vec()).unwrap();
    db.compact().unwrap();
    assert_eq!(db.get_cell(txn, CellId(1)).unwrap(), b"v1");
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"v2");
    drop(pin);
    db.compact().unwrap();
    assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"v2");
}

// --- A7.2 two-stage retrieve (exact dense rerank) end-to-end -----------------

fn seed_two_stage_cells(db: &mut Database) {
    // Both cells match "alpha beta" lexically; cell 1 additionally has a title
    // match (so it wins by default), but cell 2's vector is the exact semantic
    // match for the query vector 0,10.
    db.put_cell(
        CellId(1),
        b"scope=default\nstatus=ready\ntitle=alpha beta\nvector=10,0\n\nalpha beta".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=default\nstatus=ready\nvector=0,10\n\nalpha beta".to_vec(),
    )
    .unwrap();
}

#[test]
fn two_stage_rerank_default_off_keeps_lexical_order_through_retrieve_aql() {
    // The query carries a vector, but with the rerank knob off the lexical title
    // match (cell 1) wins in Fast mode (lexical 55%, semantic only 10%).
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_two_stage_cells(&mut db);
    let view = test_view([RetrievalMode::Fast]);
    let results = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "query_vector=0,10\nalpha beta" IN BRAIN default USING MODE fast LIMIT 10 CANDIDATES;"#,
            &view,
        )
        .unwrap();
    assert_eq!(
        results[0].cell_id,
        CellId(1),
        "with rerank off the lexical title match ranks first"
    );
}

#[test]
fn two_stage_rerank_promotes_semantic_match_through_retrieve_aql() {
    // Same corpus and query, but the two-stage rerank knob is on at full weight:
    // the exact dense match against the query vector (cell 2) is promoted to the
    // top even though cell 1 has the stronger lexical signal.
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open_with_options(
        dir.path(),
        crate::options::DatabaseOptions {
            retrieval_two_stage_rerank_weight_q16: Some(65_535),
            ..crate::options::DatabaseOptions::default()
        },
    )
    .unwrap();
    seed_two_stage_cells(&mut db);
    let view = test_view([RetrievalMode::Fast]);
    let results = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "query_vector=0,10\nalpha beta" IN BRAIN default USING MODE fast LIMIT 10 CANDIDATES;"#,
            &view,
        )
        .unwrap();
    assert_eq!(
        results[0].cell_id,
        CellId(2),
        "dense rerank promotes the exact semantic match to the top"
    );
}

// --- A4.2 temporal supersession end-to-end -----------------------------------

fn seed_superseded_facts(db: &mut Database) {
    // Two versions of the same temporal fact (subject=Apollo, metric=budget);
    // cell 2 is newer (as_of 2025-06-01 vs 2025-01-01).
    db.put_cell(
        CellId(1),
        b"scope=default\nstatus=ready\ntype=fact\nas_of=2025-01-01\n\nproject=Apollo\nmetric=budget\nvalue=12000".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=default\nstatus=ready\ntype=fact\nas_of=2025-06-01\n\nproject=Apollo\nmetric=budget\nvalue=14000".to_vec(),
    )
    .unwrap();
}

#[test]
fn temporal_supersession_off_by_default_returns_all_versions() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_superseded_facts(&mut db);
    let view = test_view([RetrievalMode::Balanced]);
    let results = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "Apollo budget" IN BRAIN default LIMIT 10 CANDIDATES;"#,
            &view,
        )
        .unwrap();
    let ids = results.iter().map(|c| c.cell_id.0).collect::<BTreeSet<_>>();
    assert!(
        ids.contains(&1) && ids.contains(&2),
        "default keeps both fact versions: {ids:?}"
    );
}

#[test]
fn temporal_supersession_returns_only_the_newest_fact_when_enabled() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open_with_options(
        dir.path(),
        crate::options::DatabaseOptions {
            retrieval_suppress_superseded: true,
            ..crate::options::DatabaseOptions::default()
        },
    )
    .unwrap();
    seed_superseded_facts(&mut db);
    let view = test_view([RetrievalMode::Balanced]);
    let results = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "Apollo budget" IN BRAIN default LIMIT 10 CANDIDATES;"#,
            &view,
        )
        .unwrap();
    let ids = results.iter().map(|c| c.cell_id.0).collect::<Vec<_>>();
    assert_eq!(
        ids,
        vec![2],
        "only the newest fact (cell 2, as_of 2025-06-01) survives: {ids:?}"
    );
}

// --- A5/A7.3: USING DIVERSITY as a per-query AQL option ----------------------

#[test]
fn aql_using_diversity_demotes_near_duplicate_through_retrieve_aql() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    // Cells 1 and 2 are near-identical vectors; cell 3 is orthogonal. All share
    // "common", so all are retrieved; the default lexical rank keeps the
    // near-duplicate (2) ahead of the orthogonal cell (3).
    db.put_cell(
        CellId(1),
        b"scope=default\nstatus=ready\nvector=100,0\n\ncommon alpha".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=default\nstatus=ready\nvector=99,1\n\ncommon alpha near".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(3),
        b"scope=default\nstatus=ready\nvector=0,100\n\ncommon".to_vec(),
    )
    .unwrap();
    let view = test_view([RetrievalMode::Balanced]);

    // Without the clause: pack is the plain ranked order (no diversification).
    let base = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "common alpha" IN BRAIN default LIMIT 10 CANDIDATES;"#,
            &view,
        )
        .unwrap();
    assert!(base.len() >= 3, "all three cells retrieved: {}", base.len());

    // With `USING DIVERSITY`: the orthogonal cell 3 is promoted above the
    // near-duplicate cell 2.
    let diversified = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "common alpha" IN BRAIN default USING DIVERSITY 20000 LIMIT 10 CANDIDATES;"#,
            &view,
        )
        .unwrap();
    let pos = |id: u64| diversified.iter().position(|c| c.cell_id.0 == id).unwrap();
    assert!(
        pos(3) < pos(2),
        "USING DIVERSITY must demote the near-duplicate cell 2 below the orthogonal cell 3: {:?}",
        diversified.iter().map(|c| c.cell_id.0).collect::<Vec<_>>()
    );
}

#[test]
fn aql_suppress_superseded_clause_returns_only_the_newest_fact() {
    // A4.2 as a per-query AQL flag (no DatabaseOptions knob set).
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_superseded_facts(&mut db);
    let view = test_view([RetrievalMode::Balanced]);
    let results = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "Apollo budget" IN BRAIN default SUPPRESS SUPERSEDED LIMIT 10 CANDIDATES;"#,
            &view,
        )
        .unwrap();
    let ids = results.iter().map(|c| c.cell_id.0).collect::<Vec<_>>();
    assert_eq!(
        ids,
        vec![2],
        "SUPPRESS SUPERSEDED keeps only the newest fact: {ids:?}"
    );
}
