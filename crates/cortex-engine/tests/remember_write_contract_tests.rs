use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_engine::verification::VerificationStatus;
use cortex_engine::{scope_id, Database};

#[test]
fn remember_allocates_unique_manifest_backed_ids_under_concurrent_calls() {
    let dir = tempfile::tempdir().unwrap();
    let db = Arc::new(Mutex::new(Database::open(dir.path()).unwrap()));
    let mut handles = Vec::new();

    for index in 0..32 {
        let db = Arc::clone(&db);
        handles.push(std::thread::spawn(move || {
            let mut db = db.lock().unwrap();
            db.remember_aql(
                &format!(
                    r#"REMEMBER "budget decision {index}" IN SCOPE project:investments AS TYPE decision;"#
                ),
                &agent_view(),
            )
            .unwrap()
            .cell_id
        }));
    }

    let mut cell_ids = BTreeSet::new();
    for handle in handles {
        cell_ids.insert(handle.join().unwrap());
    }
    assert_eq!(cell_ids.len(), 32);

    let db = Arc::try_unwrap(db).unwrap().into_inner().unwrap();
    let cursor = db
        .manifest()
        .memory_cell_cursors
        .iter()
        .find(|cursor| cursor.agent_slot == 7)
        .unwrap();
    assert_eq!(cursor.next_sequence, 32);
}

#[test]
fn remember_retrieve_verify_cycle_uses_memory_cell_as_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let remembered = db
        .remember_aql(
            r#"REMEMBER "ABC budget approved" IN SCOPE project:investments AS TYPE decision TTL 60 SECONDS;"#,
            &agent_view(),
        )
        .unwrap();

    let cells = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "ABC budget" IN BRAIN investment_projects
WHERE scope = project:investments AND type = "memory" AND memory_type = "decision" LIMIT 5 CANDIDATES;"#,
            &agent_view(),
        )
        .unwrap();
    assert_eq!(cells[0].cell_id, remembered.cell_id);

    let report = db
        .verify_fact_aql(
            r#"VERIFY FACT "ABC budget approved" IN BRAIN investment_projects;"#,
            &agent_view(),
        )
        .unwrap();
    assert_eq!(report.status, VerificationStatus::Supported);
    assert_eq!(report.evidence[0].cell_id, remembered.cell_id);
}

fn agent_view() -> AgentView {
    AgentView {
        agent_id: AgentId(7),
        label: None,
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id("project:investments")]),
        writable_scopes: BTreeSet::from([scope_id("project:investments")]),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 1_000,
        default_context_budget_tokens: 400,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3_600),
        allow_remember: true,
        allow_verify_fact: true,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}
