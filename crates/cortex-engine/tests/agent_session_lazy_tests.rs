use cortex_aql::{AgentId, AgentView, MemoryType};
use cortex_engine::{scope_id, Database, DatabaseOptions, PayloadResidency};
use tempfile::tempdir;

fn view(scope: &str) -> AgentView {
    AgentView {
        agent_id: AgentId(7),
        label: None,
        readable_brains: Default::default(),
        readable_scopes: [scope_id(scope)].into_iter().collect(),
        writable_scopes: [scope_id(scope)].into_iter().collect(),
        allowed_modes: Default::default(),
        allowed_memory_types: [MemoryType::WorkflowResult, MemoryType::Observation]
            .into_iter()
            .collect(),
        max_context_budget_tokens: 10_000,
        default_context_budget_tokens: 1_000,
        max_candidate_limit: 100,
        default_candidate_limit: 10,
        min_required_confidence_q16: Default::default(),
        max_ttl_seconds: Some(3_600),
        allow_remember: true,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}

#[test]
fn lazy_session_retrieval_loads_only_matching_session_payloads() {
    let dir = tempdir().unwrap();
    let (first_session_id, second_session_id) = {
        let mut db = Database::open(dir.path()).unwrap();
        let view = view("agent:finance");
        let first = db
            .start_agent_session(&view, "agent:finance", b"first context", 120, 1_000)
            .unwrap();
        db.remember_session_memory(&first, &view, b"first note", Some(60), 1_010)
            .unwrap();
        let second = db
            .start_agent_session(&view, "agent:finance", b"second context", 120, 1_020)
            .unwrap();
        db.remember_session_memory(&second, &view, b"second note", Some(60), 1_030)
            .unwrap();
        db.checkpoint().unwrap();
        (first.session_id, second.session_id)
    };

    let db = Database::open_with_options(
        dir.path(),
        DatabaseOptions {
            payload_residency: PayloadResidency::Lazy,
            ..DatabaseOptions::default()
        },
    )
    .unwrap();
    assert_eq!(db.storage_stats().unwrap().memtable_payload_bytes, 0);
    assert_eq!(db.payload_cache_stats().segment_loads, 0);

    let cells = db.retrieve_session_cells(&first_session_id, &view("agent:finance"), 1_040);
    assert_eq!(cells.len(), 2);
    assert!(cells.iter().any(|cell| cell
        .payload
        .windows("first note".len())
        .any(|w| w == b"first note")));
    assert!(cells.iter().all(|cell| {
        !cell
            .payload
            .windows("second note".len())
            .any(|window| window == b"second note")
    }));
    assert_eq!(
        db.payload_cache_stats().segment_loads,
        2,
        "session index should load only the requested session payloads"
    );

    let second = db.retrieve_session_cells(&second_session_id, &view("agent:finance"), 1_040);
    assert_eq!(second.len(), 2);
}
