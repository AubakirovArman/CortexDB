use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BoundPlan, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::{CellId, KnowledgeCellMetadata, KnowledgeCellType};

use crate::database::Database;
use crate::operation::DbOperation;
use crate::query::scope_id;
use crate::search::{SearchLimit, SearchMode, SearchQuery};

#[test]
fn search_result_metadata_prefers_descriptor_for_snapshot_and_persisted_paths() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        put_spoofed_payload_with_descriptor(&mut db);
        assert_descriptor_metadata(&db);
        db.checkpoint().unwrap();
    }

    let db = Database::open(dir.path()).unwrap();
    assert_descriptor_metadata(&db);
}

#[test]
fn persisted_search_bound_plan_allowed_set_filters_status_and_where() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:wide\nstatus=ready\ntype=fact\nvector=1,0\n\napollo budget allowed"
            .to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:wide\nstatus=draft\ntype=fact\nvector=200,0\n\napollo budget budget draft"
            .to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(3),
        b"scope=project:wide\nstatus=ready\ntype=document_block\nvector=100,0\n\napollo budget budget wrong type"
            .to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(4),
        b"scope=tenant:private\nstatus=ready\ntype=fact\nvector=300,0\n\napollo budget private"
            .to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();

    let access = view("project:wide");
    let legacy_vector = db
        .search_vector(&[200, 0], &access, SearchLimit(1))
        .unwrap();
    assert_eq!(legacy_vector[0].cell_id, CellId(2));

    let aql = r#"RETRIEVE CONTEXT FOR TASK "apollo budget" IN BRAIN investment_projects
WHERE space = project:wide AND status = "ready" AND type = "fact" LIMIT 10 CANDIDATES;"#;
    let (cached, _) = db.bind_aql_cached(aql, &access).unwrap();
    let BoundPlan::Retrieve(plan) = cached.bound_plan else {
        panic!("expected retrieve plan");
    };

    let keyword = db
        .search_cells_with_bound_retrieve_plan(
            SearchQuery {
                text: "apollo budget",
                vector: None,
                limit: 10,
                mode: SearchMode::Keyword,
            },
            &access,
            plan.as_ref(),
        )
        .unwrap();
    assert_eq!(cell_ids(&keyword.results), vec![CellId(1)]);

    let vector = db
        .search_cells_with_bound_retrieve_plan(
            SearchQuery {
                text: "",
                vector: Some(&[200, 0]),
                limit: 10,
                mode: SearchMode::Vector,
            },
            &access,
            plan.as_ref(),
        )
        .unwrap();
    assert_eq!(cell_ids(&vector.results), vec![CellId(1)]);

    let hybrid = db
        .search_cells_with_bound_retrieve_plan(
            SearchQuery {
                text: "apollo budget",
                vector: Some(&[200, 0]),
                limit: 10,
                mode: SearchMode::Hybrid,
            },
            &access,
            plan.as_ref(),
        )
        .unwrap();
    assert_eq!(cell_ids(&hybrid.results), vec![CellId(1)]);
}

fn put_spoofed_payload_with_descriptor(db: &mut Database) {
    let descriptor_metadata = KnowledgeCellMetadata {
        scope: "project:investments".to_owned(),
        status: "ready".to_owned(),
        cell_type: KnowledgeCellType::Fact,
        created_unix_seconds: Some(1_717_171_717),
        source_trust_q16: Some(54_321),
        source: Some("descriptor-source".to_owned()),
        ..KnowledgeCellMetadata::default()
    };
    let spoofed_payload = concat!(
        "scope=tenant:spoofed\n",
        "status=draft\n",
        "type=memory\n",
        "created_unix_seconds=1\n",
        "source_trust_q16=1\n",
        "source=payload-source\n",
        "\n",
        "budget descriptor evidence"
    )
    .as_bytes()
    .to_vec();

    db.append_then_apply_with_metadata(
        DbOperation::PutCell {
            cell_id: CellId(44),
            payload: spoofed_payload,
        },
        descriptor_metadata.encode_wal_section(),
    )
    .unwrap();
}

fn assert_descriptor_metadata(db: &Database) {
    let results = db
        .search_keyword("budget", &view("project:investments"), SearchLimit(10))
        .unwrap();

    assert_eq!(results.len(), 1);
    let result = &results[0];
    assert_eq!(result.cell_id, CellId(44));
    assert_eq!(result.metadata.scope, "project:investments");
    assert_eq!(result.metadata.status, "ready");
    assert_eq!(result.metadata.cell_type, "fact");
    assert_eq!(result.metadata.created_unix_seconds, Some(1_717_171_717));
    assert_eq!(result.metadata.source_trust_q16, Some(54_321));
    assert_eq!(result.metadata.source.as_deref(), Some("descriptor-source"));
}

fn cell_ids(results: &[crate::search::DatabaseSearchResult]) -> Vec<CellId> {
    results.iter().map(|result| result.cell_id).collect()
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
