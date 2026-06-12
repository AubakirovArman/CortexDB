use crate::query::CellMetadata;

use super::scoring::evidence_overlap_score;
use super::{
    calibrated_hybrid_rrf_weights, rerank_calibration_profile, SearchRerankInput, SearchReranker,
    WeightedScoreReranker,
};

#[test]
fn weighted_reranker_rewards_anchor_payload_matches() {
    let reranker = WeightedScoreReranker::default();
    let matched = reranker.rerank_score(SearchRerankInput {
        query_text: "Which PR #42 fixed AUTH-123?",
        query_vector: None,
        candidate_id: 1,
        lexical_score: 1,
        vector_score: 0,
        base_score: 1,
        metadata: None,
        payload: Some(b"scope=project\n\nAUTH-123 was fixed by PR #42."),
    });
    let unmatched = reranker.rerank_score(SearchRerankInput {
        query_text: "Which PR #42 fixed AUTH-123?",
        query_vector: None,
        candidate_id: 2,
        lexical_score: 1,
        vector_score: 0,
        base_score: 1,
        metadata: None,
        payload: Some(b"scope=project\n\nGeneral engineering update."),
    });

    assert!(matched > unmatched);
}

#[test]
fn weighted_reranker_penalizes_candidates_without_evidence_overlap() {
    let reranker = WeightedScoreReranker::default();
    let matched = reranker.rerank_score(SearchRerankInput {
        query_text: "Which PR #42 fixed AUTH-123?",
        query_vector: None,
        candidate_id: 1,
        lexical_score: 0,
        vector_score: 10_000,
        base_score: 10_000,
        metadata: None,
        payload: Some(b"AUTH-123 was fixed by PR #42."),
    });
    let unmatched = reranker.rerank_score(SearchRerankInput {
        query_text: "Which PR #42 fixed AUTH-123?",
        query_vector: None,
        candidate_id: 2,
        lexical_score: 0,
        vector_score: 10_000,
        base_score: 10_000,
        metadata: None,
        payload: Some(b"General engineering update."),
    });

    assert!(matched > unmatched);
    assert!(unmatched < 10_000);
}

#[test]
fn evidence_overlap_requires_more_than_one_broad_query_term() {
    let weak = evidence_overlap_score(
        "What are the upload size limits for multipart requests?",
        b"Upload planning notes only.",
    );
    let strong = evidence_overlap_score(
        "What are the upload size limits for multipart requests?",
        b"Multipart upload requests have size limits.",
    );
    let anchored = evidence_overlap_score(
        "Which PR #42 fixed AUTH-123?",
        b"AUTH-123 was fixed by PR #42.",
    );

    assert_eq!(weak, 1);
    assert!(strong >= 2);
    assert!(anchored >= 2);
}

#[test]
fn weighted_reranker_rewards_sub_requirement_coverage() {
    let reranker = WeightedScoreReranker::default();
    let matched = reranker.rerank_score(SearchRerankInput {
        query_text: "Who owns the Apollo launch blocker and what is the deadline?",
        query_vector: None,
        candidate_id: 1,
        lexical_score: 0,
        vector_score: 0,
        base_score: 1,
        metadata: None,
        payload: Some(b"Apollo owner Maya. Launch blocker is auth. Deadline is 2026-05-01."),
    });
    let weak = reranker.rerank_score(SearchRerankInput {
        query_text: "Who owns the Apollo launch blocker and what is the deadline?",
        query_vector: None,
        candidate_id: 2,
        lexical_score: 0,
        vector_score: 0,
        base_score: 1,
        metadata: None,
        payload: Some(b"Launch celebration notes."),
    });

    assert!(matched > weak);
}

#[test]
fn weighted_reranker_does_not_use_payload_scope_mapping_without_descriptor_metadata() {
    let reranker = WeightedScoreReranker::default();
    let matched = reranker.rerank_score(SearchRerankInput {
        query_text: "What did Slack say about the Apollo rollout?",
        query_vector: None,
        candidate_id: 1,
        lexical_score: 0,
        vector_score: 0,
        base_score: 1,
        metadata: None,
        payload: Some(b"source=slack\nproject=Apollo\n\nSlack Apollo rollout update."),
    });
    let weak = reranker.rerank_score(SearchRerankInput {
        query_text: "What did Slack say about the Apollo rollout?",
        query_vector: None,
        candidate_id: 2,
        lexical_score: 0,
        vector_score: 0,
        base_score: 1,
        metadata: None,
        payload: Some(b"source=gmail\nproject=Hermes\n\nSlack Apollo rollout update."),
    });

    assert_eq!(matched, weak);
}

#[test]
fn weighted_reranker_uses_descriptor_metadata_for_scope_mapping() {
    let reranker = WeightedScoreReranker::default();
    let descriptor_metadata =
        CellMetadata::from_payload(b"source=jira\nproject=Apollo\n\nLaunch update.");
    let spoofed_payload = b"source=gmail\nproject=Hermes\n\nJira Apollo rollout update.";
    let matched = reranker.rerank_score(SearchRerankInput {
        query_text: "What did Jira say about the Apollo rollout?",
        query_vector: None,
        candidate_id: 1,
        lexical_score: 0,
        vector_score: 0,
        base_score: 1,
        metadata: Some(&descriptor_metadata),
        payload: Some(spoofed_payload),
    });
    let weak = reranker.rerank_score(SearchRerankInput {
        query_text: "What did Jira say about the Apollo rollout?",
        query_vector: None,
        candidate_id: 2,
        lexical_score: 0,
        vector_score: 0,
        base_score: 1,
        metadata: None,
        payload: Some(spoofed_payload),
    });

    assert!(matched > weak);
}

#[test]
fn weighted_reranker_rewards_numeric_condition_matches() {
    let reranker = WeightedScoreReranker::default();
    let matched = reranker.rerank_score(SearchRerankInput {
        query_text: "What p95 latency threshold must be under 200 ms?",
        query_vector: None,
        candidate_id: 1,
        lexical_score: 0,
        vector_score: 0,
        base_score: 1,
        metadata: None,
        payload: Some(b"p95 latency threshold is 180 ms for the EU route."),
    });
    let weak = reranker.rerank_score(SearchRerankInput {
        query_text: "What p95 latency threshold must be under 200 ms?",
        query_vector: None,
        candidate_id: 2,
        lexical_score: 0,
        vector_score: 0,
        base_score: 1,
        metadata: None,
        payload: Some(b"p95 latency threshold is 280 ms for the EU route."),
    });

    assert!(matched > weak);
}

#[test]
fn default_reranker_is_not_enterprise_rag_calibrated() {
    let default = WeightedScoreReranker::default();
    let calibrated = WeightedScoreReranker::enterprise_rag_calibrated()
        .calibrated_for_query("Which approach is recommended for delayed adoption?");

    assert!(!default.calibrate_by_question_type);
    assert_ne!(default.vector_weight, calibrated.vector_weight);
    assert!(calibrated.vector_weight > calibrated.lexical_weight);
}

#[test]
fn calibration_profiles_are_selected_from_question_text() {
    let semantic = rerank_calibration_profile(
        "Which approach is recommended for delayed enterprise rollout adoption?",
        WeightedScoreReranker::default(),
    );
    let constrained = rerank_calibration_profile(
        "Which incident where p95 latency threshold was under 200 ms?",
        WeightedScoreReranker::default(),
    );

    assert!(semantic.reranker.vector_weight > semantic.reranker.lexical_weight);
    assert!(semantic.rrf_weights.vector_q16 > semantic.rrf_weights.lexical_q16);
    assert!(constrained.reranker.lexical_weight > constrained.reranker.vector_weight);
    assert!(constrained.reranker.condition_payload_bonus > 1);
}

#[test]
fn calibration_promotes_complex_explanatory_basic_queries_to_vector_heavy_profile() {
    let profile = rerank_calibration_profile(
        "In our GPU inference runtime, what change was introduced to cut the worst-case temporary device-memory spike when short and long requests are interleaved?",
        WeightedScoreReranker::default(),
    );

    assert_eq!(profile.question_type.as_str(), "semantic");
    assert!(profile.rrf_weights.vector_q16 > profile.rrf_weights.lexical_q16);
}

#[test]
fn calibrated_rrf_weights_do_not_use_one_fixed_profile() {
    let semantic =
        calibrated_hybrid_rrf_weights("Which approach is recommended for delayed adoption?");
    let basic = calibrated_hybrid_rrf_weights("What are the default billing migration values?");

    assert_ne!(semantic, basic);
    assert!(semantic.vector_q16 > semantic.lexical_q16);
    assert!(basic.lexical_q16 > basic.vector_q16);
}
