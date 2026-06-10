#!/usr/bin/env python3
"""Validate local HNSW no-fallback future-epic evidence gates."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

MIN_RECALL_Q16 = 49_151

GATES: dict[str, dict[str, object]] = {
    "production-no-fallback": {
        "schema": "cortexdb.hnsw_no_fallback.production_gate.v1",
        "required_evidence": [
            "fixture",
            "external",
            "metric_matrix",
            "domain",
            "recall_probe",
            "reference_suite",
        ],
        "markers": [
            ("docs/HNSW_NO_FALLBACK_PRODUCTION_DESIGN.md", "Serving Guardrails"),
            ("docs/HNSW_NO_FALLBACK_PRODUCTION_DESIGN.md", "Runtime Rollout Policy"),
            ("docs/HNSW_NO_FALLBACK_PRODUCTION_DESIGN.md", "ann_no_fallback_blocked"),
            ("docs/HNSW_NO_FALLBACK_PRODUCTION_DESIGN.md", "p99 latency"),
            ("docs/HNSW_NO_FALLBACK_PRODUCTION_DESIGN.md", "cortexdb_ann_search_latency_ms_bucket"),
            ("docs/HNSW_NO_FALLBACK_PRODUCTION_DESIGN.md", "long-running recall probes"),
            ("docs/SEARCH.md", "Guarded production mode"),
            ("docs/SEARCH.md", "p99 tail-latency"),
            ("docs/SEARCH.md", "ann_search_latency_ms"),
            ("docs/OBSERVABILITY_ALERTS.md", "CortexDbAnnNoFallbackBlocked"),
            ("examples/observability/alerts.yml", "cortexdb_ann_no_fallback_blocked"),
            ("crates/cortex-server/src/lib.rs", "cortexdb_ann_search_latency_ms_bucket"),
            ("crates/cortex-server/src/lib.rs", "cortexdb_ann_no_fallback_blocked"),
            ("scripts/ann/recall_probe.py", "cortexdb.ann_recall_probe.v1"),
            ("scripts/ann/reference_suite_gate.py", "cortexdb.ann_reference_suite_report.v1"),
            ("crates/cortex-engine/src/search/hnsw_no_fallback.rs", "HnswNoFallbackRolloutPolicy"),
            ("crates/cortex-engine/src/search/hnsw_no_fallback.rs", "RolloutDisabled"),
            ("crates/cortex-engine/src/search/hnsw_no_fallback.rs", "FallbackEnabled"),
            ("crates/cortex-engine/src/search/hnsw_no_fallback.rs", "RecallBelowMinimum"),
            ("crates/cortex-engine/src/search/hnsw_no_fallback.rs", "ReportNotProductionSafe"),
            ("Makefile", "ann-production-no-fallback-check"),
            ("Makefile", "ann-recall-probe-report"),
        ],
    },
    "real-domain-history": {
        "schema": "cortexdb.hnsw_no_fallback.real_domain_history_gate.v1",
        "required_evidence": ["domain"],
        "markers": [
            ("docs/HNSW_NO_FALLBACK_PRODUCTION_DESIGN.md", "Recall SLO"),
            ("docs/FUTURE_NON_GOAL_EPICS.md", "make ann-real-domain-history-check"),
            ("Makefile", "ann-real-domain-history-check"),
        ],
    },
    "public-corpus-history": {
        "schema": "cortexdb.hnsw_no_fallback.public_corpus_history_gate.v1",
        "required_evidence": [],
        "markers": [
            ("docs/HNSW_NO_FALLBACK_PRODUCTION_DESIGN.md", "Latency SLO"),
            ("docs/BENCHMARKS.md", "ann-public-corpus-run"),
            ("Makefile", "ann-public-corpus-history-check"),
        ],
    },
    "graph-freshness": {
        "schema": "cortexdb.hnsw_no_fallback.graph_freshness_gate.v1",
        "required_evidence": [],
        "markers": [
            ("docs/HNSW_NO_FALLBACK_PRODUCTION_DESIGN.md", "Graph Freshness"),
            ("docs/SEARCH.md", "has_uncheckpointed_changes"),
            ("crates/cortex-engine/tests/hnsw_persistence.rs", "hnsw_maintenance_reports_rebuild_lifecycle"),
            ("crates/cortex-engine/tests/hnsw_persistence.rs", "hnsw_delete_and_rebuild_policy_removes_deleted_vectors"),
            ("crates/cortex-engine/tests/hnsw_manifest_profile.rs", "validation_rejects_hnsw_graph_profile_that_differs_from_manifest"),
            ("crates/cortex-engine/src/search/hnsw_no_fallback.rs", "WeakMultiLayerTopology"),
            ("Makefile", "ann-graph-freshness-check"),
        ],
    },
}

def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as error:
        raise RuntimeError(f"failed to read {path}: {error}") from error

def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        raise RuntimeError(f"failed to read evidence report {path}: {error}") from error
    except json.JSONDecodeError as error:
        raise RuntimeError(f"evidence report {path} is invalid JSON: {error}") from error
    if not isinstance(value, dict):
        raise RuntimeError(f"evidence report {path} must be a JSON object")
    return value

def parse_evidence(values: list[str]) -> dict[str, Path]:
    result: dict[str, Path] = {}
    for value in values:
        if "=" not in value:
            raise RuntimeError(f"invalid evidence {value!r}; expected label=path")
        label, path = value.split("=", 1)
        if not label or not path:
            raise RuntimeError(f"invalid evidence {value!r}; expected label=path")
        result[label] = Path(path)
    return result

def validate_markers(markers: list[tuple[str, str]]) -> list[str]:
    failures: list[str] = []
    for file_name, marker in markers:
        if marker not in read(Path(file_name)):
            failures.append(f"marker {marker!r} missing from {file_name}")
    return failures

def int_value(report: dict[str, Any], field: str) -> int | None:
    value = report.get(field)
    return value if isinstance(value, int) else None

def bool_value(report: dict[str, Any], field: str) -> bool | None:
    value = report.get(field)
    return value if isinstance(value, bool) else None

def observed(report: dict[str, Any]) -> dict[str, Any]:
    nested = report.get("observed")
    return nested if isinstance(nested, dict) else report

def report_failures(label: str, report: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    value = observed(report)
    if bool_value(report, "passed") is not True:
        failures.append(f"{label}: passed must be true")
    if bool_value(value, "production_safe") is not True:
        failures.append(f"{label}: production_safe must be true")
    if (int_value(value, "upper_layers") or 0) <= 0:
        failures.append(f"{label}: upper_layers must be > 0")
    if (int_value(value, "upper_graph_edges") or 0) <= 0:
        failures.append(f"{label}: upper_graph_edges must be > 0")
    if (int_value(value, "graph_nodes") or 0) <= 0:
        failures.append(f"{label}: graph_nodes must be > 0")
    min_recall = int_value(value, "min_observed_recall_q16")
    mean_recall = int_value(value, "mean_recall_q16")
    if min_recall is None or min_recall < MIN_RECALL_Q16:
        failures.append(f"{label}: min_observed_recall_q16 below local threshold")
    if mean_recall is None or mean_recall < MIN_RECALL_Q16:
        failures.append(f"{label}: mean_recall_q16 below local threshold")
    p95 = int_value(value, "p95_latency_nanos")
    p99 = int_value(value, "p99_latency_nanos")
    max_latency = int_value(value, "max_latency_nanos")
    if p95 is None or p95 <= 0:
        failures.append(f"{label}: p95_latency_nanos must be positive")
    if p99 is None or p99 <= 0:
        failures.append(f"{label}: p99_latency_nanos must be positive")
    if max_latency is None or max_latency <= 0:
        failures.append(f"{label}: max_latency_nanos must be positive")
    allowed_p95 = int_value(value, "allowed_p95_latency_nanos")
    allowed_p99 = int_value(value, "allowed_p99_latency_nanos")
    allowed_max = int_value(value, "allowed_max_latency_nanos")
    if allowed_p95 is not None and p95 is not None and p95 > allowed_p95:
        failures.append(f"{label}: p95 latency exceeds gate policy")
    if allowed_p99 is not None and p99 is not None and p99 > allowed_p99:
        failures.append(f"{label}: p99 latency exceeds gate policy")
    if allowed_max is not None and max_latency is not None and max_latency > allowed_max:
        failures.append(f"{label}: max latency exceeds gate policy")
    return failures

def metric_matrix_failures(report: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    if bool_value(report, "passed") is not True:
        failures.append("metric_matrix: passed must be true")
    metrics = report.get("metrics")
    if not isinstance(metrics, list) or not metrics:
        return failures + ["metric_matrix: metrics must be a non-empty list"]
    observed_metrics: set[str] = set()
    for item in metrics:
        if not isinstance(item, dict):
            failures.append("metric_matrix: metric entry must be object")
            continue
        metric = item.get("metric")
        if isinstance(metric, str):
            observed_metrics.add(metric)
        failures.extend(report_failures(f"metric_matrix:{metric}", {"passed": True, **item}))
    missing = {"dot_product", "cosine", "l2"}.difference(observed_metrics)
    if missing:
        failures.append(f"metric_matrix: missing metrics {', '.join(sorted(missing))}")
    return failures

def recall_probe_failures(report: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    if report.get("schema_version") != "cortexdb.ann_recall_probe.v1":
        failures.append("recall_probe: invalid schema_version")
    if bool_value(report, "passed") is not True:
        failures.append("recall_probe: passed must be true")
    if bool_value(report, "production_safe") is not True:
        failures.append("recall_probe: production_safe must be true")
    if (int_value(report, "iterations") or 0) < 3:
        failures.append("recall_probe: expected at least three iterations")
    if (int_value(report, "min_observed_recall_q16_min") or 0) < MIN_RECALL_Q16:
        failures.append("recall_probe: min recall below local threshold")
    if (int_value(report, "mean_recall_q16_min") or 0) < MIN_RECALL_Q16:
        failures.append("recall_probe: mean recall below local threshold")
    if (int_value(report, "p99_latency_nanos_max") or 0) <= 0:
        failures.append("recall_probe: p99 latency must be positive")
    shape = report.get("graph_shape")
    if not isinstance(shape, dict) or not shape:
        failures.append("recall_probe: graph_shape must be present")
    return failures

def reference_suite_failures(report: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    if report.get("schema_version") != "cortexdb.ann_reference_suite_report.v1":
        failures.append("reference_suite: invalid schema_version")
    if report.get("status") != "passed":
        failures.append("reference_suite: status must be passed")
    if (int_value(report, "corpus_count") or 0) < 3:
        failures.append("reference_suite: expected at least three corpora")
    corpora = report.get("corpora")
    if not isinstance(corpora, list) or not corpora:
        return failures + ["reference_suite: corpora must be non-empty"]
    for corpus in corpora:
        if not isinstance(corpus, dict):
            failures.append("reference_suite: corpus entry must be object")
            continue
        name = corpus.get("name", "<unknown>")
        if corpus.get("status") != "passed":
            failures.append(f"reference_suite:{name}: status must be passed")
        if int_value(corpus, "fallback_count") != 0:
            failures.append(f"reference_suite:{name}: fallback_count must be zero")
        if bool_value(corpus, "production_safe") is not True:
            failures.append(f"reference_suite:{name}: production_safe must be true")
    return failures

def history_failures(history: dict[str, Any], *, min_runs: int) -> list[str]:
    failures: list[str] = []
    if int_value(history, "run_count") is None or int(history["run_count"]) < min_runs:
        failures.append(f"history: expected at least {min_runs} run(s)")
    if int_value(history, "corpus_count") is None or int(history["corpus_count"]) < 1:
        failures.append("history: expected at least one corpus")
    if int_value(history, "regression_count") is None or int(history["regression_count"]) != 0:
        failures.append("history: expected zero regressions")
    corpora = history.get("corpora")
    if not isinstance(corpora, list) or not corpora:
        failures.append("history: corpora must be non-empty")
    elif not all(isinstance(item, dict) and item.get("latest_production_safe") is True for item in corpora):
        failures.append("history: every latest corpus run must be production_safe")
    return failures

def validate(gate: str, evidence: dict[str, Path], history_path: Path | None) -> dict[str, Any]:
    spec = GATES[gate]
    failures = validate_markers(spec["markers"])  # type: ignore[arg-type]
    required = list(spec["required_evidence"])  # type: ignore[arg-type]
    reports: dict[str, dict[str, Any]] = {}
    for label in required:
        if label not in evidence:
            failures.append(f"missing --evidence {label}=<path>")
            continue
        reports[label] = load_json(evidence[label])
    for label, path in evidence.items():
        reports.setdefault(label, load_json(path))

    if gate == "production-no-fallback":
        for label in ["fixture", "external", "domain"]:
            if label in reports:
                failures.extend(report_failures(label, reports[label]))
        if "metric_matrix" in reports:
            failures.extend(metric_matrix_failures(reports["metric_matrix"]))
        if "recall_probe" in reports:
            failures.extend(recall_probe_failures(reports["recall_probe"]))
        if "reference_suite" in reports:
            failures.extend(reference_suite_failures(reports["reference_suite"]))
    elif gate == "real-domain-history":
        if "domain" in reports:
            failures.extend(report_failures("domain", reports["domain"]))
        if history_path is None:
            failures.append("missing --history")
        else:
            failures.extend(history_failures(load_json(history_path), min_runs=2))
    elif gate == "public-corpus-history":
        if history_path is None:
            failures.append("missing --history")
        else:
            failures.extend(history_failures(load_json(history_path), min_runs=2))

    local_profile_ready = not failures and gate in {
        "production-no-fallback",
        "real-domain-history",
        "graph-freshness",
    }
    return {
        "schema_version": spec["schema"],
        "gate": gate,
        "status": "passed" if not failures else "failed",
        "selected_local_profiles_ready": local_profile_ready,
        "fallback_free_general_ready": False,
        "boundary": "local no-fallback prerequisite evidence only; no global HNSW production claim",
        "evidence_reports": {label: str(path) for label, path in sorted(evidence.items())},
        "history": str(history_path) if history_path else "",
        "reports_checked": sorted(reports),
        "failures": failures,
    }

def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--gate", required=True, choices=sorted(GATES))
    parser.add_argument("--evidence", action="append", default=[])
    parser.add_argument("--history", type=Path)
    parser.add_argument("--report", required=True)
    return parser.parse_args(argv)

def main(argv: list[str]) -> int:
    args = parse_args(argv)
    output = Path(args.report)
    try:
        report = validate(args.gate, parse_evidence(args.evidence), args.history)
    except RuntimeError as error:
        print(f"HNSW no-fallback gate check failed: {error}", file=sys.stderr)
        return 1
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"HNSW no-fallback {args.gate} check passed: {output}")
    return 0

if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
