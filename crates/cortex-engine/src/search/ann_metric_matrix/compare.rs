use std::collections::BTreeMap;

use super::{AnnMetricBaseline, AnnMetricMatrixBaseline, AnnMetricReport};

pub(super) fn compare_metric_matrix_baseline(
    baseline: &AnnMetricMatrixBaseline,
    reports: &[AnnMetricReport],
) -> Vec<String> {
    let mut failures = Vec::new();
    let observed = reports
        .iter()
        .map(|report| (report.metric.as_str(), report))
        .collect::<BTreeMap<_, _>>();
    for metric in &baseline.metrics {
        let Some(report) = observed.get(metric.metric.as_str()) else {
            failures.push(format!("{}: metric report missing", metric.metric));
            continue;
        };
        compare_metric(metric, report, &mut failures);
    }
    failures
}

fn compare_metric(
    baseline: &AnnMetricBaseline,
    report: &AnnMetricReport,
    failures: &mut Vec<String>,
) {
    check_min(
        failures,
        &baseline.metric,
        "min_observed_recall_q16",
        report.min_observed_recall_q16,
        baseline.min_observed_recall_q16,
    );
    check_min(
        failures,
        &baseline.metric,
        "mean_recall_q16",
        report.mean_recall_q16,
        baseline.min_mean_recall_q16,
    );
    check_min(
        failures,
        &baseline.metric,
        "graph_nodes",
        report.graph_nodes,
        baseline.min_graph_nodes,
    );
    check_min(
        failures,
        &baseline.metric,
        "graph_edges",
        report.graph_edges,
        baseline.min_graph_edges,
    );
    check_min(
        failures,
        &baseline.metric,
        "upper_layers",
        report.upper_layers,
        baseline.min_upper_layers,
    );
    check_min(
        failures,
        &baseline.metric,
        "upper_graph_edges",
        report.upper_graph_edges,
        baseline.min_upper_graph_edges,
    );
    check_max(
        failures,
        &baseline.metric,
        "p95_latency_nanos",
        report.p95_latency_nanos,
        baseline.max_p95_latency_nanos,
    );
    check_max(
        failures,
        &baseline.metric,
        "max_latency_nanos",
        report.max_latency_nanos,
        baseline.max_max_latency_nanos,
    );
    if baseline.require_production_safe && !report.production_safe {
        failures.push(format!(
            "{}: production_safe expected true, observed false",
            baseline.metric
        ));
    }
}

fn check_min<T>(failures: &mut Vec<String>, metric: &str, field: &str, observed: T, minimum: T)
where
    T: std::fmt::Display + PartialOrd,
{
    if observed < minimum {
        failures.push(format!(
            "{metric}.{field}: expected >= {minimum}, observed {observed}"
        ));
    }
}

fn check_max<T>(failures: &mut Vec<String>, metric: &str, field: &str, observed: T, maximum: T)
where
    T: std::fmt::Display + PartialOrd,
{
    if observed > maximum {
        failures.push(format!(
            "{metric}.{field}: expected <= {maximum}, observed {observed}"
        ));
    }
}
