#!/usr/bin/env python3
"""Build a CortexDB release regression dashboard from local evidence reports."""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any


@dataclass(frozen=True)
class MetricRule:
    domain: str
    name: str
    direction: str
    max_ratio: float = 1.0


RULES: tuple[MetricRule, ...] = (
    MetricRule("storage", "storage.backup_drills", "higher_or_equal"),
    MetricRule("storage", "storage.single_node_duration_ms", "lower_or_equal", 2.0),
    MetricRule("search", "search.retrieval_mean_recall_q16", "higher_or_equal"),
    MetricRule("search", "search.retrieval_mean_mrr_q16", "higher_or_equal"),
    MetricRule("search", "search.retrieval_p95_latency_nanos", "lower_or_equal", 2.0),
    MetricRule("context", "context.token_reduction_q16", "higher_or_equal"),
    MetricRule("context", "context.evidence_coverage_q16", "higher_or_equal"),
    MetricRule("context", "context.citation_coverage_q16", "higher_or_equal"),
    MetricRule("context", "context.deterministic_order_q16", "higher_or_equal"),
    MetricRule("verify", "verify.accuracy_q16", "higher_or_equal"),
    MetricRule("verify", "verify.false_positive_count", "lower_or_equal"),
    MetricRule("verify", "verify.false_negative_count", "lower_or_equal"),
    MetricRule("api", "api.http_contract_checks_passed", "higher_or_equal"),
    MetricRule("sdk", "sdk.packages", "higher_or_equal"),
    MetricRule("sdk", "sdk.release_checks_passed", "higher_or_equal"),
)


def repo_root() -> Path:
    return Path(__file__).resolve().parent.parent


def read_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except FileNotFoundError as error:
        raise RuntimeError(f"missing report: {path}") from error
    if not isinstance(value, dict):
        raise RuntimeError(f"{path}: expected JSON object")
    return value


def status_passed(data: dict[str, Any]) -> bool:
    status = data.get("status")
    if isinstance(status, str):
        return status.lower() in {"ok", "passed"}
    return data.get("ok") is True


def count_true(value: Any) -> int:
    if not isinstance(value, dict):
        return 0
    return sum(1 for item in value.values() if item is True)


def current_metrics(args: argparse.Namespace) -> tuple[dict[str, float], list[str]]:
    repo = repo_root()
    backup = read_json(repo / args.backup_report)
    single_node = read_json(repo / args.single_node_report)
    retrieval = read_json(repo / args.retrieval_report)
    context = read_json(repo / args.context_report)
    verify = read_json(repo / args.verify_report)
    api = read_json(repo / args.api_report)
    sdk = read_json(repo / args.sdk_report)

    failures: list[str] = []
    for name, report in (
        ("backup", backup),
        ("single_node", single_node),
        ("retrieval", retrieval),
        ("context", context),
        ("verify", verify),
        ("api", api),
        ("sdk", sdk),
    ):
        if not status_passed(report):
            failures.append(f"{name} report is not passed/ok")

    retrieval_history = retrieval.get("history", {})
    if not isinstance(retrieval_history, dict):
        retrieval_history = {}

    return (
        {
            "storage.backup_drills": float(len(backup.get("drills", []))),
            "storage.single_node_duration_ms": float(single_node.get("duration_ms", 0.0)),
            "search.retrieval_mean_recall_q16": float(retrieval_history.get("latest_mean_recall_q16", 0)),
            "search.retrieval_mean_mrr_q16": float(retrieval_history.get("latest_mean_mrr_q16", 0)),
            "search.retrieval_p95_latency_nanos": float(retrieval_history.get("latest_p95_latency_nanos", 0)),
            "context.token_reduction_q16": float(context.get("token_reduction_q16", 0)),
            "context.evidence_coverage_q16": float(context.get("evidence_coverage_q16", 0)),
            "context.citation_coverage_q16": float(context.get("citation_coverage_q16", 0)),
            "context.deterministic_order_q16": float(context.get("deterministic_order_q16", 0)),
            "verify.accuracy_q16": float(verify.get("accuracy_q16", 0)),
            "verify.false_positive_count": float(verify.get("false_positive_count", 0)),
            "verify.false_negative_count": float(verify.get("false_negative_count", 0)),
            "api.http_contract_checks_passed": float(count_true(api.get("checks"))),
            "sdk.packages": float(len(sdk.get("packages", []))),
            "sdk.release_checks_passed": float(count_true(sdk.get("checks"))),
        },
        failures,
    )


def compare(rule: MetricRule, current: float, previous: float) -> tuple[str, float]:
    ratio = 0.0 if previous == 0 else round(current / previous, 6)
    if rule.direction == "higher_or_equal":
        return ("passed" if current >= previous else "failed", ratio)
    if rule.direction == "lower_or_equal":
        allowed = previous * rule.max_ratio
        return ("passed" if current <= allowed else "failed", ratio)
    raise ValueError(f"unknown direction {rule.direction}")


def build_report(args: argparse.Namespace) -> dict[str, Any]:
    repo = repo_root()
    baseline = read_json(repo / args.baseline)
    baseline_metrics = baseline.get("metrics")
    if not isinstance(baseline_metrics, dict):
        raise RuntimeError(f"{args.baseline}: missing metrics object")
    current, failures = current_metrics(args)

    comparisons: list[dict[str, Any]] = []
    domain_status: dict[str, str] = {}
    for rule in RULES:
        previous = float(baseline_metrics.get(rule.name, 0.0))
        observed = float(current.get(rule.name, 0.0))
        status, ratio = compare(rule, observed, previous)
        if status != "passed":
            failures.append(f"{rule.name}: current={observed} baseline={previous}")
        domain_status[rule.domain] = "failed" if status != "passed" else domain_status.get(rule.domain, "passed")
        comparisons.append(
            {
                "domain": rule.domain,
                "metric": rule.name,
                "direction": rule.direction,
                "baseline": previous,
                "current": observed,
                "ratio": ratio,
                "status": status,
            }
        )

    return {
        "schema_version": "cortexdb.release_regression_dashboard.v1",
        "status": "passed" if not failures else "failed",
        "baseline_release": baseline.get("release", "unknown"),
        "baseline": args.baseline,
        "domain_status": domain_status,
        "comparisons": comparisons,
        "failures": failures,
    }


def write_markdown(report: dict[str, Any], path: Path) -> None:
    lines = [
        "# Release Regression Dashboard",
        "",
        f"Status: `{report['status']}`",
        f"Baseline release: `{report['baseline_release']}`",
        "",
        "| Domain | Metric | Baseline | Current | Ratio | Status |",
        "| --- | --- | ---: | ---: | ---: | --- |",
    ]
    for row in report["comparisons"]:
        lines.append(
            f"| {row['domain']} | `{row['metric']}` | {row['baseline']} | "
            f"{row['current']} | {row['ratio']} | `{row['status']}` |"
        )
    if report["failures"]:
        lines.extend(["", "## Failures", ""])
        lines.extend(f"- {failure}" for failure in report["failures"])
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("\n".join(lines).rstrip() + "\n", encoding="utf-8")


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", default="fixtures/release_regression/history/v0.1.0-core-alpha.5/report.json")
    parser.add_argument("--backup-report", default="target/backup-drill/report.json")
    parser.add_argument("--single-node-report", default="target/single-node-performance/report.json")
    parser.add_argument("--retrieval-report", default="target/retrieval-quality/report.json")
    parser.add_argument("--context-report", default="target/context-pack-quality/report.json")
    parser.add_argument("--verify-report", default="target/verification-quality/report.json")
    parser.add_argument("--api-report", default="target/http-contract-ops/report.json")
    parser.add_argument("--sdk-report", default="target/sdk-e2e-release/report.json")
    parser.add_argument("--report", default="target/release-regression-dashboard/report.json")
    parser.add_argument("--markdown", default="target/release-regression-dashboard/dashboard.md")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        report = build_report(args)
    except Exception as error:  # noqa: BLE001 - dashboard gate reports failures.
        print(f"error: {error}", file=sys.stderr)
        return 1
    repo = repo_root()
    report_path = repo / args.report
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_markdown(report, repo / args.markdown)
    print(f"release regression dashboard: {report_path}")
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
