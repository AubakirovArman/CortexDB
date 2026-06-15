use crate::helpers::*;

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
fn database_learned_ranking_is_disabled_by_default_and_opt_in() {
    let default_dir = tempfile::tempdir().unwrap();
    let default_db = Database::open(default_dir.path()).unwrap();
    assert!(!default_db.learned_ranking_options().enabled);

    let tuned_dir = tempfile::tempdir().unwrap();
    let tuned_db = Database::open_with_options(
        tuned_dir.path(),
        DatabaseOptions {
            learned_ranking: LearnedRankingOptions { enabled: true },
            ..DatabaseOptions::default()
        },
    )
    .unwrap();
    assert!(tuned_db.learned_ranking_options().enabled);
}
