use std::collections::BTreeMap;
use std::{env, fs, path::PathBuf};

use serde::Serialize;

use crate::cases::{recall_cases, run_case};

const Q16_ONE: u32 = 65_535;
const MIN_CASES: usize = 150;
const MIN_RECALL_Q16: u32 = 58_981;
const MAX_FALSE_CONFLICT_RATE_Q16: u32 = 3_276;

pub(crate) fn run_benchmark() -> RecallReport {
    let cases = recall_cases();
    let mut failures = Vec::new();
    let mut class_counts = BTreeMap::<String, ClassStats>::new();
    let mut true_positive_count = 0;
    let mut false_negative_count = 0;
    let mut true_negative_count = 0;
    let mut false_conflict_count = 0;
    let mut observed_conflict_count = 0;

    for case in &cases {
        let observed = run_case(case);
        let stats = class_counts.entry(case.class.to_owned()).or_default();
        stats.case_count += 1;
        if case.expected_kind.is_some() {
            stats.expected_conflicts += 1;
            if observed.detected_expected_kind {
                true_positive_count += 1;
                stats.detected_conflicts += 1;
            } else {
                false_negative_count += 1;
                stats.false_negatives += 1;
                failures.push(observed.failure_message(case));
            }
        } else if observed.has_conflict {
            false_conflict_count += 1;
            stats.false_conflicts += 1;
            failures.push(observed.failure_message(case));
        } else {
            true_negative_count += 1;
        }
        if observed.has_conflict {
            observed_conflict_count += 1;
        }
    }

    let conflict_case_count = true_positive_count + false_negative_count;
    let no_conflict_case_count = true_negative_count + false_conflict_count;
    let recall_q16 = q16(true_positive_count, conflict_case_count);
    let precision_q16 = q16(true_positive_count, observed_conflict_count);
    let false_conflict_rate_q16 = q16(false_conflict_count, no_conflict_case_count);
    validate_totals(
        cases.len(),
        recall_q16,
        false_conflict_rate_q16,
        &class_counts,
        &mut failures,
    );

    RecallReport {
        schema_version: "cortexdb.verify_conflict_recall.report.v1",
        status: if failures.is_empty() {
            "passed"
        } else {
            "failed"
        },
        case_count: cases.len(),
        conflict_case_count,
        no_conflict_case_count,
        true_positive_count,
        false_negative_count,
        true_negative_count,
        false_conflict_count,
        observed_conflict_count,
        recall_q16,
        precision_q16,
        false_conflict_rate_q16,
        recall_percent: percent(recall_q16),
        precision_percent: percent(precision_q16),
        false_conflict_rate_percent: percent(false_conflict_rate_q16),
        thresholds: Thresholds {
            min_cases: MIN_CASES,
            min_recall_q16: MIN_RECALL_Q16,
            max_false_conflict_rate_q16: MAX_FALSE_CONFLICT_RATE_Q16,
        },
        class_counts,
        failures,
    }
}

fn validate_totals(
    case_count: usize,
    recall_q16: u32,
    false_conflict_rate_q16: u32,
    class_counts: &BTreeMap<String, ClassStats>,
    failures: &mut Vec<String>,
) {
    if case_count < MIN_CASES {
        failures.push(format!("expected at least {MIN_CASES} cases"));
    }
    for required in [
        "magnitude",
        "unit",
        "currency",
        "temporal",
        "citation",
        "format",
        "must_not_conflict",
    ] {
        if !class_counts.contains_key(required) {
            failures.push(format!("missing class {required}"));
        }
    }
    if recall_q16 < MIN_RECALL_Q16 {
        failures.push(format!(
            "recall_q16={recall_q16} below threshold {MIN_RECALL_Q16}"
        ));
    }
    if false_conflict_rate_q16 > MAX_FALSE_CONFLICT_RATE_Q16 {
        failures.push(format!(
            "false_conflict_rate_q16={false_conflict_rate_q16} above threshold {MAX_FALSE_CONFLICT_RATE_Q16}"
        ));
    }
}

fn q16(numerator: usize, denominator: usize) -> u32 {
    if denominator == 0 {
        return Q16_ONE;
    }
    ((numerator as u32) * Q16_ONE) / (denominator as u32)
}

fn percent(value_q16: u32) -> String {
    let hundredths = (value_q16 * 10_000 + (Q16_ONE / 2)) / Q16_ONE;
    format!("{}.{:02}%", hundredths / 100, hundredths % 100)
}

pub(crate) fn report_path() -> Option<PathBuf> {
    env::var_os("CORTEXDB_VERIFY_CONFLICT_RECALL_REPORT").map(PathBuf::from)
}

pub(crate) fn write_report(path: &PathBuf, report: &RecallReport) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, serde_json::to_string_pretty(report).unwrap() + "\n").unwrap();
}

#[derive(Default, Serialize)]
pub(crate) struct ClassStats {
    case_count: usize,
    expected_conflicts: usize,
    detected_conflicts: usize,
    false_conflicts: usize,
    false_negatives: usize,
}

#[derive(Serialize)]
pub(crate) struct Thresholds {
    min_cases: usize,
    min_recall_q16: u32,
    max_false_conflict_rate_q16: u32,
}

#[derive(Serialize)]
pub(crate) struct RecallReport {
    schema_version: &'static str,
    status: &'static str,
    case_count: usize,
    conflict_case_count: usize,
    no_conflict_case_count: usize,
    true_positive_count: usize,
    false_negative_count: usize,
    true_negative_count: usize,
    false_conflict_count: usize,
    observed_conflict_count: usize,
    recall_q16: u32,
    precision_q16: u32,
    false_conflict_rate_q16: u32,
    recall_percent: String,
    precision_percent: String,
    false_conflict_rate_percent: String,
    thresholds: Thresholds,
    class_counts: BTreeMap<String, ClassStats>,
    pub(crate) failures: Vec<String>,
}
