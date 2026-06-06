#!/usr/bin/env python3
"""Validate Core Alpha observability docs and examples."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


METRICS_FIELDS = [
    "current_seq",
    "checkpoint_seq",
    "live_segments",
    "retired_segments",
    "memtable_cells",
    "memtable_versions",
    "memtable_payload_bytes",
    "estimated_memtable_bytes",
    "estimated_index_bytes",
    "estimated_context_pack_bytes",
    "estimated_total_memory_bytes",
    "wal_size_bytes",
    "wal_writer_records",
    "wal_writer_bytes",
    "wal_writer_fsyncs",
    "wal_writer_batches",
    "ann_graph_nodes",
    "ann_total_edges",
    "ann_persisted_segments",
    "ann_has_checkpoint",
    "ann_has_uncheckpointed_changes",
    "ann_search_requests",
    "ann_fallbacks",
    "ann_no_fallback_requests",
    "ann_no_fallback_allowed",
    "ann_no_fallback_blocked",
    "ann_search_latency_ms",
    "actor_queue_depth",
    "actor_queue_capacity",
    "request_count",
    "request_rejected",
    "request_duration_ms_total",
    "validation_failures",
    "principal_quota_requests_allowed",
    "principal_quota_requests_rejected",
    "principal_quota_body_bytes_allowed",
    "principal_quota_body_bytes_rejected",
    "principal_quota_queue_acquired",
    "principal_quota_queue_rejected",
]

ANN_FIELDS = [
    "graph_nodes",
    "total_edges",
    "persisted_segments",
    "has_checkpoint",
    "has_uncheckpointed_changes",
    "deleted_vectors",
    "rebuild_count",
]

REQUIRED_MARKERS = {
    "prometheus_scrape": [
        ("examples/observability/prometheus.yml", "metrics_path: /v1/metrics"),
        ("examples/observability/prometheus.yml", "format:"),
    ],
    "alerts": [
        ("examples/observability/alerts.yml", "CortexDbWalCheckpointLag"),
        ("examples/observability/alerts.yml", "CortexDbWalGrowth"),
        ("examples/observability/alerts.yml", "CortexDbActorQueuePressure"),
        ("examples/observability/alerts.yml", "CortexDbDatabaseBusy"),
        ("examples/observability/alerts.yml", "CortexDbAnnGraphUnavailable"),
        ("examples/observability/alerts.yml", "CortexDbAnnFallbackRate"),
        ("examples/observability/alerts.yml", "CortexDbAnnNoFallbackBlocked"),
        ("examples/observability/alerts.yml", "CortexDbAnnSearchLatencyP99High"),
        ("examples/observability/alerts.yml", "CortexDbValidationFailures"),
        ("docs/OBSERVABILITY_ALERTS.md", "Suggested Actions"),
    ],
    "docs": [
        ("docs/METRICS.md", "examples/observability/prometheus.yml"),
        ("docs/METRICS.md", "METRICS_CONTRACT_V2.md"),
        ("docs/METRICS.md", "examples/observability/grafana-cortexdb-core-alpha.json"),
        ("docs/METRICS_CONTRACT_V2.md", "metrics-contract-v2-check"),
        ("docs/OBSERVABILITY_EVIDENCE.md", "make observability-check"),
    ],
}


def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as error:
        raise RuntimeError(f"failed to read {path}: {error}") from error


def validate_grafana() -> list[str]:
    failures: list[str] = []
    path = Path("examples/observability/grafana-cortexdb-core-alpha.json")
    dashboard = json.loads(read(path))
    panels = dashboard.get("panels")
    if not isinstance(panels, list) or len(panels) < 5:
        return [f"{path}: expected at least five panels"]
    text = json.dumps(dashboard)
    for marker in [
        "cortexdb_current_seq",
        "cortexdb_checkpoint_seq",
        "cortexdb_wal_size_bytes",
        "cortexdb_ann_graph_nodes",
        "cortexdb_actor_queue_depth",
        "cortexdb_ann_fallbacks",
        "cortexdb_ann_no_fallback_blocked",
        "cortexdb_ann_search_latency_ms_bucket",
        "cortexdb_validation_failures",
    ]:
        if marker not in text:
            failures.append(f"{path}: missing metric {marker}")
    return failures


def validate() -> dict[str, object]:
    failures: list[str] = []
    metrics_doc = read(Path("docs/METRICS.md"))
    for field in METRICS_FIELDS + ANN_FIELDS:
        if field not in metrics_doc:
            failures.append(f"docs/METRICS.md: missing field {field}")

    for name, markers in REQUIRED_MARKERS.items():
        for file_name, marker in markers:
            if marker not in read(Path(file_name)):
                failures.append(f"{name}: marker {marker!r} missing from {file_name}")

    failures.extend(validate_grafana())
    return {
        "schema_version": 1,
        "status": "passed" if not failures else "failed",
        "failures": failures,
        "metrics_fields_checked": len(METRICS_FIELDS),
        "ann_fields_checked": len(ANN_FIELDS),
        "examples": [
            "examples/observability/prometheus.yml",
            "examples/observability/alerts.yml",
            "examples/observability/grafana-cortexdb-core-alpha.json",
        ],
    }


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", required=True)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        report = validate()
    except (RuntimeError, json.JSONDecodeError) as error:
        print(f"observability check failed: {error}", file=sys.stderr)
        return 1
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"observability check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
