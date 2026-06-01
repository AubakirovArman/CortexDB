use super::{AnnExternalFixtureBaseline, AnnExternalFixtureReport};

pub(super) fn compare_external_baseline(
    baseline: &AnnExternalFixtureBaseline,
    report: &AnnExternalFixtureReport,
) -> Vec<String> {
    let mut failures = Vec::new();
    check_min(
        &mut failures,
        "min_observed_recall_q16",
        report.min_observed_recall_q16,
        baseline.min_observed_recall_q16,
    );
    check_min(
        &mut failures,
        "mean_recall_q16",
        report.mean_recall_q16,
        baseline.min_mean_recall_q16,
    );
    check_min(
        &mut failures,
        "graph_nodes",
        report.graph_nodes,
        baseline.min_graph_nodes,
    );
    check_min(
        &mut failures,
        "graph_edges",
        report.graph_edges,
        baseline.min_graph_edges,
    );
    check_min(
        &mut failures,
        "upper_layers",
        report.upper_layers,
        baseline.min_upper_layers,
    );
    check_min(
        &mut failures,
        "upper_graph_edges",
        report.upper_graph_edges,
        baseline.min_upper_graph_edges,
    );
    check_max(
        &mut failures,
        "p95_latency_nanos",
        report.p95_latency_nanos,
        baseline.max_p95_latency_nanos,
    );
    check_max(
        &mut failures,
        "p99_latency_nanos",
        report.p99_latency_nanos,
        baseline.max_p99_latency_nanos,
    );
    check_max(
        &mut failures,
        "max_latency_nanos",
        report.max_latency_nanos,
        baseline.max_max_latency_nanos,
    );
    if baseline.require_production_safe && !report.production_safe {
        failures.push("production_safe: expected true, observed false".to_owned());
    }
    failures
}

fn check_min<T>(failures: &mut Vec<String>, field: &str, observed: T, minimum: T)
where
    T: std::fmt::Display + PartialOrd,
{
    if observed < minimum {
        failures.push(format!(
            "{field}: expected >= {minimum}, observed {observed}"
        ));
    }
}

fn check_max<T>(failures: &mut Vec<String>, field: &str, observed: T, maximum: T)
where
    T: std::fmt::Display + PartialOrd,
{
    if observed > maximum {
        failures.push(format!(
            "{field}: expected <= {maximum}, observed {observed}"
        ));
    }
}
