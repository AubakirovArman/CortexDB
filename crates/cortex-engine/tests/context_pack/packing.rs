use super::common::prelude::*;
use super::common::retrieved;

#[test]
fn context_pack_accounts_for_required_citation_overhead() {
    let payload = "scope=project:investments\nstatus=ready\nsource=doc-a\nalpha budget";
    let pack = ContextPack::from_retrieved_with_options(
        vec![retrieved(1, payload)],
        1_000,
        true,
        &ContextPackOptions::default(),
        "",
    );

    assert_eq!(
        pack.cells[0].estimated_tokens,
        cortex_engine::estimate_tokens(payload.as_bytes()) + DEFAULT_CITATION_OVERHEAD_TOKENS
    );
    assert_eq!(pack.estimated_tokens, pack.cells[0].estimated_tokens);
}

#[test]
fn context_pack_skips_oversized_middle_candidate_and_keeps_later_fit() {
    let huge = "x ".repeat(200);
    let pack = ContextPack::from_retrieved_with_options(
        vec![
            retrieved(1, "small first"),
            retrieved(2, &huge),
            retrieved(3, "tiny"),
        ],
        12,
        false,
        &ContextPackOptions::default(),
        "",
    );

    assert_eq!(
        pack.cells
            .iter()
            .map(|cell| cell.cell_id)
            .collect::<Vec<_>>(),
        vec![CellId(1), CellId(3)]
    );
    assert!(pack.truncated);
    assert_eq!(
        pack.anomalies[0].code,
        ContextPackAnomalyCode::TokenOverload
    );
}

#[test]
fn context_pack_applies_redundancy_before_budget_overload() {
    let huge_duplicate = "alpha budget project ".repeat(100);
    let pack = ContextPack::from_retrieved_with_options(
        vec![
            retrieved(1, "alpha budget project"),
            retrieved(2, &huge_duplicate),
            retrieved(3, "gamma schedule"),
        ],
        16,
        false,
        &ContextPackOptions {
            reduce_redundancy: true,
            redundancy_threshold_q16: 10,
            ..ContextPackOptions::default()
        },
        "",
    );

    assert_eq!(
        pack.cells
            .iter()
            .map(|cell| cell.cell_id)
            .collect::<Vec<_>>(),
        vec![CellId(1), CellId(3)]
    );
    assert!(!pack.truncated);
    assert_eq!(
        pack.anomalies[0].code,
        ContextPackAnomalyCode::RedundantCell
    );
}

#[test]
fn context_pack_mmr_prefers_coverage_over_nearby_duplicate() {
    let alpha = format!("alpha launch owner {}", "detail ".repeat(20));
    let alpha_duplicate = format!("alpha launch adjacent {}", "detail ".repeat(20));
    let gamma = format!("gamma schedule deadline {}", "detail ".repeat(20));
    let pack = ContextPack::from_retrieved_with_options(
        vec![
            retrieved(1, &alpha),
            retrieved(2, &alpha_duplicate),
            retrieved(3, &gamma),
        ],
        96,
        false,
        &ContextPackOptions {
            reduce_redundancy: true,
            redundancy_threshold_q16: u16::MAX,
            ..ContextPackOptions::default()
        },
        r#"RETRIEVE CONTEXT FOR TASK "alpha gamma" IN BRAIN investment_projects;"#,
    );

    assert_eq!(
        pack.cells
            .iter()
            .map(|cell| cell.cell_id)
            .collect::<Vec<_>>(),
        vec![CellId(1), CellId(3)]
    );
}

#[test]
fn context_pack_redundancy_uses_lexical_fallback_when_only_one_cell_has_vector() {
    let pack = ContextPack::from_retrieved_with_options(
        vec![
            retrieved(1, "alpha budget project"),
            retrieved(2, "vector=1,2,3\n\nalpha budget project duplicate"),
        ],
        1_000,
        false,
        &ContextPackOptions {
            reduce_redundancy: true,
            redundancy_threshold_q16: 10,
            ..ContextPackOptions::default()
        },
        r#"RETRIEVE CONTEXT FOR TASK "alpha budget" IN BRAIN investment_projects;"#,
    );

    assert_eq!(pack.cells.len(), 1);
    assert_eq!(pack.cells[0].cell_id, CellId(1));
    assert!(pack
        .anomalies
        .iter()
        .any(|anomaly| anomaly.code == ContextPackAnomalyCode::RedundantCell));
}
