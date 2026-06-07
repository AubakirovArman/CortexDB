use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::{CellId, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};
use cortex_engine::context::ContextPackOptions;
use cortex_engine::{scope_id, Database, ToolDescriptor, ToolPermission};

#[test]
fn context_pack_with_tools_includes_relevant_tool_and_explanation() {
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
            "Investment budget variance increased in Q4.".to_owned(),
        ),
    )
    .unwrap();
    db.register_tool(
        CellId(10),
        tool(
            "project:investments",
            "budget_calculator",
            "calculates investment budget variance and financial exposure",
        ),
    )
    .unwrap();
    db.register_tool(
        CellId(11),
        tool(
            "project:investments",
            "source_finder",
            "finds source citations and evidence documents",
        ),
    )
    .unwrap();

    let result = db
        .context_pack_with_tool_recommendations_from_aql(
            r#"RETRIEVE CONTEXT FOR TASK "investment budget analysis" IN BRAIN default
WHERE scope = project:investments AND type = "fact" LIMIT 5 CANDIDATES;"#,
            &view("project:investments"),
            ContextPackOptions::default(),
            3,
        )
        .unwrap();

    assert_eq!(result.pack.cells.len(), 1);
    assert_eq!(result.pack.cells[0].cell_id, CellId(1));
    assert_eq!(result.tool_recommendations.len(), 1);
    assert_eq!(result.tool_recommendations[0].tool.cell_id, CellId(10));
    assert_eq!(
        result.tool_recommendations[0].tool.descriptor.name,
        "budget_calculator"
    );
    assert!(result.tool_recommendations[0]
        .matched_terms
        .iter()
        .any(|term| term == "budget"));
    assert!(result.tool_recommendations[0]
        .why_selected
        .contains("budget"));
}

#[test]
fn context_pack_tool_recommendations_respect_agent_scope_and_limit() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.register_tool(
        CellId(10),
        tool(
            "project:investments",
            "budget_calculator",
            "calculates investment budget variance",
        ),
    )
    .unwrap();
    db.register_tool(
        CellId(11),
        tool(
            "project:investments",
            "budget_auditor",
            "audits investment budget assumptions",
        ),
    )
    .unwrap();
    db.register_tool(
        CellId(12),
        tool(
            "project:legal",
            "legal_budget_reviewer",
            "reviews legal budget obligations",
        ),
    )
    .unwrap();

    let allowed = db
        .context_pack_with_tool_recommendations_from_aql(
            r#"RETRIEVE CONTEXT FOR TASK "investment budget" IN BRAIN default
WHERE type = "tool" LIMIT 5 CANDIDATES;"#,
            &view("project:investments"),
            ContextPackOptions::default(),
            1,
        )
        .unwrap();
    assert_eq!(allowed.tool_recommendations.len(), 1);
    assert!(allowed.tool_recommendations[0]
        .tool
        .descriptor
        .scope
        .contains("project:investments"));

    let denied = db
        .context_pack_with_tool_recommendations_from_aql(
            r#"RETRIEVE CONTEXT FOR TASK "investment budget" IN BRAIN default
WHERE type = "tool" LIMIT 5 CANDIDATES;"#,
            &view("project:private"),
            ContextPackOptions::default(),
            5,
        )
        .unwrap();
    assert!(denied.tool_recommendations.is_empty());
}

fn tool(scope: &str, name: &str, description: &str) -> ToolDescriptor {
    let mut descriptor = ToolDescriptor::new(
        scope,
        name,
        description,
        BTreeSet::from([ToolPermission::Read, ToolPermission::Execute]),
    );
    descriptor.input_schema = Some(r#"{"type":"object","required":["task"]}"#.to_owned());
    descriptor.output_schema = Some(r#"{"type":"object","required":["result"]}"#.to_owned());
    descriptor.source = Some("context-pack-tool-test".to_owned());
    descriptor
}

fn view(scope: &str) -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: None,
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id(scope)]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 10_000,
        default_context_budget_tokens: 1_000,
        max_candidate_limit: 100,
        default_candidate_limit: 10,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: None,
        allow_remember: false,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}
