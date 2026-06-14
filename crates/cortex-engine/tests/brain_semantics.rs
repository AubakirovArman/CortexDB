use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::{CellId, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};
use cortex_engine::{scope_id, Database};

#[test]
fn aql_brain_names_are_deprecated_aliases_for_default_brain() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_knowledge_cell(
        CellId(1),
        KnowledgeCell::new(
            KnowledgeCellMetadata {
                scope: "project:investments".to_owned(),
                status: "ready".to_owned(),
                cell_type: KnowledgeCellType::Fact,
                ..KnowledgeCellMetadata::default()
            },
            "ABC budget approved",
        ),
    )
    .unwrap();

    let default_cells = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN default
WHERE scope = project:investments LIMIT 10 CANDIDATES;"#,
            &view_with_brains([BrainId(1)]),
        )
        .unwrap();
    let legacy_alias_cells = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects
WHERE scope = project:investments LIMIT 10 CANDIDATES;"#,
            &view_with_brains([BrainId(1)]),
        )
        .unwrap();

    assert_eq!(default_cells.len(), 1);
    assert_eq!(legacy_alias_cells.len(), 1);
    assert_eq!(default_cells[0].cell_id, legacy_alias_cells[0].cell_id);
}

#[test]
fn single_brain_contract_still_requires_default_brain_permission() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    let error = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN any_alias
WHERE scope = project:investments LIMIT 10 CANDIDATES;"#,
            &view_with_brains([BrainId(2)]),
        )
        .unwrap_err();

    assert!(error.to_string().contains("BrainNotReadable"));
}

fn view_with_brains(brains: impl IntoIterator<Item = BrainId>) -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: None,
        readable_brains: brains.into_iter().collect(),
        readable_scopes: BTreeSet::from([scope_id("project:investments")]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 1_000,
        default_context_budget_tokens: 400,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: None,
        allow_remember: false,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}
