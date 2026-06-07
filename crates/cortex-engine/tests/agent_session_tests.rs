use cortex_aql::{AgentId, AgentView, BindError, MemoryType, PolicyError, ScopeId};
use cortex_engine::{scope_id, Database, EngineError};
use tempfile::tempdir;

fn view(scope: &str) -> AgentView {
    AgentView {
        agent_id: AgentId(7),
        label: None,
        readable_brains: Default::default(),
        readable_scopes: [scope_id(scope)].into_iter().collect(),
        writable_scopes: [scope_id(scope)].into_iter().collect(),
        allowed_modes: Default::default(),
        allowed_memory_types: [
            MemoryType::WorkflowResult,
            MemoryType::Observation,
            MemoryType::Decision,
        ]
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
fn agent_session_records_context_and_temporary_memory() {
    let dir = tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let view = view("agent:finance");

    let session = db
        .start_agent_session(&view, "agent:finance", b"review capex first", 60, 1_000)
        .unwrap();
    let memory = db
        .remember_session_memory(&session, &view, b"temporary note", None, 1_010)
        .unwrap();

    assert_eq!(memory.session_id, session.session_id);
    assert_eq!(memory.ttl_seconds, 50);

    let cells = db.retrieve_session_cells(&session.session_id, &view, 1_020);
    assert_eq!(cells.len(), 2);
    assert!(cells[0]
        .payload
        .windows("session_kind=context".len())
        .any(|w| w == b"session_kind=context"));
    assert!(cells[1]
        .payload
        .windows("temporary note".len())
        .any(|w| w == b"temporary note"));
}

#[test]
fn session_retrieval_filters_by_session_and_ttl() {
    let dir = tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let view = view("agent:finance");

    let first = db
        .start_agent_session(&view, "agent:finance", b"first", 30, 1_000)
        .unwrap();
    let second = db
        .start_agent_session(&view, "agent:finance", b"second", 30, 1_001)
        .unwrap();
    db.remember_session_memory(&first, &view, b"first note", Some(20), 1_005)
        .unwrap();
    db.remember_session_memory(&second, &view, b"second note", Some(20), 1_006)
        .unwrap();

    let first_cells = db.retrieve_session_cells(&first.session_id, &view, 1_010);
    assert_eq!(first_cells.len(), 2);
    assert!(first_cells.iter().all(|cell| {
        !cell
            .payload
            .windows("second note".len())
            .any(|w| w == b"second note")
    }));

    assert!(db
        .retrieve_session_cells(&first.session_id, &view, 1_031)
        .is_empty());
}

#[test]
fn session_memory_survives_restart() {
    let dir = tempdir().unwrap();
    let session_id = {
        let mut db = Database::open(dir.path()).unwrap();
        let view = view("agent:finance");
        let session = db
            .start_agent_session(&view, "agent:finance", b"restart context", 60, 1_000)
            .unwrap();
        db.remember_session_memory(&session, &view, b"restart note", Some(30), 1_010)
            .unwrap();
        session.session_id
    };

    let db = Database::open(dir.path()).unwrap();
    let cells = db.retrieve_session_cells(&session_id, &view("agent:finance"), 1_020);
    assert_eq!(cells.len(), 2);
    assert!(cells.iter().any(|cell| {
        cell.payload
            .windows("restart note".len())
            .any(|w| w == b"restart note")
    }));
}

#[test]
fn session_policy_denies_unwritable_scope() {
    let dir = tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let view = view("agent:finance");

    let error = db
        .start_agent_session(&view, "agent:private", b"blocked", 60, 1_000)
        .unwrap_err();
    assert!(matches!(
        error,
        EngineError::AqlBind(BindError::PolicyDenied(PolicyError::ScopeNotWritable))
    ));
}

#[test]
fn session_memory_cannot_outlive_session() {
    let dir = tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let view = view("agent:finance");
    let session = db
        .start_agent_session(&view, "agent:finance", b"context", 60, 1_000)
        .unwrap();

    let error = db
        .remember_session_memory(&session, &view, b"too long", Some(61), 1_000)
        .unwrap_err();
    assert!(matches!(
        error,
        EngineError::AqlBind(BindError::PolicyDenied(PolicyError::TtlTooLong))
    ));
}

#[test]
fn session_memory_after_expiry_is_rejected() {
    let dir = tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let view = view("agent:finance");
    let session = db
        .start_agent_session(&view, "agent:finance", b"context", 60, 1_000)
        .unwrap();

    let error = db
        .remember_session_memory(&session, &view, b"late", None, 1_060)
        .unwrap_err();
    assert!(matches!(error, EngineError::AgentSessionExpired(_)));
}

#[test]
fn unreadable_scope_cannot_retrieve_session_cells() {
    let dir = tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let readable = view("agent:finance");
    let session = db
        .start_agent_session(&readable, "agent:finance", b"context", 60, 1_000)
        .unwrap();
    let mut blocked = view("agent:other");
    blocked.writable_scopes.insert(ScopeId(0));

    assert!(db
        .retrieve_session_cells(&session.session_id, &blocked, 1_010)
        .is_empty());
}
