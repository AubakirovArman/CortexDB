use super::support::*;

#[test]
fn source_payload_prefilter_reads_source_docs_without_persisted_index() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!("cortexdb-source-prefilter-{unique}"));
    let sources = root.join("sources");
    fs::create_dir_all(sources.join("slack")).expect("create source dir");
    fs::write(
        sources.join("slack/doc-a.json"),
        json!({
            "title_field_name": "title",
            "content_field_names": ["body"],
            "title": "Alpha rollout",
            "body": "The alpha rollout mentions cache warmup and deployment status."
        })
        .to_string(),
    )
    .expect("write doc-a");
    fs::write(
        sources.join("slack/doc-b.json"),
        json!({
            "title_field_name": "title",
            "content_field_names": ["body"],
            "title": "Beta rollout",
            "body": "The beta rollout mentions source-payload prefiltering."
        })
        .to_string(),
    )
    .expect("write doc-b");
    let uuid_index = BTreeMap::from([
        ("doc-a".to_owned(), "slack/doc-a.json".to_owned()),
        ("doc-b".to_owned(), "slack/doc-b.json".to_owned()),
    ]);
    let external = ExternalPrefilterRetrieval {
        by_question_id: BTreeMap::from([(
            "q1".to_owned(),
            vec!["doc-b".to_owned(), "doc-a".to_owned()],
        )]),
        rows: 1,
    };
    let mut source_payloads =
        SourcePayloadPrefilter::new(&uuid_index, sources).expect("source prefilter");
    let mut document_vectors = DocumentVectorLookup::empty();
    let output = source_prefilter_payloads(
        &mut source_payloads,
        &mut document_vectors,
        &external,
        "q1",
        SearchQuery {
            text: "Which rollout mentions prefiltering?",
            vector: None,
            limit: 2,
            mode: SearchMode::HybridRerank,
        },
        2,
        None,
    )
    .expect("source prefilter output");
    let ids = output
        .payloads
        .iter()
        .filter_map(|payload| doc_id_from_payload(payload))
        .collect::<Vec<_>>();

    assert_eq!(ids, vec!["doc-b".to_owned(), "doc-a".to_owned()]);
    assert!(output.diversity_diagnostics.is_some());
    fs::remove_dir_all(root).ok();
}

#[test]
fn prefilter_merge_preserves_lexical_head_then_promotes_reranked_candidates() {
    fn candidate(id: u64) -> PrefilterCandidate {
        PrefilterCandidate {
            cell_id: CellId(id),
            payload: format!("doc_id=doc-{id}\n\nbody").into_bytes(),
            score: id,
            lexical_score: id,
            vector_score: id,
            evidence_score: 0,
        }
    }
    let lexical = (1..=10).map(candidate).collect::<Vec<_>>();
    let reranked = [3, 20, 21, 22, 23, 24, 25, 26]
        .into_iter()
        .map(candidate)
        .collect::<Vec<_>>();

    let merged = merge_prefilter_candidates("Find invoice Q4", lexical, reranked, 10)
        .into_iter()
        .map(|candidate| candidate.cell_id.0)
        .collect::<Vec<_>>();

    assert_eq!(merged, vec![1, 2, 3, 4, 20, 21]);
}

#[test]
fn prefilter_merge_keeps_strong_evidence_tail_candidates() {
    fn candidate(id: u64, evidence_score: u32) -> PrefilterCandidate {
        PrefilterCandidate {
            cell_id: CellId(id),
            payload: format!("doc_id=doc-{id}\n\nbody").into_bytes(),
            score: id,
            lexical_score: id,
            vector_score: id,
            evidence_score,
        }
    }
    let lexical = (1..=10).map(|id| candidate(id, 0)).collect::<Vec<_>>();
    let reranked = [
        candidate(20, 0),
        candidate(21, 0),
        candidate(22, 0),
        candidate(23, 0),
        candidate(24, 0),
        candidate(25, 0),
        candidate(26, ENGINE_PREFILTER_STRONG_EVIDENCE_SCORE),
        candidate(27, ENGINE_PREFILTER_STRONG_EVIDENCE_SCORE + 1),
    ];

    let merged = merge_prefilter_candidates("Find invoice Q4", lexical, reranked.to_vec(), 10)
        .into_iter()
        .map(|candidate| candidate.cell_id.0)
        .collect::<Vec<_>>();

    assert_eq!(merged, vec![1, 2, 3, 4, 20, 21, 26, 27]);
}

#[test]
fn prefilter_hybrid_rerank_diversifies_metadata_clusters() {
    fn candidate(id: u64, score: u64, doc_id: &str, body: &str) -> PrefilterCandidate {
        PrefilterCandidate {
            cell_id: CellId(id),
            payload: format!("doc_id={doc_id}\n\n{body}").into_bytes(),
            score,
            lexical_score: score,
            vector_score: 0,
            evidence_score: 0,
        }
    }
    let candidates = vec![
        candidate(1, 100, "doc-a", "billing blocker owner deadline"),
        candidate(2, 99, "doc-a", "billing blocker owner deadline duplicate"),
        candidate(3, 80, "doc-b", "security blocker status dependency"),
    ];

    let selection = select_diverse_prefilter_candidates(
        candidates.clone(),
        candidates,
        "List all support, security, and billing requirements",
        2,
    );
    let selected = selection
        .candidates
        .iter()
        .map(|candidate| candidate.cell_id.0)
        .collect::<Vec<_>>();

    assert!(selection.diagnostics.diversity_enabled);
    assert_eq!(selection.diagnostics.input_candidates, 3);
    assert_eq!(selection.diagnostics.output_candidates, 2);
    assert_eq!(selection.diagnostics.skipped_candidates, 1);
    assert_eq!(selected, vec![1, 3]);
    assert!(selection.diagnostics.max_cluster_similarity_q16 > 0);
}

#[test]
fn prefilter_diversity_gate_preserves_requested_top_k_for_lookup_rows() {
    fn candidate(id: u64) -> PrefilterCandidate {
        PrefilterCandidate {
            cell_id: CellId(id),
            payload: format!("doc_id=doc-{id}\n\nlookup evidence body").into_bytes(),
            score: 100 - id,
            lexical_score: 100 - id,
            vector_score: 0,
            evidence_score: 0,
        }
    }
    let candidates = (1..=10).map(candidate).collect::<Vec<_>>();

    let selection =
        select_diverse_prefilter_candidates(candidates.clone(), candidates, "Find invoice Q4", 10);

    assert!(!selection.diagnostics.diversity_enabled);
    assert_eq!(selection.candidates.len(), 10);
    assert_eq!(selection.diagnostics.output_candidates, 10);
}

#[test]
fn prefilter_tail_limit_preserves_multi_evidence_queries_without_oracle_labels() {
    assert_eq!(prefilter_default_doc_limit("Find invoice Q4", 10), 6);
    assert_eq!(
        prefilter_default_doc_limit(
            "What caused the 429 spike and how do we verify it is not burning SLOs?",
            10,
        ),
        10
    );
    assert_eq!(
        prefilter_default_doc_limit("List all Fireflies calls mentioning data residency", 10),
        10
    );
    assert_eq!(
        prefilter_default_doc_limit("What is the temporary kill switch name?", 10),
        7
    );
}

#[test]
fn prefilter_lexical_head_preserves_clean_external_order_for_complex_queries() {
    assert_eq!(prefilter_lexical_head_count("Find invoice Q4", 10), 4);
    assert_eq!(
        prefilter_lexical_head_count(
            "What caused the 429 spike and how do we verify it is not burning SLOs?",
            10,
        ),
        10
    );
    assert_eq!(
        prefilter_lexical_head_count("List all Fireflies calls mentioning data residency", 10),
        10
    );
}

#[test]
fn prefilter_evidence_score_rewards_anchors_conditions_and_terms() {
    let score = prefilter_evidence_score(
        "Which PR #42 fixed AUTH-123 before 2026-05-01?",
        b"source=github\n\nAUTH-123 was fixed by PR #42 on 2026-04-30.",
    );

    assert!(score >= ENGINE_PREFILTER_STRONG_EVIDENCE_SCORE);
}
