use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::search::{
    analyze_search_query, CorpusSynonymOptions, DatabaseSearchResult, QueryAnchorKind, SearchMode,
    SearchQuery, SearchRerankInput, SearchReranker,
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
fn database_vector_search_uses_named_view_vectors_in_live_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nvector=0,100\n\nbody vector only".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\ntitle_vector=100,0\nvector=0,1\n\nview vector"
            .to_vec(),
    )
    .unwrap();

    let results = db
        .search_vector(&[100, 0], &view("project:investments"), SearchLimit(2))
        .unwrap();

    assert_eq!(results[0].cell_id, CellId(2));
    assert!(results[0].vector_score > 0);
}

#[test]
fn database_vector_search_report_explains_winning_view_vector() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nvector=0,100\n\nbody vector only".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\ntitle_vector=100,0\nvector=0,1\n\nview vector"
            .to_vec(),
    )
    .unwrap();

    let outcome = db
        .search_vector_with_report(&[100, 0], &view("project:investments"), SearchLimit(2))
        .unwrap();

    assert_eq!(outcome.results[0].cell_id, CellId(2));
    assert_eq!(outcome.view_traces[0].cell_id, CellId(2));
    assert_eq!(outcome.view_traces[0].candidate_id, 2);
    assert_eq!(outcome.view_traces[0].vector_view.as_deref(), Some("title"));
    assert!(outcome.view_traces[0].vector_score > 0);
}

#[test]
fn database_hybrid_search_uses_named_view_vectors_in_live_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nvector=0,100\n\nbudget body".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\ntitle=budget view\ntitle_vector=100,0\nvector=0,1\n\ncommon".to_vec(),
    )
    .unwrap();

    let results = db
        .search_cells(
            SearchQuery {
                text: "budget",
                vector: Some(&[100, 0]),
                limit: 2,
                mode: SearchMode::Hybrid,
            },
            &view("project:investments"),
        )
        .unwrap();

    assert_eq!(results[0].cell_id, CellId(2));
    assert!(results[0].lexical_score > 0);
    assert!(results[0].vector_score > 0);
}

#[test]
fn database_hybrid_rerank_promotes_anchor_match_in_live_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_hybrid_rerank_cells(&mut db);

    let hybrid = search_hybrid_anchor_question(&db, SearchMode::Hybrid);
    let reranked = search_hybrid_anchor_question(&db, SearchMode::HybridRerank);

    assert_eq!(hybrid[0].cell_id, CellId(1));
    assert_eq!(reranked[0].cell_id, CellId(2));
    assert!(reranked[0].score > hybrid[0].score);
}

#[test]
fn database_hybrid_rerank_promotes_anchor_match_from_persisted_index() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        seed_hybrid_rerank_cells(&mut db);
        db.checkpoint().unwrap();
    }
    let db = Database::open(dir.path()).unwrap();

    let hybrid = search_hybrid_anchor_question(&db, SearchMode::Hybrid);
    let reranked = search_hybrid_anchor_question(&db, SearchMode::HybridRerank);

    assert_eq!(hybrid[0].cell_id, CellId(1));
    assert_eq!(reranked[0].cell_id, CellId(2));
    assert!(reranked[0].score > hybrid[0].score);
}

#[test]
fn database_hybrid_rerank_penalizes_candidate_without_evidence_overlap() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nvector=100,0\n\nGeneral engineering update."
            .to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\nvector=1,0\n\nAUTH-123 was fixed by PR #42."
            .to_vec(),
    )
    .unwrap();

    let results = db
        .search_cells(
            SearchQuery {
                text: "Which PR #42 fixed AUTH-123?",
                vector: Some(&[100, 0]),
                limit: 1,
                mode: SearchMode::HybridRerank,
            },
            &view("project:investments"),
        )
        .unwrap();

    assert_eq!(results[0].cell_id, CellId(2));
}

#[test]
fn database_hybrid_rerank_prefers_trusted_fresh_conflicting_source_live_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_conflicting_policy_sources(&mut db);

    assert_trusted_fresh_policy_source_first(&db);
}

#[test]
fn database_hybrid_rerank_prefers_trusted_fresh_conflicting_source_persisted() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        seed_conflicting_policy_sources(&mut db);
        db.checkpoint().unwrap();
    }
    let db = Database::open(dir.path()).unwrap();

    assert_trusted_fresh_policy_source_first(&db);
}

#[test]
fn database_hybrid_rerank_diversifies_completeness_results() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\n\nproject blocker database migration owner maya risk retry retry"
            .to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\n\nproject blocker database migration owner maya risk retry"
            .to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(3),
        b"scope=project:investments\nstatus=ready\n\nproject blocker security access oauth auth incident"
            .to_vec(),
    )
    .unwrap();

    let results = db
        .search_cells(
            SearchQuery {
                text: "List all project blockers",
                vector: None,
                limit: 2,
                mode: SearchMode::HybridRerank,
            },
            &view("project:investments"),
        )
        .unwrap();

    assert_eq!(results.len(), 2);
    assert!(results.iter().any(|result| result.cell_id == CellId(3)));
}

#[test]
fn database_hybrid_rerank_diversifies_completeness_by_document_cluster() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_cluster_diversity_cells(&mut db);

    let results = db
        .search_cells(
            SearchQuery {
                text: "List all rollout blockers and owners",
                vector: None,
                limit: 2,
                mode: SearchMode::HybridRerank,
            },
            &view("project:investments"),
        )
        .unwrap();

    assert_eq!(results.len(), 2);
    let result_ids = results
        .iter()
        .map(|result| result.cell_id)
        .collect::<BTreeSet<_>>();
    assert!(result_ids.contains(&CellId(3)));
    assert_ne!(result_ids, BTreeSet::from([CellId(1), CellId(2)]));
}

#[test]
fn database_hybrid_rerank_reports_cluster_diversity_diagnostics() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_cluster_diversity_cells(&mut db);

    let outcome = db
        .search_cells_with_report(
            SearchQuery {
                text: "List all rollout blockers and owners",
                vector: None,
                limit: 2,
                mode: SearchMode::HybridRerank,
            },
            &view("project:investments"),
        )
        .unwrap();

    let diagnostics = outcome.diversity_diagnostics.unwrap();
    assert!(diagnostics.diversity_enabled);
    assert_eq!(diagnostics.input_candidates, 3);
    assert_eq!(diagnostics.output_candidates, 2);
    assert_eq!(diagnostics.skipped_candidates, 1);
    assert!(diagnostics.max_cluster_similarity_q16 >= 36_864);
    assert!(diagnostics.selected_with_cluster_similarity >= 1);
}

#[test]
fn database_search_expands_child_hit_with_parent_context_live_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_parent_child_context_cells(&mut db);

    let results = search_child_anchor(&db);

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].cell_id, CellId(2));
    assert_eq!(results[1].cell_id, CellId(1));
    assert!(String::from_utf8_lossy(&results[1].payload).contains("Parent context"));
}

#[test]
fn database_search_expands_child_hit_with_parent_context_from_persisted_index() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        seed_parent_child_context_cells(&mut db);
        db.checkpoint().unwrap();
    }
    let db = Database::open(dir.path()).unwrap();

    let results = search_child_anchor(&db);

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].cell_id, CellId(2));
    assert_eq!(results[1].cell_id, CellId(1));
    assert!(String::from_utf8_lossy(&results[1].payload).contains("Parent context"));
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
fn database_keyword_search_consumes_persisted_corpus_synonyms() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\n\nzephyr quartz rollout".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\n\nzephyr quartz incident".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(3),
        b"scope=project:investments\nstatus=ready\n\nquartz migration note".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(4),
        b"scope=project:investments\nstatus=ready\n\nzephyr rollout status".to_vec(),
    )
    .unwrap();

    let before = db
        .search_keyword("zephyr", &view("project:investments"), SearchLimit(3))
        .unwrap();
    assert!(!before.iter().any(|result| result.cell_id == CellId(3)));

    db.persist_corpus_synonym_dictionary(CorpusSynonymOptions::default())
        .unwrap();

    let snapshot_results = db
        .search_keyword("zephyr", &view("project:investments"), SearchLimit(3))
        .unwrap();
    assert!(snapshot_results
        .iter()
        .any(|result| result.cell_id == CellId(3)));

    db.checkpoint().unwrap();
    let persisted_results = db
        .search_keyword("zephyr", &view("project:investments"), SearchLimit(3))
        .unwrap();
    assert!(persisted_results
        .iter()
        .any(|result| result.cell_id == CellId(3)));
}

#[test]
fn checkpoint_publishes_corpus_synonyms_for_search() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\n\nzephyr quartz rollout".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\n\nzephyr quartz incident".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(3),
        b"scope=project:investments\nstatus=ready\n\nquartz migration note".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(4),
        b"scope=project:investments\nstatus=ready\n\nzephyr rollout status".to_vec(),
    )
    .unwrap();

    assert!(!db.corpus_synonym_dictionary_path().exists());
    db.checkpoint().unwrap();
    assert!(db.corpus_synonym_dictionary_path().exists());

    let results = db
        .search_keyword("zephyr", &view("project:investments"), SearchLimit(4))
        .unwrap();
    assert!(results.iter().any(|result| result.cell_id == CellId(3)));
}

#[test]
fn checkpoint_publishes_abbreviation_synonyms_for_search() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\n\nThe single sign on (SSO) rollout is blocked."
            .to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\n\nSingle sign on migration playbook.".to_vec(),
    )
    .unwrap();

    let before = db
        .search_keyword("SSO", &view("project:investments"), SearchLimit(2))
        .unwrap();
    assert!(!before.iter().any(|result| result.cell_id == CellId(2)));

    db.checkpoint().unwrap();

    let after = db
        .search_keyword("SSO", &view("project:investments"), SearchLimit(2))
        .unwrap();
    assert!(after.iter().any(|result| result.cell_id == CellId(2)));
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
fn database_keyword_search_uses_bidirectional_query_expansion_from_question_text() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\n\nrelease owner is Maya; blocker is dependency on auth"
            .to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\n\nlaunch celebration notes".to_vec(),
    )
    .unwrap();

    let results = db
        .search_keyword(
            "Who is the DRI for the slipped rollout?",
            &view("project:investments"),
            SearchLimit(2),
        )
        .unwrap();

    assert_eq!(results[0].cell_id, CellId(1));
    assert!(results[0].lexical_score > 0);
}

#[test]
fn database_keyword_search_uses_high_level_phrase_expansion_from_question_text() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\ntitle=Company charter\n\nOur mission is to provide enterprise context infrastructure."
            .to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\n\nweekly sprint note".to_vec(),
    )
    .unwrap();

    let results = db
        .search_keyword(
            "Give me the high level company overview",
            &view("project:investments"),
            SearchLimit(2),
        )
        .unwrap();

    assert_eq!(results[0].cell_id, CellId(1));
    assert!(results[0].lexical_score > 0);
}

#[test]
fn database_search_high_level_query_fills_summary_anchor_live_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_high_level_anchor_cells(&mut db);

    let results = search_high_level_anchor(&db);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].cell_id, CellId(1));
    assert!(String::from_utf8_lossy(&results[0].payload).contains("Northstar"));
}

#[test]
fn database_search_high_level_query_fills_summary_anchor_from_persisted_index() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        seed_high_level_anchor_cells(&mut db);
        db.checkpoint().unwrap();
    }
    let db = Database::open(dir.path()).unwrap();

    let results = search_high_level_anchor(&db);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].cell_id, CellId(1));
    assert!(String::from_utf8_lossy(&results[0].payload).contains("Northstar"));
}

#[test]
fn database_search_project_query_adds_same_project_artifacts_live_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    seed_project_artifact_cells(&mut db);

    let results = search_project_launch_owner(&db);

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].cell_id, CellId(1));
    assert!(results.iter().any(|result| result.cell_id == CellId(2)));
    assert!(results.iter().any(|result| result.cell_id == CellId(3)));
    assert!(!results.iter().any(|result| result.cell_id == CellId(4)));
}

#[test]
fn database_search_project_query_adds_same_project_artifacts_from_persisted_index() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        seed_project_artifact_cells(&mut db);
        db.checkpoint().unwrap();
    }
    let db = Database::open(dir.path()).unwrap();

    let results = search_project_launch_owner(&db);

    assert_eq!(results.len(), 3);
    assert_eq!(results[0].cell_id, CellId(1));
    assert!(results.iter().any(|result| result.cell_id == CellId(2)));
    assert!(results.iter().any(|result| result.cell_id == CellId(3)));
    assert!(!results.iter().any(|result| result.cell_id == CellId(4)));
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
fn database_hybrid_rerank_uses_adaptive_result_limit() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    for id in 1..=12 {
        db.put_cell(
            CellId(id),
            format!("scope=project:investments\nstatus=ready\n\ninvoice q4 payment record {id}")
                .into_bytes(),
        )
        .unwrap();
    }
    for id in 13..=24 {
        db.put_cell(
            CellId(id),
            format!("scope=project:investments\nstatus=ready\n\nproject blockers evidence {id}")
                .into_bytes(),
        )
        .unwrap();
    }

    let lookup_results = db
        .search_cells(
            SearchQuery {
                text: "Find invoice Q4",
                vector: None,
                limit: 10,
                mode: SearchMode::HybridRerank,
            },
            &view("project:investments"),
        )
        .unwrap();
    let broad_results = db
        .search_cells(
            SearchQuery {
                text: "List all project blockers",
                vector: None,
                limit: 10,
                mode: SearchMode::HybridRerank,
            },
            &view("project:investments"),
        )
        .unwrap();

    assert_eq!(lookup_results.len(), 5);
    assert_eq!(broad_results.len(), 10);
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

fn seed_hybrid_rerank_cells(db: &mut Database) {
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

fn search_hybrid_anchor_question(db: &Database, mode: SearchMode) -> Vec<DatabaseSearchResult> {
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

fn seed_conflicting_policy_sources(db: &mut Database) {
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

fn seed_cluster_diversity_cells(db: &mut Database) {
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

fn assert_trusted_fresh_policy_source_first(db: &Database) {
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

fn seed_parent_child_context_cells(db: &mut Database) {
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

fn search_child_anchor(db: &Database) -> Vec<DatabaseSearchResult> {
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

fn seed_high_level_anchor_cells(db: &mut Database) {
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

fn search_high_level_anchor(db: &Database) -> Vec<DatabaseSearchResult> {
    db.search_keyword(
        "Give me the big picture",
        &view("project:investments"),
        SearchLimit(2),
    )
    .unwrap()
}

fn seed_project_artifact_cells(db: &mut Database) {
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

fn search_project_launch_owner(db: &Database) -> Vec<DatabaseSearchResult> {
    db.search_keyword(
        "Who owns the launch?",
        &view("project:investments"),
        SearchLimit(3),
    )
    .unwrap()
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
