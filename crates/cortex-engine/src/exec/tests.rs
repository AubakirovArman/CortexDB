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

    assert_eq!(execution.trace.name, "PackOp");
    assert_eq!(execution.trace.input_count, 1);
    assert_eq!(execution.trace.output_count, execution.pack.cells.len());
    assert_eq!(execution.pack.cells.len(), 1);
}
