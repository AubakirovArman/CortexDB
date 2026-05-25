use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::{CellId, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};
use cortex_engine::{scope_id, CellMetadata, Database};
use std::collections::BTreeSet;

#[test]
fn put_knowledge_cell_encodes_metadata_for_aql_retrieve() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let cell = KnowledgeCell::new(
        KnowledgeCellMetadata {
            scope: "project:investments".to_owned(),
            status: "verified".to_owned(),
            cell_type: KnowledgeCellType::Fact,
            memory_type: None,
            ttl_seconds: None,
            created_unix_seconds: None,
            source_trust_q16: Some(60_000),
            source: Some("annual-report".to_owned()),
        },
        "Бюджет проекта ABC подтвержден",
    );
    db.put_knowledge_cell(CellId(77), cell).unwrap();

    let cells = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE space = project:investments AND status = "verified" AND type = "fact" LIMIT 10 CANDIDATES;"#,
            &view(),
        )
        .unwrap();
    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0].cell_id, CellId(77));
    assert!(String::from_utf8_lossy(&cells[0].payload).contains("source=annual-report"));
    assert!(String::from_utf8_lossy(&cells[0].payload).contains("source_trust_q16=60000"));
}

#[test]
fn knowledge_cell_metadata_sanitizes_header_lines() {
    let cell = KnowledgeCell::new(
        KnowledgeCellMetadata {
            scope: "tenant\nalpha".to_owned(),
            ..KnowledgeCellMetadata::default()
        },
        "payload",
    );
    let payload = String::from_utf8(cell.encode_payload()).unwrap();
    assert!(payload.contains("scope=tenant alpha"));
}

#[test]
fn cell_metadata_parses_header_and_body_separately() {
    let payload = concat!(
        "scope=project:investments\n",
        "status=verified\n",
        "type=fact\n",
        "source=annual-report\n",
        "citation=page-7\n",
        "source_trust_q16=60000\n",
        "\n",
        "body line\n",
        "scope=not-header"
    );
    let metadata = CellMetadata::from_payload(payload.as_bytes());
    assert_eq!(metadata.scope, "project:investments");
    assert_eq!(metadata.status, "verified");
    assert_eq!(metadata.cell_type, "fact");
    assert_eq!(metadata.source_trust_q16, Some(60_000));
    assert_eq!(metadata.citation(), Some("page-7"));
    assert!(metadata.body_text.contains("scope=not-header"));
    assert!(metadata.terms.contains(&"body".to_owned()));
}

#[test]
fn remember_aql_writes_policy_checked_memory_cell() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let result = db
        .remember_aql(
            r#"REMEMBER "use conservative budget" IN SCOPE project:investments AS TYPE decision TTL 60 SECONDS;"#,
            &memory_view(true),
        )
        .unwrap();
    assert_eq!(result.ttl_seconds, Some(60));

    let payload = db.get_latest_cell(result.cell_id).unwrap();
    let text = String::from_utf8_lossy(&payload);
    assert!(text.contains("scope=project:investments"));
    assert!(text.contains("type=memory"));
    assert!(text.contains("memory_type=decision"));
    assert!(text.contains("ttl_seconds=60"));
    assert!(text.contains("created_unix_seconds="));
    assert!(text.contains("use conservative budget"));

    let cells = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "memory" IN BRAIN investment_projects
WHERE scope = project:investments AND type = "memory" AND memory_type = "decision" LIMIT 10 CANDIDATES;"#,
            &memory_view(true),
        )
        .unwrap();
    assert_eq!(cells[0].cell_id, result.cell_id);
}

#[test]
fn remember_aql_denied_without_write_scope() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let error = db
        .remember_aql(
            r#"REMEMBER "blocked" IN SCOPE project:investments AS TYPE decision;"#,
            &memory_view(false),
        )
        .unwrap_err();
    assert!(error.to_string().contains("ScopeNotWritable"));
}

#[test]
fn remember_aql_survives_restart() {
    let dir = tempfile::tempdir().unwrap();
    let cell_id = {
        let mut db = Database::open(dir.path()).unwrap();
        let result = db
            .remember_aql(
                r#"REMEMBER "persist this memory" IN SCOPE project:investments AS TYPE decision;"#,
                &memory_view(true),
            )
            .unwrap();
        result.cell_id
    };

    let db = Database::open(dir.path()).unwrap();
    let payload = db.get_latest_cell(cell_id).unwrap();
    assert!(String::from_utf8_lossy(&payload).contains("persist this memory"));
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

fn memory_view(can_write: bool) -> AgentView {
    let mut view = view();
    view.allow_remember = true;
    if can_write {
        view.writable_scopes = BTreeSet::from([scope_id("project:investments")]);
    }
    view
}
