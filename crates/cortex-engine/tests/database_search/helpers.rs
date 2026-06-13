#![allow(unused_imports)]

pub(crate) use std::collections::BTreeSet;

pub(crate) use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
pub(crate) use cortex_core::{CellId, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};
pub(crate) use cortex_engine::search::{
    analyze_search_query, CorpusSynonymOptions, DatabaseSearchResult, QueryAnchorKind, SearchMode,
    SearchQuery, SearchRerankInput, SearchReranker,
};
pub(crate) use cortex_engine::{
    scope_id, Database, DatabaseOptions, PayloadResidency, SearchLimit,
};
pub(crate) use cortex_storage::indexes::LexicalIndex;
pub(crate) use cortex_storage::vectors::VectorIndex;

pub(crate) fn seed_private_stronger_keyword_cell(db: &mut Database) {
    db.put_cell(
        CellId(1),
        b"scope=tenant:private\nstatus=ready\n\nbudget budget budget budget budget hidden".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\n\nbudget approved".to_vec(),
    )
    .unwrap();
}

pub(crate) fn assert_keyword_limit_one_returns_public_cell(db: &Database) {
    let results = db
        .search_keyword("budget", &view("project:investments"), SearchLimit(1))
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].cell_id, CellId(2));
    let payload = String::from_utf8_lossy(&results[0].payload);
    assert!(payload.contains("approved"));
    assert!(!payload.contains("hidden"));
}

pub(crate) fn seed_private_stronger_vector_cell(db: &mut Database) {
    db.put_cell(
        CellId(1),
        b"scope=tenant:private\nstatus=ready\nvector=100,0\n\nhidden exact vector".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\nvector=1,0\n\napproved vector".to_vec(),
    )
    .unwrap();
}

pub(crate) fn assert_vector_limit_one_returns_public_cell(db: &Database) {
    let results = db
        .search_vector(&[100, 0], &view("project:investments"), SearchLimit(1))
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].cell_id, CellId(2));
    let payload = String::from_utf8_lossy(&results[0].payload);
    assert!(payload.contains("approved"));
    assert!(!payload.contains("hidden"));
}

pub(crate) fn seed_private_stronger_hybrid_cell(db: &mut Database) {
    db.put_cell(
        CellId(1),
        b"scope=tenant:private\nstatus=ready\nvector=100,0\n\nbudget budget budget hidden exact vector"
            .to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\nvector=1,0\n\nbudget approved vector".to_vec(),
    )
    .unwrap();
}

pub(crate) fn assert_hybrid_limit_one_returns_public_cell(db: &Database) {
    let results = db
        .search_cells(
            SearchQuery {
                text: "budget",
                vector: Some(&[100, 0]),
                limit: 1,
                mode: SearchMode::Hybrid,
            },
            &view("project:investments"),
        )
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].cell_id, CellId(2));
    let payload = String::from_utf8_lossy(&results[0].payload);
    assert!(payload.contains("approved"));
    assert!(!payload.contains("hidden"));
}

pub(crate) fn seed_hybrid_rerank_cells(db: &mut Database) {
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nvector=2,0\n\nbudget budget budget generic update"
            .to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\nvector=1,0\n\nbudget AUTH-123 was fixed by PR #42"
            .to_vec(),
    )
    .unwrap();
}

pub(crate) fn search_hybrid_anchor_question(
    db: &Database,
    mode: SearchMode,
) -> Vec<DatabaseSearchResult> {
    db.search_cells(
        SearchQuery {
            text: "Which PR #42 fixed AUTH-123 budget?",
            vector: Some(&[2, 0]),
            limit: 1,
            mode,
        },
        &view("project:investments"),
    )
    .unwrap()
}

pub(crate) fn seed_conflicting_policy_sources(db: &mut Database) {
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nsource_trust_q16=1000\ncreated_unix_seconds=100\n\ncurrent conflicting deployment policy says rollback approval uses the legacy runbook"
            .to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\nsource_trust_class=official\ncreated_unix_seconds=200\n\ncurrent conflicting deployment policy says rollback approval uses the incident commander runbook"
            .to_vec(),
    )
    .unwrap();
}

pub(crate) fn seed_cluster_diversity_cells(db: &mut Database) {
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\ndocument_id=doc-a\nsource=slack\nproject=apollo\n\nblocker rollout queue migration deadline owner maya alpha alpha alpha"
            .to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\ndocument_id=doc-a\nsource=slack\nproject=apollo\n\nblocker rollout queue migration deadline owner maya beta beta beta"
            .to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(3),
        b"scope=project:investments\nstatus=ready\ndocument_id=doc-b\nsource=jira\nproject=apollo\n\nblocker rollout security access deadline owner ivan"
            .to_vec(),
    )
    .unwrap();
}

pub(crate) fn assert_trusted_fresh_policy_source_first(db: &Database) {
    let results = db
        .search_cells(
            SearchQuery {
                text: "What is the current conflicting deployment policy?",
                vector: None,
                limit: 2,
                mode: SearchMode::HybridRerank,
            },
            &view("project:investments"),
        )
        .unwrap();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].cell_id, CellId(2));
    assert!(results[0].score > results[1].score);
    assert!(String::from_utf8_lossy(&results[0].payload).contains("incident commander"));
}

pub(crate) fn seed_parent_child_context_cells(db: &mut Database) {
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\ndocument_id=doc-alpha\nchunk_id=parent-alpha\nchunk_role=document\ntitle=Alpha full document\n\nParent context includes owner, deadline, and rollout notes."
            .to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\ndocument_id=doc-alpha\nchunk_id=child-alpha-1\nparent_id=parent-alpha\nchunk_role=child\nsection=Risk details\n\nspecific-child-anchor appears here."
            .to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(3),
        b"scope=tenant:private\nstatus=ready\ndocument_id=doc-alpha\nchunk_id=private-parent\nchunk_role=document\n\nPrivate parent must not be expanded."
            .to_vec(),
    )
    .unwrap();
}

pub(crate) fn search_child_anchor(db: &Database) -> Vec<DatabaseSearchResult> {
    db.search_cells(
        SearchQuery {
            text: "specific-child-anchor",
            vector: None,
            limit: 2,
            mode: SearchMode::Keyword,
        },
        &view("project:investments"),
    )
    .unwrap()
}

pub(crate) fn seed_high_level_anchor_cells(db: &mut Database) {
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\ndocument_id=company-northstar\nchunk_role=summary\ntitle=Northstar plan\n\nEnterprise context infrastructure for agents."
            .to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=tenant:private\nstatus=ready\ndocument_id=private-northstar\nchunk_role=summary\ntitle=Private Northstar\n\nPrivate strategy must not be returned."
            .to_vec(),
    )
    .unwrap();
}

pub(crate) fn search_high_level_anchor(db: &Database) -> Vec<DatabaseSearchResult> {
    db.search_keyword(
        "Give me the big picture",
        &view("project:investments"),
        SearchLimit(2),
    )
    .unwrap()
}

pub(crate) fn seed_project_artifact_cells(db: &mut Database) {
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nproject=Apollo\nowner=Maya\ntitle=Launch owner\n\nlaunch owner Maya"
            .to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\nproject=Apollo\nstatus_tag=blocked\ntitle=PR evidence\n\nPR 42 updates the service adapter."
            .to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(3),
        b"scope=project:investments\nstatus=ready\nproject=Apollo\nevent_date=2026-05-01\ntitle=Slack thread\n\nRisk was discussed in the channel."
            .to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(4),
        b"scope=tenant:private\nstatus=ready\nproject=Apollo\ntitle=Private Apollo\n\nPrivate artifact must not be expanded."
            .to_vec(),
    )
    .unwrap();
}

pub(crate) fn search_project_launch_owner(db: &Database) -> Vec<DatabaseSearchResult> {
    db.search_keyword(
        "Who owns the launch?",
        &view("project:investments"),
        SearchLimit(3),
    )
    .unwrap()
}

pub(crate) fn view(scope: &str) -> AgentView {
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
