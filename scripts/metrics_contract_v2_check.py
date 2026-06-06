#!/usr/bin/env python3
"""Validate the CortexDB Metrics Contract v2 surface."""

from __future__ import annotations

import argparse
import json
import re
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
    "backup_latest_age_seconds",
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

HISTOGRAM_FIELDS = [
    "count",
    "sum_ms",
    "le_10_ms",
    "le_50_ms",
    "le_100_ms",
    "le_500_ms",
    "le_1000_ms",
    "gt_1000_ms",
]

PROMETHEUS_SERIES = [
    "cortexdb_current_seq",
    "cortexdb_checkpoint_seq",
    "cortexdb_live_segments",
    "cortexdb_retired_segments",
    "cortexdb_memtable_cells",
    "cortexdb_memtable_versions",
    "cortexdb_memtable_payload_bytes",
    "cortexdb_estimated_memtable_bytes",
    "cortexdb_estimated_index_bytes",
    "cortexdb_estimated_context_pack_bytes",
    "cortexdb_estimated_total_memory_bytes",
    "cortexdb_wal_size_bytes",
    "cortexdb_wal_writer_records",
    "cortexdb_wal_writer_bytes",
    "cortexdb_wal_writer_fsyncs",
    "cortexdb_wal_writer_batches",
    "cortexdb_backup_latest_age_seconds",
    "cortexdb_ann_graph_nodes",
    "cortexdb_ann_total_edges",
    "cortexdb_ann_persisted_segments",
    "cortexdb_actor_queue_depth",
    "cortexdb_actor_queue_capacity",
    "cortexdb_request_count",
    "cortexdb_request_rejected",
    "cortexdb_request_duration_ms_total",
    "cortexdb_ann_search_requests",
    "cortexdb_ann_fallbacks",
    "cortexdb_ann_no_fallback_requests",
    "cortexdb_ann_no_fallback_allowed",
    "cortexdb_ann_no_fallback_blocked",
    "cortexdb_ann_search_latency_ms_bucket",
    "cortexdb_ann_search_latency_ms_count",
    "cortexdb_ann_search_latency_ms_sum",
    "cortexdb_validation_failures",
    "cortexdb_principal_quota_requests_allowed",
    "cortexdb_principal_quota_requests_rejected",
    "cortexdb_principal_quota_body_bytes_allowed",
    "cortexdb_principal_quota_body_bytes_rejected",
    "cortexdb_principal_quota_queue_acquired",
    "cortexdb_principal_quota_queue_rejected",
]


def read(path: str) -> str:
    return Path(path).read_text(encoding="utf-8")


def rust_fields(source: str, struct_name: str) -> list[str]:
    match = re.search(rf"pub struct {struct_name} \{{(?P<body>.*?)\n\}}", source, re.S)
    if not match:
        return []
    return re.findall(r"^\s+pub (\w+):", match.group("body"), re.M)


def yaml_schema_block(source: str, schema_name: str, next_schema: str) -> str:
    start = source.find(f"    {schema_name}:")
    end = source.find(f"    {next_schema}:", start)
    if start < 0 or end < 0:
        return ""
    return source[start:end]


def required_yaml_fields(block: str) -> list[str]:
    required = []
    in_required = False
    for line in block.splitlines():
        if line.strip() == "required:":
            in_required = True
            continue
        if in_required and line.startswith("      - "):
            required.append(line.strip()[2:].strip())
            continue
        if in_required and line.startswith("      properties:"):
            break
    return required


def property_yaml_fields(block: str) -> list[str]:
    return re.findall(r"^        ([a-zA-Z0-9_]+):$", block, re.M)


def missing(expected: list[str], actual: list[str] | str, label: str) -> list[str]:
    if isinstance(actual, str):
        missing_values = [item for item in expected if item not in actual]
    else:
        actual_set = set(actual)
        missing_values = [item for item in expected if item not in actual_set]
    return [f"{label}: missing {item}" for item in missing_values]


def validate() -> dict[str, object]:
    failures: list[str] = []
    responses = read("crates/cortex-server/src/responses.rs")
    openapi = read("docs/openapi.yaml")
    metrics_doc = read("docs/METRICS.md")
    contract_doc = read("docs/METRICS_CONTRACT_V2.md")
    snapshot = read(
        "crates/cortex-server/src/tests/snapshots/"
        "cortex_server__tests__response_snapshot_tests__snapshot_metrics_response.snap"
    )
    server_source = read("crates/cortex-server/src/router.rs") + read("crates/cortex-server/src/lib.rs")

    failures.extend(missing(METRICS_FIELDS, rust_fields(responses, "MetricsResponse"), "MetricsResponse"))
    failures.extend(
        missing(HISTOGRAM_FIELDS, rust_fields(responses, "LatencyHistogramResponse"), "LatencyHistogramResponse")
    )

    metrics_block = yaml_schema_block(openapi, "MetricsResponse", "LatencyHistogramResponse")
    histogram_block = yaml_schema_block(openapi, "LatencyHistogramResponse", "AnnMetricsResponse")
    failures.extend(missing(METRICS_FIELDS, required_yaml_fields(metrics_block), "OpenAPI MetricsResponse required"))
    failures.extend(missing(METRICS_FIELDS, property_yaml_fields(metrics_block), "OpenAPI MetricsResponse properties"))
    failures.extend(
        missing(HISTOGRAM_FIELDS, required_yaml_fields(histogram_block), "OpenAPI LatencyHistogram required")
    )
    failures.extend(
        missing(HISTOGRAM_FIELDS, property_yaml_fields(histogram_block), "OpenAPI LatencyHistogram properties")
    )

    failures.extend(missing(METRICS_FIELDS + HISTOGRAM_FIELDS, metrics_doc, "docs/METRICS.md"))
    failures.extend(missing(METRICS_FIELDS + PROMETHEUS_SERIES, contract_doc, "docs/METRICS_CONTRACT_V2.md"))
    failures.extend(missing(METRICS_FIELDS + HISTOGRAM_FIELDS, snapshot, "metrics response snapshot"))
    failures.extend(missing(PROMETHEUS_SERIES, server_source, "Prometheus source"))

    for marker in [
        "snapshot_metrics_response",
        "snapshot_metrics_response_shape",
        "metrics_prometheus_output_contains_contract_series",
        "metrics-contract-v2-check",
        "format=prometheus",
    ]:
        if marker not in server_source + contract_doc + read("Makefile"):
            failures.append(f"contract marker missing: {marker}")

    return {
        "schema_version": 1,
        "contract": "metrics-contract.v2",
        "status": "passed" if not failures else "failed",
        "failures": failures,
        "json_fields_checked": len(METRICS_FIELDS),
        "histogram_fields_checked": len(HISTOGRAM_FIELDS),
        "prometheus_series_checked": len(PROMETHEUS_SERIES),
    }


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", required=True)
    args = parser.parse_args(argv)
    report = validate()
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"metrics contract v2 check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
