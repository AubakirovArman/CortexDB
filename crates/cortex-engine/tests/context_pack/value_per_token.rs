use super::common::prelude::*;
use super::common::retrieved;

#[test]
fn value_per_token_optimizer_improves_budget_allocation() {
    let large_low_yield = format!(
        "scope=project:investments\nstatus=ready\nsource=large\n\nalpha {}",
        "filler ".repeat(220)
    );
    let cells = vec![
        retrieved(1, &large_low_yield),
        retrieved(
            2,
            "scope=project:investments\nstatus=ready\nsource=small-beta\n\nbeta mitigation",
        ),
        retrieved(
            3,
            "scope=project:investments\nstatus=ready\nsource=small-gamma\n\ngamma owner",
        ),
    ];
    let query = r#"RETRIEVE CONTEXT FOR TASK "alpha beta gamma" IN BRAIN investment_projects;"#;

    let baseline = ContextPack::from_retrieved_with_options(
        cells.clone(),
        64,
        false,
        &ContextPackOptions::default(),
        query,
    );
    let optimized = ContextPack::from_retrieved_with_options(
        cells,
        64,
        false,
        &ContextPackOptions {
            optimize_value_per_token: true,
            ..ContextPackOptions::default()
        },
        query,
    );

    assert_eq!(baseline.cells[0].cell_id, CellId(1));
    assert!(baseline.estimated_tokens > baseline.token_budget_tokens);
    assert_eq!(
        optimized
            .cells
            .iter()
            .map(|cell| cell.cell_id)
            .collect::<Vec<_>>(),
        vec![CellId(2), CellId(3)]
    );
    assert!(optimized.estimated_tokens <= optimized.token_budget_tokens);
    assert!(optimized.answerability_q16 > baseline.answerability_q16);
}
