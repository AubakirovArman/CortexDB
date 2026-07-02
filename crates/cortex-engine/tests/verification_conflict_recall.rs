#[path = "verification_conflict_recall/cases.rs"]
mod cases;
#[path = "verification_conflict_recall/report.rs"]
mod report;

#[test]
fn verification_conflict_recall_benchmark_meets_thresholds() {
    let report = report::run_benchmark();
    if let Some(path) = report::report_path() {
        report::write_report(&path, &report);
    }
    assert!(
        report.failures.is_empty(),
        "verification conflict recall failures: {:#?}",
        report.failures
    );
}
