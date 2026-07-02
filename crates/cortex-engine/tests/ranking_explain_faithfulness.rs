use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use cortex_core::CellId;
use cortex_engine::determinism_hash::frozen_ranking_weights_identity;
use cortex_engine::{
    ContextExplain, ContextPack, ContextPackOptions, ContextScoreComponent, RetrievedCell,
};
use serde_json::{json, Value};

#[test]
fn context_pack_explain_components_sum_to_final_score_under_frozen_weights() {
    let feedback_scores = BTreeMap::from([(CellId(11), 7), (CellId(12), -5)]);
    let pack = ContextPack::from_retrieved_with_feedback_options(
        vec![
            retrieved(
                11,
                "scope=project:investments\nstatus=ready\nsource_trust_q16=60000\ncreated_unix_seconds=100\n\nSolar budget delay risk mitigation plan for customer escalation.",
            ),
            retrieved(
                12,
                "scope=project:investments\nstatus=ready\nsource_trust_class=internal\ncreated_unix_seconds=200\n\nSolar budget delay risk mitigation plan adds evidence for customer escalation.",
            ),
        ],
        2_000,
        false,
        &ContextPackOptions::default(),
        "RETRIEVE CONTEXT FOR TASK \"Solar budget delay risk mitigation\" IN BRAIN investment_projects;",
        &feedback_scores,
    );

    assert_eq!(pack.cells.len(), 2);
    let mut cell_reports = Vec::new();
    for cell in &pack.cells {
        let explain = cell.explain.as_ref().expect("selected cell has explain");
        let expected_feedback = *feedback_scores.get(&cell.cell_id).unwrap_or(&0);
        assert_explain_score_is_faithful(cell.cell_id, explain, expected_feedback);
        cell_reports.push(cell_report(cell.cell_id, explain));
    }
    assert!(
        pack.cells
            .iter()
            .filter_map(|cell| cell.explain.as_ref())
            .any(|explain| explain.redundancy_penalty > 0),
        "fixture must exercise the frozen redundancy penalty weight",
    );

    let identity = frozen_ranking_weights_identity();
    write_report(json!({
        "schema_version": "cortexdb.ranking_explain_faithfulness.report.v1",
        "status": "passed",
        "frozen_weights_version": identity.version,
        "frozen_weights_artifact_hash": identity.artifact_hash,
        "checked_cells": cell_reports.len(),
        "cell_reports": cell_reports,
    }));
}

fn assert_explain_score_is_faithful(
    cell_id: CellId,
    explain: &ContextExplain,
    expected_feedback: i32,
) {
    assert_component(
        cell_id,
        &explain.score_components,
        "base_bm25",
        explain.base_bm25,
        i64::from(explain.base_bm25),
    );
    assert_component(
        cell_id,
        &explain.score_components,
        "source_trust_bonus",
        explain.source_trust_bonus,
        i64::from(explain.source_trust_bonus),
    );
    assert_component(
        cell_id,
        &explain.score_components,
        "source_freshness_bonus",
        explain.source_freshness_bonus,
        i64::from(explain.source_freshness_bonus),
    );
    assert_component(
        cell_id,
        &explain.score_components,
        "redundancy_penalty",
        explain.redundancy_penalty,
        -i64::from(explain.redundancy_penalty),
    );

    let feedback_contribution = component_contribution(&explain.score_components, "feedback_bonus");
    assert_eq!(
        feedback_contribution, expected_feedback,
        "cell {} feedback component must match the feedback score",
        cell_id.0
    );

    let component_sum = explain
        .score_components
        .iter()
        .map(|component| i64::from(component.contribution))
        .sum::<i64>();
    assert!(
        component_sum >= 0,
        "cell {} explain components summed below zero",
        cell_id.0
    );
    assert_eq!(
        u32::try_from(component_sum).expect("component sum must fit u32"),
        explain.score,
        "cell {} explain score must equal sum(score_components.contribution)",
        cell_id.0
    );
}

fn assert_component(
    cell_id: CellId,
    components: &[ContextScoreComponent],
    name: &str,
    expected_value: u32,
    expected_contribution: i64,
) {
    let component = components
        .iter()
        .find(|component| component.name == name)
        .unwrap_or_else(|| panic!("cell {} missing component {name}", cell_id.0));
    assert_eq!(
        component.value, expected_value,
        "cell {} component {name} value drifted",
        cell_id.0
    );
    assert_eq!(
        i64::from(component.contribution),
        expected_contribution,
        "cell {} component {name} contribution drifted",
        cell_id.0
    );
}

fn component_contribution(components: &[ContextScoreComponent], name: &str) -> i32 {
    components
        .iter()
        .filter(|component| component.name == name)
        .map(|component| component.contribution)
        .sum()
}

fn cell_report(cell_id: CellId, explain: &ContextExplain) -> Value {
    json!({
        "cell_id": cell_id.0,
        "score": explain.score,
        "component_sum": explain.score_components.iter()
            .map(|component| i64::from(component.contribution))
            .sum::<i64>(),
        "components": explain.score_components.iter().map(|component| {
            json!({
                "name": component.name,
                "value": component.value,
                "contribution": component.contribution,
            })
        }).collect::<Vec<_>>(),
    })
}

fn retrieved(cell_id: u64, payload: &str) -> RetrievedCell {
    RetrievedCell::from_payload(CellId(cell_id), payload.as_bytes().to_vec())
}

fn write_report(report: Value) {
    let Some(path) = std::env::var_os("CORTEX_RANKING_EXPLAIN_FAITHFULNESS_REPORT") else {
        return;
    };
    let path = PathBuf::from(path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create ranking explain faithfulness report parent");
    }
    fs::write(
        path,
        serde_json::to_string_pretty(&report)
            .expect("serialize ranking explain faithfulness report")
            + "\n",
    )
    .expect("write ranking explain faithfulness report");
}
