use std::cmp::Reverse;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use cortex_engine::{SearchRerankInput, SearchReranker, WeightedScoreReranker};
use serde::Deserialize;
use serde_json::json;

const FIXTURE: &str =
    include_str!("../../../fixtures/enterprise_rag_bench/learned_ranking/offline_v1.jsonl");
const MIN_LIFT_BPS: u64 = 2_500;
const MIN_WIN_RATE_PCT: u64 = 75;

#[derive(Debug, Deserialize)]
struct Row {
    split: String,
    question_id: String,
    question_type: String,
    question: String,
    expected_doc_ids: Vec<String>,
    candidates: Vec<Candidate>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    document_id: String,
    base_score: u64,
    lexical_score: u64,
    vector_score: u64,
}

#[test]
fn engine_learned_ranking_reproduces_offline_heldout_lift() {
    let rows = read_rows();
    let heldout = rows
        .iter()
        .filter(|row| row.split == "heldout")
        .collect::<Vec<_>>();
    assert!(!heldout.is_empty(), "fixture must include heldout rows");

    let baseline = WeightedScoreReranker::fixed_default();
    let learned = WeightedScoreReranker::enterprise_rag_calibrated();
    let baseline_mrr_bps = mrr_bps(&heldout, baseline);
    let learned_mrr_bps = mrr_bps(&heldout, learned);
    let lift_bps = learned_mrr_bps.saturating_sub(baseline_mrr_bps);
    let (win_rate_pct, regressions) = win_rate_and_regressions(&heldout, baseline, learned);
    let details = heldout
        .iter()
        .map(|row| {
            json!({
                "question_id": row.question_id,
                "question_type": row.question_type,
                "baseline_ranked_doc_ids": ranked_doc_ids(row, baseline),
                "learned_ranked_doc_ids": ranked_doc_ids(row, learned),
                "expected_doc_ids": row.expected_doc_ids,
            })
        })
        .collect::<Vec<_>>();

    write_report(json!({
        "schema_version": "cortexdb.ranking_learned_lift.v1",
        "status": if lift_bps >= MIN_LIFT_BPS && win_rate_pct >= MIN_WIN_RATE_PCT && regressions.is_empty() {
            "passed"
        } else {
            "failed"
        },
        "heldout_rows": heldout.len(),
        "heldout_baseline_mrr_bps": baseline_mrr_bps,
        "heldout_learned_mrr_bps": learned_mrr_bps,
        "heldout_mrr_lift_bps": lift_bps,
        "heldout_win_rate_pct": win_rate_pct,
        "policy_regressions": regressions,
        "details": details,
    }));

    assert!(
        lift_bps >= MIN_LIFT_BPS,
        "heldout lift {lift_bps} < {MIN_LIFT_BPS}"
    );
    assert!(
        win_rate_pct >= MIN_WIN_RATE_PCT,
        "heldout win-rate {win_rate_pct} < {MIN_WIN_RATE_PCT}"
    );
}

fn read_rows() -> Vec<Row> {
    FIXTURE
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("ranking fixture row must decode"))
        .collect()
}

fn ranked_doc_ids(row: &Row, reranker: WeightedScoreReranker) -> Vec<String> {
    let mut scored = row
        .candidates
        .iter()
        .map(|candidate| {
            (
                candidate.document_id.clone(),
                reranker.rerank_score(SearchRerankInput {
                    query_text: &row.question,
                    query_vector: None,
                    candidate_id: 0,
                    lexical_score: candidate.lexical_score,
                    vector_score: candidate.vector_score,
                    base_score: candidate.base_score,
                    metadata: None,
                    payload: None,
                }),
            )
        })
        .collect::<Vec<_>>();
    scored.sort_by_key(|(document_id, score)| (Reverse(*score), document_id.clone()));
    scored
        .into_iter()
        .map(|(document_id, _)| document_id)
        .collect()
}

fn reciprocal_rank_bps(row: &Row, reranker: WeightedScoreReranker) -> u64 {
    let expected = row
        .expected_doc_ids
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    ranked_doc_ids(row, reranker)
        .iter()
        .position(|document_id| expected.contains(document_id))
        .map(|index| 10_000 / u64::try_from(index + 1).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

fn mrr_bps(rows: &[&Row], reranker: WeightedScoreReranker) -> u64 {
    let total = rows
        .iter()
        .map(|row| reciprocal_rank_bps(row, reranker))
        .sum::<u64>();
    (total + u64::try_from(rows.len() / 2).unwrap_or(0)) / u64::try_from(rows.len()).unwrap_or(1)
}

fn win_rate_and_regressions(
    rows: &[&Row],
    baseline: WeightedScoreReranker,
    learned: WeightedScoreReranker,
) -> (u64, Vec<String>) {
    let mut wins = 0u64;
    let mut regressions = Vec::new();
    for row in rows {
        let baseline_rr = reciprocal_rank_bps(row, baseline);
        let learned_rr = reciprocal_rank_bps(row, learned);
        if learned_rr > baseline_rr {
            wins += 1;
        }
        if learned_rr < baseline_rr {
            regressions.push(row.question_id.clone());
        }
    }
    let row_count = u64::try_from(rows.len()).unwrap_or(1);
    ((wins * 100 + row_count / 2) / row_count, regressions)
}

fn write_report(report: serde_json::Value) {
    let Some(path) = std::env::var_os("CORTEX_RANKING_LEARNED_LIFT_REPORT") else {
        return;
    };
    let path = PathBuf::from(path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create learned-lift report parent");
    }
    fs::write(
        path,
        serde_json::to_string_pretty(&report).expect("serialize learned-lift report") + "\n",
    )
    .expect("write learned-lift report");
}
