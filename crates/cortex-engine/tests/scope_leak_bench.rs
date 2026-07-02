#[path = "scope_leak_bench/fixture.rs"]
mod fixture;

use cortex_aql::AgentView;
use cortex_engine::{
    AnnFallbackReason, AnnSearchPolicy, ContextPack, ContextPackExportFormat, ContextPackOptions,
    Database, SearchLimit,
};
use fixture::{
    assert_no_forbidden, hnsw_options, public_agent_views, seed_scope_leak_fixture, verify_query,
    BudgetMode, QueryShape, CONTEXT_FORMATS, VERIFICATION_FORMATS,
};
use serde_json::Value;

#[test]
fn scope_leak_bench_scans_all_output_surfaces() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open_with_options(dir.path(), hnsw_options()).unwrap();
    seed_scope_leak_fixture(&mut db);

    let mut surface_count = scan_state("in_memory", &db);
    db.checkpoint().unwrap();
    surface_count += scan_state("post_checkpoint", &db);
    db.compact().unwrap();
    surface_count += scan_state("post_compact", &db);

    assert!(
        surface_count >= 200,
        "scope leak bench must cover >=200 surfaces, covered {surface_count}"
    );
}

fn scan_state(state: &str, db: &Database) -> usize {
    let mut surface_count = 0;
    for view in public_agent_views() {
        surface_count += scan_verification_surfaces(state, db, &view);
        for query_shape in QueryShape::ALL {
            for budget_mode in BudgetMode::ALL {
                for (format, format_name) in CONTEXT_FORMATS {
                    let label = format!(
                        "{state}/agent-{}/query-{}/budget-{}/format-{format_name}",
                        view.agent_id.0,
                        query_shape.as_str(),
                        budget_mode.as_str()
                    );
                    surface_count +=
                        scan_context_surface(db, &view, query_shape, budget_mode, format, &label);
                }
            }
        }
    }
    surface_count
}

fn scan_context_surface(
    db: &Database,
    view: &AgentView,
    query_shape: QueryShape,
    budget_mode: BudgetMode,
    format: ContextPackExportFormat,
    label: &str,
) -> usize {
    match budget_mode {
        BudgetMode::Normal | BudgetMode::TightToken => {
            let options = ContextPackOptions {
                token_budget_tokens: budget_mode.token_budget(),
                ..ContextPackOptions::default()
            };
            match db.context_pack_from_aql(query_shape.query(), view, options) {
                Ok(pack) => scan_context_pack_surface(label, &pack, format),
                Err(error) => {
                    let message = error.safe_message();
                    assert_no_forbidden(label, &message);
                    1
                }
            }
        }
        BudgetMode::AnnBudgetExhausted => {
            let outcome = db
                .search_vector_with_report_with_policy(
                    &[16, 0],
                    view,
                    SearchLimit(4),
                    AnnSearchPolicy {
                        max_visited_candidates: Some(0),
                        fallback: true,
                        ..AnnSearchPolicy::default()
                    },
                )
                .unwrap();
            if let Some(report) = &outcome.ann_report {
                if label.starts_with("in_memory/") {
                    assert_eq!(
                        report.fallback_reason,
                        Some(AnnFallbackReason::NoPersistedSegments)
                    );
                } else {
                    assert_eq!(
                        report.fallback_reason,
                        Some(AnnFallbackReason::VisitBudgetExceeded)
                    );
                }
            }
            let pack = db.context_pack_from_search_outcome_with_options(
                outcome,
                view,
                1_000,
                true,
                &ContextPackOptions::default(),
                "scope leak vector budget",
            );
            scan_context_pack_surface(label, &pack, format)
        }
    }
}

fn scan_context_pack_surface(
    label: &str,
    pack: &ContextPack,
    format: ContextPackExportFormat,
) -> usize {
    let exported = pack.export(format);
    assert_no_forbidden(label, &exported);
    if format == ContextPackExportFormat::Json {
        let value: Value = serde_json::from_str(&exported).unwrap();
        if !pack.cells.is_empty() {
            assert!(value["cells"][0]["source_ref"].is_object());
            assert!(value["cells"][0]["explain"]["why_selected"].is_string());
            assert!(value["cells"][0]["explain"]["score_components"].is_array());
        }
        let mut strings = Vec::new();
        collect_json_strings("$", &value, &mut strings);
        for (path, value) in strings {
            assert_no_forbidden(&format!("{label}/json/{path}"), &value);
        }
    }
    1
}

fn scan_verification_surfaces(state: &str, db: &Database, view: &AgentView) -> usize {
    let label = format!("{state}/agent-{}/verify", view.agent_id.0);
    let report = db.verify_fact_aql(verify_query(), view).unwrap();
    assert!(!report.numeric_conflicts.is_empty());
    assert_no_forbidden(&format!("{label}/fact"), &report.fact);
    for evidence in report
        .evidence
        .iter()
        .chain(report.contradicting_evidence.iter())
    {
        if let Some(citation) = &evidence.citation {
            assert_no_forbidden(&format!("{label}/evidence/citation"), citation);
        }
    }
    for conflict in &report.numeric_conflicts {
        assert_no_forbidden(
            &format!("{label}/numeric_conflicts/metric"),
            &conflict.metric,
        );
        assert_no_forbidden(&format!("{label}/numeric_conflicts/left"), &conflict.left);
        assert_no_forbidden(&format!("{label}/numeric_conflicts/right"), &conflict.right);
    }
    for guard in &report.guards {
        assert_no_forbidden(&format!("{label}/guard/message"), &guard.message);
    }
    for (format, format_name) in VERIFICATION_FORMATS {
        let exported = report.export(format);
        assert_no_forbidden(&format!("{label}/export/{format_name}"), &exported);
    }
    2
}

fn collect_json_strings(path: &str, value: &Value, out: &mut Vec<(String, String)>) {
    match value {
        Value::String(text) => out.push((path.to_owned(), text.to_owned())),
        Value::Array(values) => {
            for (index, item) in values.iter().enumerate() {
                collect_json_strings(&format!("{path}[{index}]"), item, out);
            }
        }
        Value::Object(values) => {
            for (key, item) in values {
                collect_json_strings(&format!("{path}.{key}"), item, out);
            }
        }
        _ => {}
    }
}
