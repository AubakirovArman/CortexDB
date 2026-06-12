use super::common::prelude::*;
use super::common::retrieved;

#[test]
fn context_pack_span_level_packing_selects_relevant_window_under_budget() {
    let payload = format!(
        "scope=project:investments\nstatus=ready\nsource=doc-a\ntitle=Project Apollo\n\n{}\nApollo blocker: database migration is owned by Maya and due Friday.\n{}",
        "intro background ".repeat(120),
        "appendix notes ".repeat(120)
    );
    let pack = ContextPack::from_retrieved_with_options(
        vec![retrieved(1, &payload)],
        128,
        false,
        &ContextPackOptions {
            span_level_packing: true,
            span_context_lines: 0,
            ..ContextPackOptions::default()
        },
        r#"RETRIEVE CONTEXT FOR TASK "Apollo database migration blocker" IN BRAIN investment_projects;"#,
    );

    assert_eq!(pack.cells.len(), 1);
    assert!(pack.truncated);
    assert!(pack.estimated_tokens <= pack.token_budget_tokens);
    let packed = String::from_utf8_lossy(&pack.cells[0].payload);
    assert!(packed.contains("Apollo blocker: database migration"));
    assert!(packed.contains("[context_pack_span=true"));
    assert!(!packed.contains("intro background intro background intro background"));
    assert!(pack.anomalies.iter().any(|anomaly| {
        anomaly
            .why_excluded
            .as_deref()
            .unwrap_or_default()
            .contains("span_level_packing")
    }));
}

#[test]
fn context_pack_span_level_packing_is_opt_in() {
    let payload = format!(
        "scope=project:investments\nstatus=ready\nsource=doc-a\n\n{}\nApollo blocker: database migration is owned by Maya.",
        "intro background ".repeat(80)
    );
    let pack = ContextPack::from_retrieved_with_options(
        vec![retrieved(1, &payload)],
        48,
        false,
        &ContextPackOptions::default(),
        r#"RETRIEVE CONTEXT FOR TASK "Apollo database migration blocker" IN BRAIN investment_projects;"#,
    );

    let packed = String::from_utf8_lossy(&pack.cells[0].payload);
    assert!(packed.contains("intro background intro background"));
    assert!(!packed.contains("[context_pack_span=true"));
}
