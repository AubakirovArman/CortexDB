#!/usr/bin/env python3
"""Validate CortexDB ANN reports against an external-reference suite."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

Q16_ONE = 65_535


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except OSError as error:
        raise RuntimeError(f"failed to read {path}: {error}") from error
    except json.JSONDecodeError as error:
        raise RuntimeError(f"{path} is invalid JSON: {error}") from error
    if not isinstance(value, dict):
        raise RuntimeError(f"{path} must contain a JSON object")
    return value


def observed(report: dict[str, Any]) -> dict[str, Any]:
    nested = report.get("observed")
    return nested if isinstance(nested, dict) else report


def int_field(report: dict[str, Any], field: str) -> int | None:
    value = report.get(field)
    return value if isinstance(value, int) else None


def bool_field(report: dict[str, Any], field: str) -> bool | None:
    value = report.get(field)
    return value if isinstance(value, bool) else None


def ratio_q16(observed_value: int, reference_value: int) -> int:
    if reference_value <= 0:
        return -1
    return int((observed_value * Q16_ONE) / reference_value)


def path_from_suite(suite_path: Path, value: str) -> Path:
    path = Path(value)
    if path.is_absolute():
        return path
    cwd_path = Path.cwd() / path
    if cwd_path.exists():
        return cwd_path
    return suite_path.parent / path


def reference_for(suite_path: Path, item: dict[str, Any]) -> dict[str, Any]:
    inline = item.get("reference")
    if isinstance(inline, dict):
        return inline
    reference_report = item.get("reference_report")
    if isinstance(reference_report, str):
        return observed(load_json(path_from_suite(suite_path, reference_report)))
    raise RuntimeError(f"corpus {item.get('name')}: missing reference or reference_report")


def corpus_result(
    suite_path: Path,
    suite: dict[str, Any],
    item: dict[str, Any],
) -> dict[str, Any]:
    name = item.get("name")
    if not isinstance(name, str) or not name:
        raise RuntimeError("every corpus entry needs a non-empty name")
    report_path_value = item.get("cortexdb_report")
    if not isinstance(report_path_value, str) or not report_path_value:
        raise RuntimeError(f"corpus {name}: missing cortexdb_report")
    report_path = path_from_suite(suite_path, report_path_value)
    engine = observed(load_json(report_path))
    reference = reference_for(suite_path, item)
    failures: list[str] = []

    require_production_safe = bool(item.get("require_production_safe", True))
    require_zero_fallback = bool(item.get("require_zero_fallback", True))
    min_upper_layers = int(item.get("min_upper_layers", 1))
    min_upper_graph_edges = int(item.get("min_upper_graph_edges", 1))

    min_mean_recall_ratio_q16 = int(
        item.get(
            "min_mean_recall_ratio_q16",
            suite.get("default_min_mean_recall_ratio_q16", Q16_ONE),
        )
    )
    min_min_recall_ratio_q16 = int(
        item.get(
            "min_min_recall_ratio_q16",
            suite.get("default_min_min_recall_ratio_q16", Q16_ONE),
        )
    )
    max_p95_latency_ratio_q16 = int(
        item.get(
            "max_p95_latency_ratio_q16",
            suite.get("default_max_p95_latency_ratio_q16", Q16_ONE * 2),
        )
    )
    max_p99_latency_ratio_q16 = int(
        item.get(
            "max_p99_latency_ratio_q16",
            suite.get("default_max_p99_latency_ratio_q16", Q16_ONE * 2),
        )
    )
    max_max_latency_ratio_q16 = int(
        item.get(
            "max_max_latency_ratio_q16",
            suite.get("default_max_max_latency_ratio_q16", Q16_ONE * 2),
        )
    )

    mean_recall = int_field(engine, "mean_recall_q16")
    ref_mean_recall = int_field(reference, "mean_recall_q16")
    min_recall = int_field(engine, "min_observed_recall_q16")
    ref_min_recall = int_field(reference, "min_observed_recall_q16")
    p95 = int_field(engine, "p95_latency_nanos")
    ref_p95 = int_field(reference, "p95_latency_nanos")
    p99 = int_field(engine, "p99_latency_nanos")
    ref_p99 = int_field(reference, "p99_latency_nanos")
    max_latency = int_field(engine, "max_latency_nanos")
    ref_max_latency = int_field(reference, "max_latency_nanos")

    if mean_recall is None or ref_mean_recall is None:
        failures.append("mean_recall_q16 missing from engine or reference")
        mean_recall_ratio = -1
    else:
        mean_recall_ratio = ratio_q16(mean_recall, ref_mean_recall)
        if mean_recall_ratio < min_mean_recall_ratio_q16:
            failures.append(
                "mean_recall_ratio_q16 "
                f"{mean_recall_ratio} < {min_mean_recall_ratio_q16}"
            )
    if min_recall is None or ref_min_recall is None:
        failures.append("min_observed_recall_q16 missing from engine or reference")
        min_recall_ratio = -1
    else:
        min_recall_ratio = ratio_q16(min_recall, ref_min_recall)
        if min_recall_ratio < min_min_recall_ratio_q16:
            failures.append(
                "min_observed_recall_ratio_q16 "
                f"{min_recall_ratio} < {min_min_recall_ratio_q16}"
            )

    latency_ratios: dict[str, int] = {}
    for field, value, ref_value, maximum in [
        ("p95_latency_ratio_q16", p95, ref_p95, max_p95_latency_ratio_q16),
        ("p99_latency_ratio_q16", p99, ref_p99, max_p99_latency_ratio_q16),
        ("max_latency_ratio_q16", max_latency, ref_max_latency, max_max_latency_ratio_q16),
    ]:
        if value is None or ref_value is None:
            failures.append(f"{field}: source latency missing from engine or reference")
            latency_ratios[field] = -1
            continue
        observed_ratio = ratio_q16(value, ref_value)
        latency_ratios[field] = observed_ratio
        if observed_ratio > maximum:
            failures.append(f"{field} {observed_ratio} > {maximum}")

    fallback_count = int_field(engine, "fallback_count")
    if require_zero_fallback and fallback_count != 0:
        failures.append(f"fallback_count expected 0, observed {fallback_count}")
    if require_production_safe and bool_field(engine, "production_safe") is not True:
        failures.append("production_safe must be true")
    if (int_field(engine, "upper_layers") or 0) < min_upper_layers:
        failures.append(f"upper_layers below {min_upper_layers}")
    if (int_field(engine, "upper_graph_edges") or 0) < min_upper_graph_edges:
        failures.append(f"upper_graph_edges below {min_upper_graph_edges}")

    return {
        "name": name,
        "status": "passed" if not failures else "failed",
        "cortexdb_report": str(report_path),
        "reference_id": reference.get("reference_id", reference.get("baseline_id", "")),
        "mean_recall_ratio_q16": mean_recall_ratio,
        "min_observed_recall_ratio_q16": min_recall_ratio,
        **latency_ratios,
        "fallback_count": fallback_count,
        "production_safe": bool_field(engine, "production_safe"),
        "failures": failures,
    }


def validate(suite_path: Path) -> dict[str, Any]:
    suite = load_json(suite_path)
    if suite.get("schema_version") != "cortexdb.ann_reference_suite.v1":
        raise RuntimeError("suite schema_version must be cortexdb.ann_reference_suite.v1")
    corpora = suite.get("corpora")
    if not isinstance(corpora, list) or not corpora:
        raise RuntimeError("suite must contain non-empty corpora")
    corpus_reports = [corpus_result(suite_path, suite, item) for item in corpora]
    failures = [
        f"{corpus['name']}: {failure}"
        for corpus in corpus_reports
        for failure in corpus["failures"]
    ]
    min_corpora = int(suite.get("min_corpora", 3))
    if len(corpus_reports) < min_corpora:
        failures.append(f"expected at least {min_corpora} corpora, observed {len(corpus_reports)}")
    return {
        "schema_version": "cortexdb.ann_reference_suite_report.v1",
        "suite_id": suite.get("suite_id", ""),
        "status": "passed" if not failures else "failed",
        "corpus_count": len(corpus_reports),
        "boundary": suite.get(
            "boundary",
            "local external-reference gate; not a global production HNSW claim",
        ),
        "corpora": corpus_reports,
        "failures": failures,
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--suite", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        report = validate(args.suite)
    except RuntimeError as error:
        print(f"ANN reference suite failed: {error}", file=sys.stderr)
        return 1
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(report, sort_keys=True))
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
