use super::trace::{drain, MaterializedOp};
use super::*;
use crate::context::ContextPackOptions;
use crate::database::RetrievedCell;
use cortex_core::CellId;

#[test]
fn materialized_operator_reports_available_output() {
    let mut op = MaterializedOp::new("RankOp", 3, vec![1, 2], 7);

    assert_eq!(drain(&mut op), vec![1, 2]);
    assert_eq!(
        op.trace(),
        PhysicalOperatorTrace {
            name: "RankOp".to_owned(),
            input_count: 3,
            output_count: 2,
            elapsed_nanos: 7
        }
    );
}

#[test]
fn explain_collector_tracks_operator_output_counts() {
    let mut collector = ExplainCollector::default();
    assert_eq!(collector.last_output_count(), 0);

    collector.push(PhysicalOperatorTrace {
        name: "BitmapIndexScan".to_owned(),
        input_count: 0,
        output_count: 2,
        elapsed_nanos: 5,
    });

    assert_eq!(collector.last_output_count(), 2);
    assert_eq!(collector.into_traces().len(), 1);
}

#[test]
fn pack_operator_reports_input_and_selected_cells() {
    let cells = vec![RetrievedCell::from_payload(
        CellId(1),
        b"source=doc-a\n\nalpha evidence".to_vec(),
    )];
    let options = ContextPackOptions::default();
    let feedback_scores = std::collections::BTreeMap::new();
    let mut op = PackOp::new(
        cells,
        1_000,
        false,
        &options,
        "alpha",
        &feedback_scores,
        None,
    );
    let Some(pack) = op.next() else {
        panic!("PackOp should emit one ContextPack");
    };

    assert!(op.next().is_none());
    assert_eq!(op.trace().name, "PackOp");
    assert_eq!(op.trace().input_count, 1);
    assert_eq!(op.trace().output_count, pack.cells.len());
    assert_eq!(pack.cells.len(), 1);
}

#[test]
fn pack_operator_matches_context_pack_constructor() {
    let direct = crate::context::ContextPack::from_retrieved_with_feedback_options_and_view(
        vec![RetrievedCell::from_payload(
            CellId(1),
            b"source=doc-a\n\nalpha evidence".to_vec(),
        )],
        1_000,
        false,
        &ContextPackOptions::default(),
        "alpha",
        &std::collections::BTreeMap::new(),
        None,
    );
    let execution = PackOp::execute(
        vec![RetrievedCell::from_payload(
            CellId(1),
            b"source=doc-a\n\nalpha evidence".to_vec(),
        )],
        1_000,
        false,
        &ContextPackOptions::default(),
        "alpha",
        &std::collections::BTreeMap::new(),
        None,
    );

    assert_eq!(execution.pack, direct);
    assert_eq!(execution.trace.output_count, direct.cells.len());
}

#[test]
fn pack_operator_reports_budget_filled_signal() {
    let execution = PackOp::execute(
        vec![RetrievedCell::from_payload(
            CellId(1),
            b"source=doc-a\n\nalpha evidence with enough body text to exceed a tiny budget"
                .to_vec(),
        )],
        4,
        false,
        &ContextPackOptions::default(),
        "alpha",
        &std::collections::BTreeMap::new(),
        None,
    );

    assert!(execution.budget_filled);
}

#[test]
fn pack_operator_budget_signal_stays_false_when_room_remains() {
    let execution = PackOp::execute(
        vec![RetrievedCell::from_payload(
            CellId(1),
            b"source=doc-a\n\nalpha evidence".to_vec(),
        )],
        1_000,
        false,
        &ContextPackOptions::default(),
        "alpha",
        &std::collections::BTreeMap::new(),
        None,
    );

    assert!(!execution.budget_filled);
}
