#!/usr/bin/env python3
"""Run small class-specific HTTP load scenarios against cortex-server."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.parse
from pathlib import Path
from typing import Any, Callable

from load_smoke_check import (
    context,
    free_port,
    read_cell,
    request_json,
    run_phase,
    search,
    verify,
    wait_for_health,
    write_cell,
)
from perf_latency import latency_summary


WorkloadFn = Callable[[str], tuple[dict[str, Any], list[str]]]


def seeded_writes(base_url: str, count: int, offset: int = 1, scope: str = "load") -> list[str]:
    errors: list[str] = []
    for cell_id in range(offset, offset + count):
        payload = (
            f"scope={scope}\n"
            "status=ready\n"
            "type=fact\n"
            f"source=load-suite-{cell_id}\n\n"
            f"load suite cell {cell_id} budget ready tenant context verify ingest"
        ).encode("utf-8")
        try:
            request_json("POST", f"{base_url}/v1/cell?cell_id={cell_id}", payload)
        except Exception as error:
            errors.append(f"seed_write[{cell_id}]: {error}")
    return errors


def summarize(name: str, latencies: list[float], errors: list[str]) -> dict[str, Any]:
    return {
        "name": name,
        "ok": not errors,
        "latency": latency_summary(latencies),
        "errors": errors,
    }


def read_heavy(base_url: str) -> tuple[dict[str, Any], list[str]]:
    errors = seeded_writes(base_url, 12)
    ids = [(index % 12) + 1 for index in range(60)]
    latencies, read_errors = run_phase("read_heavy", 8, ids, lambda cell_id: read_cell(base_url, cell_id))
    errors.extend(read_errors)
    return summarize("read_heavy", latencies, errors), errors


def write_heavy(base_url: str) -> tuple[dict[str, Any], list[str]]:
    latencies, errors = run_phase(
        "write_heavy",
        8,
        list(range(101, 141)),
        lambda cell_id: write_cell(base_url, cell_id),
    )
    return summarize("write_heavy", latencies, errors), errors


def context_heavy(base_url: str) -> tuple[dict[str, Any], list[str]]:
    errors = seeded_writes(base_url, 20, offset=201)
    latencies: list[float] = []
    for index in range(12):
        try:
            latencies.append(context(base_url))
        except Exception as error:
            errors.append(f"context_heavy[{index}]: {error}")
    return summarize("context_heavy", latencies, errors), errors


def verify_heavy(base_url: str) -> tuple[dict[str, Any], list[str]]:
    errors = seeded_writes(base_url, 10, offset=301)
    latencies: list[float] = []
    for index in range(12):
        try:
            latencies.append(verify(base_url))
        except Exception as error:
            errors.append(f"verify_heavy[{index}]: {error}")
    return summarize("verify_heavy", latencies, errors), errors


def ingest_heavy(base_url: str) -> tuple[dict[str, Any], list[str]]:
    errors: list[str] = []
    latencies: list[float] = []
    for index in range(10):
        body = (
            f"investment project {index} budget ready\n\n"
            f"transport sector row {index} with source references"
        ).encode("utf-8")
        start = time.perf_counter()
        try:
            response = request_json(
                "POST",
                f"{base_url}/v1/ingest/text?scope=load&source=load-suite-{index}.txt",
                body,
            )
            if int(response.get("chunks_ingested", 0)) <= 0:
                errors.append(f"ingest_heavy[{index}]: no chunks ingested: {response}")
            latencies.append((time.perf_counter() - start) * 1000.0)
        except Exception as error:
            errors.append(f"ingest_heavy[{index}]: {error}")
    return summarize("ingest_heavy", latencies, errors), errors


def mixed_tenant(base_url: str) -> tuple[dict[str, Any], list[str]]:
    errors: list[str] = []
    latencies: list[float] = []
    tenants = [f"tenant-{index:02d}" for index in range(50)]
    for index, tenant in enumerate(tenants):
        cell_id = 1_000 + index
        payload = (
            "scope=load\n"
            "status=ready\n"
            "type=fact\n"
            f"source=load-suite-{tenant}-{cell_id}\n\n"
            f"{tenant} load suite cell {cell_id}"
        ).encode("utf-8")
        start = time.perf_counter()
        try:
            request_json(
                "POST",
                f"{base_url}/v1/cell?tenant={urllib.parse.quote(tenant)}&cell_id={cell_id}",
                payload,
            )
            read = request_json(
                "GET",
                f"{base_url}/v1/cell?tenant={urllib.parse.quote(tenant)}&cell_id={cell_id}",
            )
            if not isinstance(read.get("cell"), dict):
                errors.append(f"mixed_tenant[{index}]: missing cell in {tenant}: {read}")
            latencies.append((time.perf_counter() - start) * 1000.0)
        except Exception as error:
            errors.append(f"mixed_tenant[{index}]: {error}")
    report = summarize("mixed_tenant", latencies, errors)
    report["tenant_count"] = len(tenants)
    return report, errors


WORKLOADS: dict[str, WorkloadFn] = {
    "read_heavy": read_heavy,
    "write_heavy": write_heavy,
    "context_heavy": context_heavy,
    "verify_heavy": verify_heavy,
    "ingest_heavy": ingest_heavy,
    "mixed_tenant": mixed_tenant,
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--server", required=True, help="Path to cortex-server binary")
    parser.add_argument("--root", default="target/load-suite", help="Working root")
    parser.add_argument("--report", default="target/load-suite/report.json")
    parser.add_argument("--max-total-ms", type=float, default=45_000.0)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    server = Path(args.server)
    if not server.exists():
        print(f"server binary not found: {server}", file=sys.stderr)
        return 2

    root = Path(args.root)
    shutil.rmtree(root, ignore_errors=True)
    root.mkdir(parents=True, exist_ok=True)
    db_dir = tempfile.mkdtemp(prefix="db-", dir=root)
    port = free_port()
    base_url = f"http://127.0.0.1:{port}"
    start = time.perf_counter()
    process = subprocess.Popen(
        [str(server), db_dir, f"127.0.0.1:{port}"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )

    report: dict[str, Any] = {
        "schema_version": "cortexdb.load_suite.v1",
        "workload_class": "local_http_load_suite",
        "workloads": {},
        "errors": [],
    }
    errors: list[str] = []
    try:
        wait_for_health(base_url)
        for name, workload in WORKLOADS.items():
            workload_report, workload_errors = workload(base_url)
            report["workloads"][name] = workload_report
            errors.extend(workload_errors)

        validation = request_json("GET", f"{base_url}/v1/validate")
        metrics = request_json("GET", f"{base_url}/v1/metrics")
        if validation.get("ok") is not True:
            errors.append(f"validation failed: {validation}")
        if int(metrics.get("request_rejected", 0)) > 0:
            errors.append(f"database_busy/request rejected observed: {metrics}")
        duration_ms = (time.perf_counter() - start) * 1000.0
        if duration_ms > args.max_total_ms:
            errors.append(f"load suite exceeded max-total-ms: {duration_ms:.3f}")
        report.update(
            {
                "ok": not errors,
                "duration_ms": round(duration_ms, 3),
                "validation_ok": validation.get("ok") is True,
                "request_rejected": metrics.get("request_rejected"),
                "errors": errors,
            }
        )
    finally:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:  # pragma: no cover
            process.kill()

    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if errors:
        print("LOAD SUITE CHECK FAILED:")
        for error in errors:
            print(f"  {error}")
        print(f"report: {output}")
        return 1
    print(f"load suite check passed: {output}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
