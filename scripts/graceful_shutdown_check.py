#!/usr/bin/env python3
"""Check that SIGTERM does not lose acknowledged HTTP writes."""

from __future__ import annotations

import argparse
import json
import shutil
import threading
import time
import urllib.error
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

from cortexdb_server_harness import Server, request_json, run, verify_expected


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default="target/graceful-shutdown")
    parser.add_argument("--report", default="target/graceful-shutdown/report.json")
    parser.add_argument("--requests", type=int, default=24)
    parser.add_argument("--shutdown-delay-ms", type=int, default=50)
    parser.add_argument("--max-shutdown-ms", type=int, default=3000)
    parser.add_argument("--min-acked", type=int, default=1)
    return parser.parse_args()


def put_worker(
    base_url: str,
    cell_id: int,
    expected: dict[int, str],
    lock: threading.Lock,
) -> dict:
    payload = f"scope=graceful\nstatus=ready\ngraceful payload {cell_id}"
    try:
        response = request_json(base_url, "POST", f"/v1/cell?cell_id={cell_id}", payload)
        if response.get("cell_id") != cell_id:
            return {"cell_id": cell_id, "acked": False, "error": f"bad response {response}"}
        with lock:
            expected[cell_id] = payload
        return {"cell_id": cell_id, "acked": True}
    except (ConnectionError, TimeoutError, urllib.error.URLError, OSError) as error:
        return {"cell_id": cell_id, "acked": False, "error": str(error)}


def main() -> int:
    args = parse_args()
    repo = Path(__file__).resolve().parents[1]
    root = Path(args.root)
    report_path = Path(args.report)
    db = root / "db"
    log_path = root / "server.log"

    if root.exists():
        shutil.rmtree(root)
    root.mkdir(parents=True)
    report_path.parent.mkdir(parents=True, exist_ok=True)

    run(
        [
            "cargo",
            "build",
            "-p",
            "cortex-server",
            "--bin",
            "cortex-server",
        ],
        repo,
    )

    server = Server(repo, db, log_path)
    expected: dict[int, str] = {}
    lock = threading.Lock()
    results: list[dict] = []

    try:
        server.start()
        with ThreadPoolExecutor(max_workers=min(args.requests, 16)) as pool:
            futures = [
                pool.submit(put_worker, server.base_url, cell_id, expected, lock)
                for cell_id in range(1, args.requests + 1)
            ]
            time.sleep(max(args.shutdown_delay_ms, 0) / 1000)
            shutdown_started = time.monotonic()
            server.terminate()
            shutdown_duration_ms = int((time.monotonic() - shutdown_started) * 1000)
            for future in as_completed(futures):
                results.append(future.result())

        server = Server(repo, db, log_path)
        server.start()
        verified = verify_expected(server.base_url, expected)
        validation = request_json(server.base_url, "GET", "/v1/validate")
        if not validation.get("ok"):
            raise AssertionError(f"validation failed after SIGTERM: {validation}")
    finally:
        server.kill()

    acked = sum(1 for result in results if result.get("acked"))
    unacked = len(results) - acked
    status = (
        "ok"
        if acked >= args.min_acked
        and verified == acked
        and shutdown_duration_ms <= args.max_shutdown_ms
        else "failed"
    )
    report = {
        "status": status,
        "requests": args.requests,
        "shutdown_delay_ms": args.shutdown_delay_ms,
        "shutdown_duration_ms": shutdown_duration_ms,
        "max_shutdown_ms": args.max_shutdown_ms,
        "min_acked": args.min_acked,
        "acked_writes": acked,
        "unacked_or_failed_writes": unacked,
        "cells_verified_after_restart": verified,
        "validation": validation,
        "results": sorted(results, key=lambda item: item["cell_id"]),
    }
    with report_path.open("w", encoding="utf-8") as handle:
        json.dump(report, handle, indent=2, sort_keys=True)
        handle.write("\n")
    print(f"graceful shutdown evidence written to {report_path}")
    print(
        "status={status} acked={acked} verified={verified} unacked={unacked}".format(
            status=status,
            acked=acked,
            verified=verified,
            unacked=unacked,
        )
    )
    return 0 if status == "ok" else 1


if __name__ == "__main__":
    raise SystemExit(main())
