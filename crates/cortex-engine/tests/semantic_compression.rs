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
