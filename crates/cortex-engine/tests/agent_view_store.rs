use std::collections::BTreeSet;
use std::fs;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, ScopeId};
use cortex_engine::Database;

#[test]
fn agent_view_survives_restart() {
    let dir = tempfile::tempdir().unwrap();
    {
        let db = Database::open(dir.path()).unwrap();
        db.save_agent_view(&view(AgentId(7), Some("finance\nagent")))
            .unwrap();
    }

    let db = Database::open(dir.path()).unwrap();
    let loaded = db.load_agent_view(AgentId(7)).unwrap().unwrap();
    assert_eq!(loaded.agent_id, AgentId(7));
    assert_eq!(loaded.label.as_deref(), Some("finance\nagent"));
    assert_eq!(
        loaded.readable_brains,
        BTreeSet::from([BrainId(1), BrainId(2)])
    );
    assert_eq!(
        loaded.readable_scopes,
        BTreeSet::from([ScopeId(10), ScopeId(11)])
    );
    assert_eq!(loaded.writable_scopes, BTreeSet::from([ScopeId(11)]));
    assert!(loaded.allowed_modes.contains(&RetrievalMode::Audit));
    assert!(loaded
        .allowed_memory_types
        .contains(&MemoryType::Observation));
    assert_eq!(loaded.max_ttl_seconds, Some(3_600));
    assert_eq!(loaded.private_scope, Some(ScopeId(99)));
    assert!(loaded.allow_remember);
    assert!(loaded.allow_verify_fact);
}

#[test]
fn list_agent_views_is_sorted_by_agent_id() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    db.save_agent_view(&view(AgentId(9), None)).unwrap();
    db.save_agent_view(&view(AgentId(3), None)).unwrap();

    let ids = db
        .list_agent_views()
        .unwrap()
        .into_iter()
        .map(|view| view.agent_id)
        .collect::<Vec<_>>();
    assert_eq!(ids, vec![AgentId(3), AgentId(9)]);
}

#[test]
fn unknown_agent_view_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    assert!(db.load_agent_view(AgentId(404)).unwrap().is_none());
}

#[test]
fn orphan_agent_view_temp_file_does_not_break_open() {
    let dir = tempfile::tempdir().unwrap();
    let agent_dir = dir.path().join("agent_views");
    fs::create_dir_all(&agent_dir).unwrap();
    fs::write(agent_dir.join("7.view.tmp"), b"partial").unwrap();

    let db = Database::open(dir.path()).unwrap();
    assert!(!agent_dir.join("7.view.tmp").exists());
    assert!(db.list_agent_views().unwrap().is_empty());
}

fn view(agent_id: AgentId, label: Option<&str>) -> AgentView {
    AgentView {
        agent_id,
        label: label.map(ToOwned::to_owned),
        readable_brains: BTreeSet::from([BrainId(1), BrainId(2)]),
        readable_scopes: BTreeSet::from([ScopeId(10), ScopeId(11)]),
        writable_scopes: BTreeSet::from([ScopeId(11)]),
        allowed_modes: BTreeSet::from([
            RetrievalMode::Fast,
            RetrievalMode::Balanced,
            RetrievalMode::Audit,
        ]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision, MemoryType::Observation]),
        max_context_budget_tokens: 12_000,
        default_context_budget_tokens: 4_000,
        max_candidate_limit: 500,
        default_candidate_limit: 50,
        min_required_confidence_q16: 42_000,
        max_ttl_seconds: Some(3_600),
        allow_remember: true,
        allow_verify_fact: true,
        allow_audit_mode: true,
        require_citations_by_default: true,
        private_scope: Some(ScopeId(99)),
    }
}
