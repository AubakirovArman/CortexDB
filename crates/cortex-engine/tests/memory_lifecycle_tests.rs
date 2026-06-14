use std::collections::BTreeSet;

use cortex_aql::{
    AgentId, AgentView, BrainId, MemoryType, RetrievalMode, RetrievalWeights, Q16_ZERO,
};
use cortex_core::{CellId, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};
use cortex_engine::{scope_id, Database, DatabaseOptions, PayloadResidency, RetrievedCell};

#[test]
fn aql_retrieve_filters_expired_memory_without_manual_expire() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(CellId(1), memory_cell(100, Some(1)))
        .unwrap();
    db.put_knowledge_cell(CellId(2), memory_cell(now_unix_seconds(), Some(3_600)))
        .unwrap();

    let cells = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "memory" IN BRAIN investment_projects
WHERE scope = project:investments AND type = "memory" AND memory_type = "decision"
LIMIT 10 CANDIDATES;"#,
            &view(),
        )
        .unwrap();

    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0].cell_id, CellId(2));
}

#[test]
fn memory_lifecycle_uses_descriptor_index_after_lazy_restart() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_knowledge_cell(CellId(1), memory_cell(100, Some(10)))
            .unwrap();
        db.put_knowledge_cell(CellId(2), memory_cell(100, None))
            .unwrap();
        db.checkpoint().unwrap();
    }

    let db = Database::open_with_options(
        dir.path(),
        DatabaseOptions {
            payload_residency: PayloadResidency::Lazy,
            payload_cache_bytes: 0,
            ..DatabaseOptions::default()
        },
    )
    .unwrap();

    let expired = db.expired_memory_cells(111);
    assert_eq!(expired.len(), 1);
    assert_eq!(expired[0].cell_id, CellId(1));
    let scores = db.memory_decay_scores(150);
    assert_eq!(scores.len(), 2);
    assert_eq!(
        db.payload_cache_stats().segment_loads,
        0,
        "memory lifecycle index must not materialize lazy payloads"
    );
}

#[test]
fn memory_decay_lowers_rank_for_stale_ttl_memory() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    let now = now_unix_seconds();
    let stale = RetrievedCell::from_payload(CellId(1), memory_payload(now - 90, Some(100)));
    let fresh = RetrievedCell::from_payload(CellId(2), memory_payload(now - 10, Some(100)));

    let ranked = db.rerank_retrieved_cells_for_task(
        vec![stale, fresh],
        "memory",
        &RetrievalWeights {
            lexical_q16: u16::MAX,
            semantic_q16: 0,
            recency_q16: 0,
            trust_q16: 0,
        },
    );

    assert_eq!(ranked[0].cell_id, CellId(2));
    assert_eq!(ranked[1].cell_id, CellId(1));
}

fn now_unix_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn memory_payload(created: u64, ttl: Option<u64>) -> Vec<u8> {
    let ttl = ttl
        .map(|ttl| format!("ttl_seconds={ttl}\n"))
        .unwrap_or_default();
    format!(
        "scope=project:investments\nstatus=ready\ntype=memory\nmemory_type=decision\n{ttl}created_unix_seconds={created}\nsource=test\n\nmemory payload"
    )
    .into_bytes()
}

fn memory_cell(created: u64, ttl: Option<u64>) -> KnowledgeCell {
    KnowledgeCell::new(
        KnowledgeCellMetadata {
            scope: "project:investments".to_owned(),
            status: "ready".to_owned(),
            cell_type: KnowledgeCellType::Memory,
            memory_type: Some("decision".to_owned()),
            ttl_seconds: ttl,
            created_unix_seconds: Some(created),
            source_trust_q16: None,
            source: Some("test".to_owned()),
        },
        "memory payload",
    )
}

fn view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: None,
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id("project:investments")]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 1_000,
        default_context_budget_tokens: 400,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3_600),
        allow_remember: false,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}
