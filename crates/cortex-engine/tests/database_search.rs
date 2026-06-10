use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::search::{
    analyze_search_query, QueryAnchorKind, SearchMode, SearchQuery, SearchRerankInput,
    SearchReranker,
};
use cortex_engine::{scope_id, Database, SearchLimit};
use cortex_storage::indexes::LexicalIndex;
use cortex_storage::vectors::VectorIndex;

#[test]
fn database_keyword_search_returns_visible_cells() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nalpha budget approved".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=tenant:private\nstatus=ready\nalpha budget hidden".to_vec(),
    )
    .unwrap();

    let results = db
        .search_keyword("budget", &view("project:investments"), SearchLimit(10))
        .unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].cell_id, CellId(1));
    assert!(String::from_utf8_lossy(&results[0].payload).contains("approved"));
}

#[test]
fn database_keyword_search_applies_acl_before_topk_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_private_stronger_keyword_cell(&mut db);

    assert_keyword_limit_one_returns_public_cell(&db);
}

#[test]
fn database_keyword_search_applies_acl_before_topk_persisted() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_private_stronger_keyword_cell(&mut db);
    db.checkpoint().unwrap();

    assert_keyword_limit_one_returns_public_cell(&db);
}

#[test]
fn database_vector_search_applies_acl_before_topk_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_private_stronger_vector_cell(&mut db);

    assert_vector_limit_one_returns_public_cell(&db);
}

#[test]
fn database_vector_search_applies_acl_before_topk_persisted() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_private_stronger_vector_cell(&mut db);
    db.checkpoint().unwrap();

    assert_vector_limit_one_returns_public_cell(&db);
}

#[test]
fn database_hybrid_search_applies_acl_before_topk_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_private_stronger_hybrid_cell(&mut db);

    assert_hybrid_limit_one_returns_public_cell(&db);
}

#[test]
fn database_hybrid_search_applies_acl_before_topk_persisted() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_private_stronger_hybrid_cell(&mut db);
    db.checkpoint().unwrap();

    assert_hybrid_limit_one_returns_public_cell(&db);
}

#[test]
fn database_keyword_search_survives_checkpoint_restart() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(
            CellId(u64::MAX),
            b"scope=project:investments\nstatus=ready\nlarge budget".to_vec(),
        )
        .unwrap();
        db.checkpoint().unwrap();
    }

    let db = Database::open(dir.path()).unwrap();
    let results = db
        .search_keyword("budget", &view("project:investments"), SearchLimit(10))
        .unwrap();
    assert_eq!(results[0].cell_id, CellId(u64::MAX));
}

#[test]
fn database_keyword_search_reads_persisted_aci_without_wal_tail_changes() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\n\nalpha budget approved".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=tenant:private\nstatus=ready\n\nalpha budget hidden".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();

    let results = db
        .search_keyword("budget", &view("project:investments"), SearchLimit(10))
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].cell_id, CellId(1));
}

#[test]
fn database_keyword_search_falls_back_to_snapshot_for_uncheckpointed_changes() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\n\nold term".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();
    db.patch_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\n\nfresh budget".to_vec(),
    )
    .unwrap();

    let results = db
        .search_keyword("budget", &view("project:investments"), SearchLimit(10))
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].cell_id, CellId(1));
    assert!(String::from_utf8_lossy(&results[0].payload).contains("fresh budget"));
}

#[test]
fn database_vector_search_reads_persisted_acv_without_wal_tail_changes() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nvector=5,0\n\nalpha".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=tenant:private\nstatus=ready\nvector=9,0\n\nhidden".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();

    let results = db
        .search_vector(&[2, 0], &view("project:investments"), SearchLimit(10))
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].cell_id, CellId(1));
    assert!(results[0].vector_score > 0);
}

#[test]
fn database_hybrid_search_reads_persisted_aci_and_acv_without_snapshot_rebuild() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(
            CellId(1),
            b"scope=project:investments\nstatus=ready\nvector=1,0,0\n\nbudget investment".to_vec(),
        )
        .unwrap();
        db.put_cell(
            CellId(2),
            b"scope=project:investments\nstatus=ready\nvector=5,0,0\n\nbudget workflow".to_vec(),
        )
        .unwrap();
        db.put_cell(
            CellId(3),
            b"scope=project:investments\nstatus=ready\nvector=9,0,0\n\nunrelated".to_vec(),
        )
        .unwrap();
        db.checkpoint().unwrap();
    }

    let db = Database::open(dir.path()).unwrap();
    let results = db
        .search_cells(
            SearchQuery {
                text: "budget investment",
                vector: Some(&[9, 0, 0]),
                limit: 3,
                mode: SearchMode::Hybrid,
            },
            &view("project:investments"),
        )
        .unwrap();

    assert_eq!(results[0].cell_id, CellId(1));
    assert!(results[0].lexical_score > 0);
    assert!(results[0].vector_score > 0);
    assert!(results.iter().any(|result| result.vector_score > 0));
}

#[test]
fn database_vector_search_falls_back_to_snapshot_for_uncheckpointed_changes() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nvector=0,9\n\nold".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();
    db.patch_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nvector=9,0\n\nfresh".to_vec(),
    )
    .unwrap();

    let results = db
        .search_vector(&[3, 0], &view("project:investments"), SearchLimit(10))
        .unwrap();

    assert_eq!(results[0].cell_id, CellId(1));
    assert!(results[0].vector_score > 0);
    assert!(String::from_utf8_lossy(&results[0].payload).contains("fresh"));
}

#[test]
fn database_keyword_search_uses_body_terms_not_header_terms() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\n\nbody budget".to_vec(),
    )
    .unwrap();

    assert!(db
        .search_keyword("project", &view("project:investments"), SearchLimit(10))
        .unwrap()
        .is_empty());
    assert_eq!(
        db.search_keyword("budget", &view("project:investments"), SearchLimit(10))
            .unwrap()[0]
            .cell_id,
        CellId(1)
    );
}

#[test]
fn checkpoint_lexical_index_persists_doc_lengths() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\n\nalpha budget budget".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();

    let index = LexicalIndex::read(dir.path().join("segments").join("segment-1.aci")).unwrap();
    assert_eq!(index.doc_lengths.get(&1), Some(&3));
    assert_eq!(
        index
            .term_frequencies
            .get("budget")
            .and_then(|values| values.get(&1)),
        Some(&2)
    );
}

#[test]
fn database_keyword_search_uses_persisted_title_weighting() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\ntitle=budget\n\nworkflow note".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\n\nbudget budget".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();

    let results = db
        .search_keyword("budget", &view("project:investments"), SearchLimit(2))
        .unwrap();

    assert_eq!(results[0].cell_id, CellId(1));
    assert!(results[0].lexical_score > results[1].lexical_score);
}

#[test]
fn search_query_understanding_extracts_anchors_without_oracle_metadata() {
    let analyzed = analyze_search_query(
        "Find the GitHub PR #77 for AUTH-456 in src/server/auth.rs before v2.1.0",
    );

    assert!(analyzed.source_hints.contains(&"github".to_owned()));
    assert!(analyzed
        .anchors
        .iter()
        .any(|anchor| anchor.kind == QueryAnchorKind::PullRequest && anchor.text == "#77"));
    assert!(analyzed
        .anchors
        .iter()
        .any(|anchor| anchor.kind == QueryAnchorKind::TicketId && anchor.text == "AUTH-456"));
    assert!(analyzed
        .anchors
        .iter()
        .any(|anchor| anchor.kind == QueryAnchorKind::FilePath));
}

#[test]
fn database_keyword_search_uses_query_expansion_from_question_text() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\n\nrelease dependency assigned to DRI".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\n\nlaunch celebration notes".to_vec(),
    )
    .unwrap();

    let results = db
        .search_keyword(
            "Who owns the blocker?",
            &view("project:investments"),
            SearchLimit(2),
        )
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].cell_id, CellId(1));
    assert!(results[0].lexical_score > 0);
}

#[test]
fn database_search_reranker_can_use_payload_without_bypassing_scope_filter() {
    struct PayloadNeedleReranker(&'static str);

    impl SearchReranker for PayloadNeedleReranker {
        fn rerank_score(&self, input: SearchRerankInput<'_>) -> u64 {
            let payload = input
                .payload
                .map(String::from_utf8_lossy)
                .unwrap_or_default();
            if payload.contains(self.0) {
                input.base_score.saturating_add(10_000_000)
            } else {
                input.base_score
            }
        }
    }

    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\n\nbudget ordinary".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\n\nbudget preferred-answer".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(3),
        b"scope=tenant:private\nstatus=ready\n\nbudget preferred-answer".to_vec(),
    )
    .unwrap();

    let results = db
        .search_cells_with_reranker(
            SearchQuery {
                text: "budget",
                vector: None,
                limit: 2,
                mode: SearchMode::Keyword,
            },
            &view("project:investments"),
            &PayloadNeedleReranker("preferred-answer"),
        )
        .unwrap();

    assert_eq!(results[0].cell_id, CellId(2));
    assert!(!results.iter().any(|result| result.cell_id == CellId(3)));
}

#[test]
fn checkpoint_vector_index_persists_payload_vectors() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nvector=3,4\n\nalpha".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();

    let index = VectorIndex::read(dir.path().join("segments").join("segment-1.acv")).unwrap();
    assert_eq!(index.vectors.get(&1), Some(&vec![3, 4]));
    assert!(!dir.path().join("segments").join("segment-1.ach").exists());
}

fn seed_private_stronger_keyword_cell(db: &mut Database) {
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

fn assert_keyword_limit_one_returns_public_cell(db: &Database) {
    let results = db
        .search_keyword("budget", &view("project:investments"), SearchLimit(1))
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].cell_id, CellId(2));
    let payload = String::from_utf8_lossy(&results[0].payload);
    assert!(payload.contains("approved"));
    assert!(!payload.contains("hidden"));
}

fn seed_private_stronger_vector_cell(db: &mut Database) {
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

fn assert_vector_limit_one_returns_public_cell(db: &Database) {
    let results = db
        .search_vector(&[100, 0], &view("project:investments"), SearchLimit(1))
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].cell_id, CellId(2));
    let payload = String::from_utf8_lossy(&results[0].payload);
    assert!(payload.contains("approved"));
    assert!(!payload.contains("hidden"));
}

fn seed_private_stronger_hybrid_cell(db: &mut Database) {
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

fn assert_hybrid_limit_one_returns_public_cell(db: &Database) {
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
