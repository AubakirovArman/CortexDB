use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::context::ContextPackOptions;
use cortex_engine::{scope_id, Database, ToolDescriptor, ToolPermission};

fn descriptor(scope: &str, name: &str) -> ToolDescriptor {
    let mut descriptor = ToolDescriptor::new(
        scope,
        name,
        format!("{name} tool for budget analysis and evidence lookup"),
        BTreeSet::from([
            ToolPermission::Read,
            ToolPermission::Execute,
            ToolPermission::ApprovalRequired,
        ]),
    );
    descriptor.input_schema = Some(r#"{"type":"object","required":["query"]}"#.to_owned());
    descriptor.output_schema = Some(r#"{"type":"object","required":["answer"]}"#.to_owned());
    descriptor.source = Some("tool-registry-test".to_owned());
    descriptor
}

fn task_descriptor(scope: &str, name: &str, description: &str) -> ToolDescriptor {
    let mut descriptor = ToolDescriptor::new(
        scope,
        name,
        description,
        BTreeSet::from([ToolPermission::Read, ToolPermission::Execute]),
    );
    descriptor.input_schema = Some(r#"{"type":"object","required":["task"]}"#.to_owned());
    descriptor.output_schema = Some(r#"{"type":"object","required":["result"]}"#.to_owned());
    descriptor.source = Some("tool-registry-test".to_owned());
    descriptor
}

#[test]
fn register_tool_writes_tool_cell() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();

    let registered = db
        .register_tool(CellId(10), descriptor("project:investments", "calculator"))
        .unwrap();

    assert_eq!(registered.cell_id, CellId(10));
    assert_eq!(registered.descriptor.name, "calculator");
    let payload = String::from_utf8(db.get_latest_cell(CellId(10)).unwrap()).unwrap();
    assert!(payload.contains("type=tool"));
    assert!(payload.contains("name=calculator"));
    assert!(payload.contains("permissions=read,execute,approval_required"));
}

#[test]
fn list_tools_respects_agent_scope() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.register_tool(CellId(10), descriptor("project:investments", "calculator"))
        .unwrap();
    db.register_tool(CellId(11), descriptor("project:legal", "contract_search"))
        .unwrap();

    let tools = db.list_tools(&view("project:investments"));

    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].cell_id, CellId(10));
    assert_eq!(tools[0].descriptor.name, "calculator");
}

#[test]
fn context_pack_can_include_tool_cell() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.register_tool(CellId(10), descriptor("project:investments", "calculator"))
        .unwrap();

    let pack = db
        .context_pack_from_aql(
            r#"RETRIEVE CONTEXT FOR TASK "calculator tool" IN BRAIN default
WHERE scope = project:investments AND type = "tool" LIMIT 5 CANDIDATES;"#,
            &view("project:investments"),
            ContextPackOptions {
                token_budget_tokens: 1_000,
                ..ContextPackOptions::default()
            },
        )
        .unwrap();

    assert_eq!(pack.cells.len(), 1);
    assert_eq!(pack.cells[0].cell_id, CellId(10));
    assert!(String::from_utf8_lossy(&pack.cells[0].payload).contains("name=calculator"));
}

#[test]
fn agent_without_scope_cannot_retrieve_tool() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.register_tool(CellId(10), descriptor("project:investments", "calculator"))
        .unwrap();

    let result = db
        .context_pack_from_aql(
            r#"RETRIEVE CONTEXT FOR TASK "calculator tool" IN BRAIN default
WHERE type = "tool" LIMIT 5 CANDIDATES;"#,
            &view("project:legal"),
            ContextPackOptions {
                token_budget_tokens: 1_000,
                ..ContextPackOptions::default()
            },
        )
        .unwrap();

    assert!(result.cells.is_empty());
    assert!(db.list_tools(&view("project:legal")).is_empty());
}

#[test]
fn tool_retrieval_by_task_returns_relevant_tool_cell() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.register_tool(
        CellId(10),
        task_descriptor(
            "project:investments",
            "budget_calculator",
            "calculates investment budget variance and financial exposure",
        ),
    )
    .unwrap();
    db.register_tool(
        CellId(11),
        task_descriptor(
            "project:investments",
            "source_finder",
            "finds source citations and evidence documents",
        ),
    )
    .unwrap();
    db.register_tool(
        CellId(12),
        task_descriptor(
            "project:legal",
            "legal_review",
            "reviews contract clauses and legal obligations",
        ),
    )
    .unwrap();

    let recommendations = db.recommend_tools_for_task(
        &view("project:investments"),
        "investment budget analysis",
        2,
    );

    assert_eq!(recommendations.len(), 1);
    assert_eq!(recommendations[0].tool.cell_id, CellId(10));
    assert_eq!(recommendations[0].tool.descriptor.name, "budget_calculator");
    assert!(recommendations[0]
        .matched_terms
        .iter()
        .any(|term| term == "budget"));
    assert!(db
        .recommend_tools_for_task(&view("project:legal"), "investment budget analysis", 2)
        .is_empty());
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
