#!/usr/bin/env python3
"""HTTP load smoke gate for CortexDB.

This is intentionally small and dependency-free. It starts a real
`cortex-server`, drives concurrent write/read/search/context requests, validates
storage, and writes a machine-readable report for CI artifacts.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import json
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Any

from perf_latency import (
    check_latency_thresholds,
    latency_summary,
    load_smoke_latency_thresholds,
)


def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return int(sock.getsockname()[1])


def request_json(method: str, url: str, body: bytes | None = None) -> dict[str, Any]:
    req = urllib.request.Request(url, data=body, method=method)
    if body is not None:
        req.add_header("Content-Type", "text/plain")
    with urllib.request.urlopen(req, timeout=10) as response:
        return json.loads(response.read().decode("utf-8"))


def wait_for_health(base_url: str, timeout: float = 10.0) -> None:
    deadline = time.time() + timeout
    last_error: Exception | None = None
    while time.time() < deadline:
        try:
            health = request_json("GET", f"{base_url}/v1/health")
            if health.get("status") == "ok":
                return
        except Exception as error:  # pragma: no cover - best effort readiness loop
            last_error = error
        time.sleep(0.1)
    raise RuntimeError(f"server did not become healthy: {last_error}")


def timed(callable_obj):
    start = time.perf_counter()
    value = callable_obj()
    elapsed_ms = (time.perf_counter() - start) * 1000.0
    return value, elapsed_ms


def write_cell(base_url: str, cell_id: int) -> float:
    payload = (
        "scope=load\n"
        "status=ready\n"
        "type=fact\n"
        f"source=load-smoke-{cell_id}\n\n"
        f"load smoke cell {cell_id} budget ready"
    ).encode("utf-8")
    _, elapsed = timed(
        lambda: request_json("POST", f"{base_url}/v1/cell?cell_id={cell_id}", payload)
    )
    return elapsed


def read_cell(base_url: str, cell_id: int) -> float:
    response, elapsed = timed(
        lambda: request_json("GET", f"{base_url}/v1/cell?cell_id={cell_id}")
    )
    cell = response.get("cell")
    if not isinstance(cell, dict) or cell.get("cell_id") != cell_id:
        raise AssertionError(f"unexpected cell lookup response for {cell_id}: {response}")
    return elapsed


def search(base_url: str) -> float:
    response, elapsed = timed(
        lambda: request_json(
            "POST",
            f"{base_url}/v1/search?scope=load&mode=keyword&q=budget&limit=10",
            b"",
        )
    )
    if not response.get("results"):
        raise AssertionError(f"search returned no results: {response}")
    return elapsed


def context(base_url: str) -> float:
    statement = (
        'RETRIEVE CONTEXT FOR TASK "load smoke budget" IN BRAIN default '
        'WHERE space = load AND status = "ready" LIMIT 10 CANDIDATES;'
    ).encode("utf-8")
    response, elapsed = timed(
        lambda: request_json("POST", f"{base_url}/v1/context?scope=load", statement)
    )
    if not response.get("cells"):
        raise AssertionError(f"context returned no cells: {response}")
    return elapsed


def verify(base_url: str) -> float:
    statement = 'VERIFY FACT "budget ready" IN BRAIN default;'.encode("utf-8")
    response, elapsed = timed(
        lambda: request_json("POST", f"{base_url}/v1/verify?scope=load", statement)
    )
    if response.get("status") not in {"supported", "mixed"}:
        raise AssertionError(f"verify returned unexpected status: {response}")
    return elapsed


def run_phase(label: str, workers: int, items: list[int], call) -> tuple[list[float], list[str]]:
    latencies: list[float] = []
    errors: list[str] = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=workers) as executor:
        futures = {executor.submit(call, item): item for item in items}
        for future in concurrent.futures.as_completed(futures):
            item = futures[future]
            try:
                latencies.append(float(future.result()))
            except Exception as error:  # pragma: no cover - captured in report
                errors.append(f"{label}[{item}]: {error}")
    return latencies, errors


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--server", required=True, help="Path to cortex-server binary")
    parser.add_argument("--root", default="target/load-smoke", help="Working root")
    parser.add_argument("--report", default="target/load-smoke/report.json")
    parser.add_argument("--cells", type=int, default=100)
    parser.add_argument("--reads", type=int, default=100)
    parser.add_argument("--searches", type=int, default=20)
    parser.add_argument("--contexts", type=int, default=5)
    parser.add_argument("--verifies", type=int, default=5)
    parser.add_argument("--workers", type=int, default=8)
    parser.add_argument("--max-total-ms", type=float, default=30_000.0)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.cells <= 0 or args.reads <= 0 or args.workers <= 0:
        print("--cells, --reads, and --workers must be positive", file=sys.stderr)
        return 2

    server = Path(args.server)
    if not server.exists():
        print(f"server binary not found: {server}", file=sys.stderr)
        return 2

    root = Path(args.root)
    report_path = Path(args.report)
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

    errors: list[str] = []
    report: dict[str, Any] = {
        "schema_version": "cortexdb.load_smoke.v1",
        "cells": args.cells,
        "reads": args.reads,
        "searches": args.searches,
        "contexts": args.contexts,
        "verifies": args.verifies,
        "workers": args.workers,
        "workload_class": "local_http_smoke",
    }

    try:
        wait_for_health(base_url)
        write_latencies, write_errors = run_phase(
            "write",
            args.workers,
            list(range(1, args.cells + 1)),
            lambda cell_id: write_cell(base_url, cell_id),
        )
        errors.extend(write_errors)

        read_ids = [(index % args.cells) + 1 for index in range(args.reads)]
        read_latencies, read_errors = run_phase(
            "read", args.workers, read_ids, lambda cell_id: read_cell(base_url, cell_id)
        )
        errors.extend(read_errors)

        search_latencies = [search(base_url) for _ in range(args.searches)]
        context_latencies = [context(base_url) for _ in range(args.contexts)]
        verify_latencies = [verify(base_url) for _ in range(args.verifies)]

        validation = request_json("GET", f"{base_url}/v1/validate")
        stats = request_json("GET", f"{base_url}/v1/stats")
        metrics = request_json("GET", f"{base_url}/v1/metrics")
        if validation.get("ok") is not True:
            errors.append(f"validation failed: {validation}")
        if int(stats.get("current_seq", 0)) < args.cells:
            errors.append(f"current_seq lower than cells written: {stats}")
        if int(metrics.get("request_rejected", 0)) > 0:
            errors.append(f"database_busy/request rejected observed: {metrics}")

        elapsed_ms = (time.perf_counter() - start) * 1000.0
        if elapsed_ms > args.max_total_ms:
            errors.append(f"load smoke exceeded max-total-ms: {elapsed_ms:.3f}")

        summaries = {
            "write": latency_summary(write_latencies),
            "read": latency_summary(read_latencies),
            "search": latency_summary(search_latencies),
            "context": latency_summary(context_latencies),
            "verify": latency_summary(verify_latencies),
        }
        thresholds = load_smoke_latency_thresholds()
        errors.extend(check_latency_thresholds(summaries, thresholds))
        queue_capacity = max(int(metrics.get("actor_queue_capacity", 0)), 1)
        queue_depth = int(metrics.get("actor_queue_depth", 0))

        report.update(
            {
                "ok": not errors,
                "duration_ms": round(elapsed_ms, 3),
                "validation_ok": validation.get("ok") is True,
                "current_seq": stats.get("current_seq"),
                "latency_thresholds": thresholds,
                "latencies": summaries,
                "actor": {
                    "queue_depth": queue_depth,
                    "queue_capacity": metrics.get("actor_queue_capacity"),
                    "queue_saturation": round(queue_depth / queue_capacity, 6),
                    "database_busy_count": metrics.get("request_rejected"),
                    "request_count": metrics.get("request_count"),
                    "request_duration_ms_total": metrics.get("request_duration_ms_total"),
                },
                "errors": errors,
            }
        )
    finally:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:  # pragma: no cover
            process.kill()

    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True), encoding="utf-8")
    if errors:
        print("LOAD SMOKE CHECK FAILED:")
        for error in errors:
            print(f"  {error}")
        print(f"report: {report_path}")
        return 1
    print(f"load smoke check passed: {report_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
