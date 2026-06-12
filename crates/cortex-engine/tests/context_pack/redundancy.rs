use super::common::prelude::*;
use super::common::retrieved;

#[test]
fn context_pack_can_reduce_sparse_redundancy() {
    let cells = vec![
        retrieved(1, "alpha budget project"),
        retrieved(2, "alpha budget project duplicate"),
        retrieved(3, "gamma schedule"),
    ];
    let pack = ContextPack::from_retrieved_with_options(
        cells,
        1_000,
        false,
        &ContextPackOptions {
            reduce_redundancy: true,
            redundancy_threshold_q16: 32_768,
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
    assert_eq!(
        pack.anomalies[0].code,
        ContextPackAnomalyCode::RedundantCell
    );
    assert_eq!(pack.anomalies[0].cell_id, Some(CellId(2)));
    assert!(pack.anomalies[0]
        .why_excluded
        .as_deref()
        .unwrap_or_default()
        .contains("reduce_redundancy"));
}

#[test]
fn context_pack_can_reduce_dense_vector_redundancy() {
    let cells = vec![
        retrieved(1, "scope=project\nvector=1, 2, 3\nfirst cell"),
        retrieved(2, "scope=project\nvector=1, 2, 4\nsecond cell"),
        retrieved(3, "scope=project\nvector=-1, -2, -3\nthird cell"),
    ];
    let pack = ContextPack::from_retrieved_with_options(
        cells,
        1_000,
        false,
        &ContextPackOptions {
            reduce_redundancy: true,
            redundancy_threshold_q16: 32_768,
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
    assert_eq!(
        pack.anomalies[0].code,
        ContextPackAnomalyCode::RedundantCell
    );
    assert_eq!(pack.anomalies[0].cell_id, Some(CellId(2)));
    assert!(pack.anomalies[0]
        .why_excluded
        .as_deref()
        .unwrap_or_default()
        .contains("reduce_redundancy"));
}

#[test]
fn context_pack_keeps_redundant_cells_by_default() {
    let cells = vec![
        retrieved(1, "alpha budget project"),
        retrieved(2, "alpha budget project duplicate"),
    ];
    let pack = ContextPack::from_retrieved(cells, 1_000, false);

    assert_eq!(pack.cells.len(), 2);
    assert!(pack.anomalies.is_empty());
}
