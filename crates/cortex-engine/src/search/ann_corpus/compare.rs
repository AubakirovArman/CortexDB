use super::{AnnCorpusOptions, AnnCorpusReport};

pub(super) fn compare_corpus_report(
    options: AnnCorpusOptions,
    report: &AnnCorpusReport,
) -> Vec<String> {
    let mut failures = Vec::new();
    check_min(
        &mut failures,
        "min_observed_recall_q16",
        report.min_observed_recall_q16,
        options.min_recall_q16,
    );
    check_min(
        &mut failures,
        "mean_recall_q16",
        report.mean_recall_q16,
        options.min_mean_recall_q16,
    );
    check_max(
        &mut failures,
        "p95_latency_nanos",
        report.p95_latency_nanos,
        options.max_p95_latency_nanos,
    );
    check_max(
        &mut failures,
        "p99_latency_nanos",
        report.p99_latency_nanos,
        options.max_p99_latency_nanos,
    );
    check_max(
        &mut failures,
        "max_latency_nanos",
        report.max_latency_nanos,
        options.max_max_latency_nanos,
    );
    if options.require_production_safe && !report.production_safe {
        failures.push("production_safe expected true, observed false".to_owned());
    }
    if options.require_production_safe && report.graph_freshness_q16 < 65_535 {
        failures.push(format!(
            "graph_freshness_q16: expected 65535, observed {}",
            report.graph_freshness_q16
        ));
    }
    if options.require_production_safe && report.fallback_count > 0 {
        failures.push(format!(
            "fallback_count: expected 0, observed {}",
            report.fallback_count
        ));
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
