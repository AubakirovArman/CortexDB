use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{
    scope_id, CellMetadata, ContextPack, ContextPackOptions, Database, DatabaseOptions,
    EngineError, RetrievedCell, SemanticCompressionOptions, SemanticCompressionRequest,
    SemanticCompressionSourceRef,
};

#[test]
fn semantic_compression_requires_feature_flag() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();

    let error = db
        .commit_semantic_memory_compression(&view("project:alpha"), request())
        .unwrap_err();

    assert!(matches!(
        error,
        EngineError::FeatureDisabled("semantic_compression")
    ));
}

#[test]
fn semantic_compression_commits_external_summary_with_audit_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = open_enabled(dir.path());
    seed_source_memories(&mut db);

    let report = db
        .commit_semantic_memory_compression(&view("project:alpha"), request())
        .unwrap();

    assert_eq!(report.summary_cell_id, CellId(99));
    assert_eq!(report.source_cell_ids, vec![CellId(1), CellId(2)]);
    assert_eq!(report.source_ref_count, 2);
    assert_eq!(report.answerability_q16, 60_000);
    assert!(report.provenance_preserved);
    assert!(report.auditable);

    let payload = db.get_latest_cell(CellId(99)).unwrap();
    let metadata = CellMetadata::from_payload(&payload);
    assert_eq!(
        metadata.compression_kind.as_deref(),
        Some("semantic_summary")
    );
    assert_eq!(
        metadata.compression_source_cells,
        vec![CellId(1), CellId(2)]
    );
    assert_eq!(metadata.compression_answerability_q16, Some(60_000));
    assert_eq!(
        metadata.compression_worker.as_deref(),
        Some("mcp-summary-v1")
    );

    let pack = ContextPack::from_retrieved_with_options(
        vec![RetrievedCell::from_payload(CellId(99), payload)],
        1_000,
        false,
        &ContextPackOptions::default(),
        "alpha rollout OAuth mitigation",
    );
    assert_eq!(
        pack.cells[0].metadata.compression_source_cells,
        vec![CellId(1), CellId(2)]
    );
    assert!(pack.answerability_q16 > 0);
}

#[test]
fn semantic_compression_rejects_unreadable_source_cell() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = open_enabled(dir.path());
    seed_source_memories(&mut db);

    let error = db
        .commit_semantic_memory_compression(&view("project:alpha"), private_source_request())
        .unwrap_err();

    assert!(matches!(error, EngineError::AqlBind(_)));
}

#[test]
fn semantic_compression_rejects_low_answerability() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = open_enabled(dir.path());
    seed_source_memories(&mut db);
    let mut request = request();
    request.answerability_q16 = 10_000;
    request.summary_payload = summary_payload(10_000);

    let error = db
        .commit_semantic_memory_compression(&view("project:alpha"), request)
        .unwrap_err();

    assert!(matches!(error, EngineError::InvalidSemanticCompression(_)));
}

#[test]
fn semantic_compression_candidates_require_feature_flag() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();

    let error = db
        .semantic_compression_candidates(&view("project:alpha"), "project:alpha", 1_080, 32_768, 8)
        .unwrap_err();

    assert!(matches!(
        error,
        EngineError::FeatureDisabled("semantic_compression")
    ));
}

#[test]
fn semantic_compression_candidates_reject_unreadable_scope() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = open_enabled(dir.path());
    seed_consolidation_memories(&mut db);

    // view("project:alpha") cannot read tenant:private.
    let error = db
        .semantic_compression_candidates(&view("project:alpha"), "tenant:private", 1_080, 32_768, 8)
        .unwrap_err();

    assert!(matches!(error, EngineError::AqlBind(_)));
}

#[test]
fn semantic_compression_candidates_select_stale_episodic_grouped_by_subtype() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = open_enabled(dir.path());
    seed_consolidation_memories(&mut db);

    let groups = db
        .semantic_compression_candidates(&view("project:alpha"), "project:alpha", 1_080, 32_768, 8)
        .unwrap();

    // Deterministic: groups by subtype key (observation < workflow_result); within a
    // group, stalest first then cell id. Only stale episodic non-summary,
    // non-already-sourced cells appear.
    let flat: Vec<(Option<String>, Vec<CellId>)> = groups
        .iter()
        .map(|group| {
            (
                group.memory_type.clone(),
                group.candidates.iter().map(|c| c.cell_id).collect(),
            )
        })
        .collect();
    assert_eq!(
        flat,
        vec![
            (Some("observation".to_owned()), vec![CellId(10), CellId(14)]),
            (Some("workflow_result".to_owned()), vec![CellId(11)]),
        ]
    );

    // Excluded: fresh (12), semantic decision (13), the summary itself (20), and the
    // cell already consolidated into that summary (30).
    let selected: BTreeSet<CellId> = groups
        .iter()
        .flat_map(|group| group.candidates.iter().map(|c| c.cell_id))
        .collect();
    for excluded in [CellId(12), CellId(13), CellId(20), CellId(30)] {
        assert!(
            !selected.contains(&excluded),
            "{excluded:?} must be excluded"
        );
    }
}

#[test]
fn semantic_compression_candidates_are_deterministic_and_respect_max_groups() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = open_enabled(dir.path());
    seed_consolidation_memories(&mut db);

    let first = db
        .semantic_compression_candidates(&view("project:alpha"), "project:alpha", 1_080, 32_768, 8)
        .unwrap();
    let second = db
        .semantic_compression_candidates(&view("project:alpha"), "project:alpha", 1_080, 32_768, 8)
        .unwrap();
    assert_eq!(first, second, "candidate selection must be deterministic");

    // Capping at one group keeps only the first subtype (observation).
    let capped = db
        .semantic_compression_candidates(&view("project:alpha"), "project:alpha", 1_080, 32_768, 1)
        .unwrap();
    assert_eq!(capped.len(), 1);
    assert_eq!(capped[0].memory_type.as_deref(), Some("observation"));

    // max_groups = 0 selects nothing.
    let none = db
        .semantic_compression_candidates(&view("project:alpha"), "project:alpha", 1_080, 32_768, 0)
        .unwrap();
    assert!(none.is_empty());
}

#[test]
fn compression_sources_requires_feature_flag() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();

    let error = db
        .compression_sources(&view("project:alpha"), CellId(99))
        .unwrap_err();

    assert!(matches!(
        error,
        EngineError::FeatureDisabled("semantic_compression")
    ));
}

#[test]
fn compression_sources_resolves_present_and_missing() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = open_enabled(dir.path());
    seed_source_memories(&mut db);
    db.commit_semantic_memory_compression(&view("project:alpha"), request())
        .unwrap();

    let report = db
        .compression_sources(&view("project:alpha"), CellId(99))
        .unwrap();
    assert_eq!(report.source_cell_ids, vec![CellId(1), CellId(2)]);
    assert!(report.missing_source_cell_ids.is_empty());
    assert!(report.all_sources_present);

    // Removing a source reports it missing rather than silently dropping it.
    db.forget_cell(CellId(1)).unwrap();
    let report = db
        .compression_sources(&view("project:alpha"), CellId(99))
        .unwrap();
    assert_eq!(report.source_cell_ids, vec![CellId(2)]);
    assert_eq!(report.missing_source_cell_ids, vec![CellId(1)]);
    assert!(!report.all_sources_present);
}

#[test]
fn compression_sources_rejects_non_summary_cell() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = open_enabled(dir.path());
    seed_source_memories(&mut db);

    // CellId(1) is a plain source memory, not a semantic summary.
    let error = db
        .compression_sources(&view("project:alpha"), CellId(1))
        .unwrap_err();

    assert!(matches!(error, EngineError::InvalidSemanticCompression(_)));
}

#[test]
fn compression_sources_fail_closed_on_unreadable_source() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = open_enabled(dir.path());
    // A private source the alpha view cannot read.
    db.put_cell(
        CellId(50),
        b"scope=tenant:private\nstatus=ready\ntype=memory\nmemory_type=observation\nsource=x\n\nprivate source"
            .to_vec(),
    )
    .unwrap();
    // A summary in the readable scope that references the unreadable source.
    db.put_cell(
        CellId(51),
        b"scope=project:alpha\nstatus=ready\ntype=memory\nmemory_type=observation\ncompression_kind=semantic_summary\ncompression_source_cells=50\ncompression_answerability_q16=60000\ncompression_worker=w\nsource=w\n\nsummary"
            .to_vec(),
    )
    .unwrap();

    // The summary is readable but a source is not: fail closed.
    let error = db
        .compression_sources(&view("project:alpha"), CellId(51))
        .unwrap_err();
    assert!(matches!(error, EngineError::AqlBind(_)));
}

#[test]
fn retire_compression_sources_requires_feature_flag() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();

    let error = db
        .retire_compression_sources(&view("project:alpha"), CellId(99), 10, 2_000_000_000)
        .unwrap_err();
    assert!(matches!(
        error,
        EngineError::FeatureDisabled("semantic_compression")
    ));
}

#[test]
fn retire_compression_sources_demotes_episodic_only() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = open_enabled(dir.path());
    // seed_source_memories: cell 1 = memory_type=decision (semantic),
    // cell 2 = memory_type=observation (episodic). request() summarizes both.
    seed_source_memories(&mut db);
    db.commit_semantic_memory_compression(&view("project:alpha"), request())
        .unwrap();

    let now = 2_000_000_000;
    let report = db
        .retire_compression_sources(&view("project:alpha"), CellId(99), 10, now)
        .unwrap();

    // The episodic source (2) is demoted; the semantic source (1) is skipped.
    let retired: Vec<CellId> = report.retired.iter().map(|r| r.cell_id).collect();
    assert_eq!(retired, vec![CellId(2)]);
    assert!(report.skipped.contains(&CellId(1)));

    // The demoted source now expires at now + demote_ttl, with its body intact.
    let payload = db.get_latest_cell(CellId(2)).unwrap();
    let metadata = CellMetadata::from_payload(&payload);
    let created = metadata.created_unix_seconds.unwrap();
    let ttl = metadata.ttl_seconds.unwrap();
    assert_eq!(created + ttl, now + 10, "expires at now + demote_ttl");
    assert!(
        String::from_utf8_lossy(&payload).contains("Mitigation is to split"),
        "body must be preserved"
    );
}

fn seed_consolidation_memories(db: &mut Database) {
    let mem = |cell: u64, memory_type: &str, created: u64, ttl: u64, body: &str| -> Vec<u8> {
        format!(
            "scope=project:alpha\nstatus=ready\ntype=memory\nmemory_type={memory_type}\ncreated_unix_seconds={created}\nttl_seconds={ttl}\nsource=test\n\n{body} {cell}"
        )
        .into_bytes()
    };
    // Stale episodic (freshness 13107 < 32768 at now=1080, ttl=100).
    db.put_cell(CellId(10), mem(10, "observation", 1_000, 100, "stale obs"))
        .unwrap();
    db.put_cell(
        CellId(11),
        mem(11, "workflow_result", 1_000, 100, "stale wf"),
    )
    .unwrap();
    db.put_cell(CellId(14), mem(14, "observation", 1_000, 100, "stale obs"))
        .unwrap();
    // Fresh episodic (freshness 60292 >= 32768) — excluded.
    db.put_cell(
        CellId(12),
        mem(12, "observation", 1_000, 1_000, "fresh obs"),
    )
    .unwrap();
    // Stale but semantic — excluded by class.
    db.put_cell(
        CellId(13),
        mem(13, "decision", 1_000, 100, "semantic decision"),
    )
    .unwrap();
    // Stale episodic that is already consolidated into the summary below — excluded.
    db.put_cell(
        CellId(30),
        mem(30, "observation", 1_000, 100, "already sourced"),
    )
    .unwrap();
    // A summary consolidating cell 30 (compression_kind set) — excluded as a summary.
    db.put_cell(
        CellId(20),
        b"scope=project:alpha\nstatus=ready\ntype=memory\nmemory_type=observation\ncompression_kind=semantic_summary\ncompression_source_cells=30\ncompression_answerability_q16=60000\ncompression_worker=mcp-summary-v1\nsource=mcp-summary-v1\n\nSummary of observation 30."
            .to_vec(),
    )
    .unwrap();
}

fn open_enabled(path: &std::path::Path) -> Database {
    Database::open_with_options(
        path,
        DatabaseOptions {
            semantic_compression: SemanticCompressionOptions {
                enabled: true,
                min_answerability_q16: 32_768,
            },
            ..DatabaseOptions::default()
        },
    )
    .unwrap()
}

fn seed_source_memories(db: &mut Database) {
    db.put_cell(
        CellId(1),
        b"scope=project:alpha\nstatus=ready\ntype=memory\nmemory_type=decision\nsource=slack\n\nAlpha rollout risk is OAuth approval."
            .to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:alpha\nstatus=ready\ntype=memory\nmemory_type=observation\nsource=jira\n\nMitigation is to split mobile login rollout."
            .to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(3),
        b"scope=tenant:private\nstatus=ready\ntype=memory\nmemory_type=decision\nsource=private\n\nPrivate source."
            .to_vec(),
    )
    .unwrap();
}

fn request() -> SemanticCompressionRequest {
    SemanticCompressionRequest {
        agent_id: AgentId(7),
        scope: "project:alpha".to_owned(),
        summary_cell_id: Some(CellId(99)),
        summary_payload: summary_payload(60_000),
        source_refs: vec![
            SemanticCompressionSourceRef {
                source_cell_id: CellId(1),
                source_byte_start: 0,
                source_byte_end: 64,
            },
            SemanticCompressionSourceRef {
                source_cell_id: CellId(2),
                source_byte_start: 0,
                source_byte_end: 72,
            },
        ],
        answerability_q16: 60_000,
        external_worker: "mcp-summary-v1".to_owned(),
        idempotency_key: Some("alpha-summary-1".to_owned()),
    }
}

fn private_source_request() -> SemanticCompressionRequest {
    let mut request = request();
    request.source_refs = vec![SemanticCompressionSourceRef {
        source_cell_id: CellId(3),
        source_byte_start: 0,
        source_byte_end: 10,
    }];
    request.summary_payload =
        b"scope=project:alpha\nstatus=ready\ntype=memory\nmemory_type=observation\ncompression_kind=semantic_summary\ncompression_source_cells=3\ncompression_answerability_q16=60000\ncompression_worker=mcp-summary-v1\nsource=mcp-summary-v1\n\nPrivate summary."
            .to_vec();
    request
}

fn summary_payload(answerability_q16: u16) -> Vec<u8> {
    format!(
        "scope=project:alpha\nstatus=ready\ntype=memory\nmemory_type=observation\ncompression_kind=semantic_summary\ncompression_source_cells=1,2\ncompression_answerability_q16={answerability_q16}\ncompression_worker=mcp-summary-v1\nsource=mcp-summary-v1\n\nAlpha rollout OAuth risk is covered by split mobile login mitigation."
    )
    .into_bytes()
}

fn view(scope: &str) -> AgentView {
    let scope = scope_id(scope);
    AgentView {
        agent_id: AgentId(7),
        label: Some("semantic-compression-test".to_owned()),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope]),
        writable_scopes: BTreeSet::from([scope]),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision, MemoryType::Observation]),
        max_context_budget_tokens: 4_000,
        default_context_budget_tokens: 1_000,
        max_candidate_limit: 32,
        allow_remember: true,
        allow_verify_fact: true,
        allow_audit_mode: true,
        private_scope: Some(scope),
        max_ttl_seconds: None,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        require_citations_by_default: false,
    }
}
