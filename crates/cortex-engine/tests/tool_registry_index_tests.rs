use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{
    scope_id, Database, DatabaseOptions, PayloadResidency, ToolDescriptor, ToolPermission,
};

#[test]
fn tool_recommendation_term_index_tracks_patch_and_tombstone() {
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
            "source_finder",
            "finds source citations and evidence documents",
        ),
    )
    .unwrap();

    let view = view("project:investments");
    let budget_tools = db.recommend_tools_for_task(&view, "budget variance", 5);
    assert_eq!(budget_tools.len(), 1);
    assert_eq!(budget_tools[0].tool.cell_id, CellId(10));

    db.patch_cell(
        CellId(10),
        tool(
            "project:investments",
            "contract_review",
            "reviews legal contract clauses",
        )
        .to_knowledge_cell()
        .unwrap()
        .encode_payload(),
    )
    .unwrap();

    assert!(db
        .recommend_tools_for_task(&view, "budget variance", 5)
        .is_empty());
    let contract_tools = db.recommend_tools_for_task(&view, "contract clauses", 5);
    assert_eq!(contract_tools.len(), 1);
    assert_eq!(contract_tools[0].tool.cell_id, CellId(10));

    db.tombstone_cell(CellId(10)).unwrap();
    assert!(db
        .recommend_tools_for_task(&view, "contract clauses", 5)
        .is_empty());
}

#[test]
fn lazy_tool_catalog_rebuilds_index_on_open() {
    let dir = tempfile::tempdir().unwrap();
    {
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
        db.checkpoint().unwrap();
    }

    let db = Database::open_with_options(
        dir.path(),
        DatabaseOptions {
            payload_residency: PayloadResidency::Lazy,
            ..DatabaseOptions::default()
        },
    )
    .unwrap();
    assert_eq!(db.storage_stats().unwrap().memtable_payload_bytes, 0);

    let view = view("project:investments");
    let tools = db.list_tools(&view);
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].cell_id, CellId(10));
    let recommendations = db.recommend_tools_for_task(&view, "budget variance", 5);
    assert_eq!(recommendations.len(), 1);
    assert_eq!(recommendations[0].tool.cell_id, CellId(10));
    assert_eq!(db.storage_stats().unwrap().memtable_payload_bytes, 0);
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
    descriptor.source = Some("tool-registry-index-test".to_owned());
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
