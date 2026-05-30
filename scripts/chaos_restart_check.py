#!/usr/bin/env python3
"""Repeatable kill/restart chaos check for the CortexDB server."""

from __future__ import annotations

import argparse
import json
import os
import random
import shutil
import signal
import socket
import subprocess
import time
import urllib.error
import urllib.request
from pathlib import Path


def run(cmd: list[str], cwd: Path, *, capture: bool = False) -> str:
    result = subprocess.run(
        cmd,
        cwd=cwd,
        check=True,
        text=True,
        stdout=subprocess.PIPE if capture else None,
        stderr=subprocess.STDOUT if capture else None,
    )
    return result.stdout or ""


def free_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return int(sock.getsockname()[1])


class Server:
    def __init__(self, repo: Path, db: Path, log_path: Path) -> None:
        self.repo = repo
        self.db = db
        self.log_path = log_path
        self.port = free_port()
        self.base_url = f"http://127.0.0.1:{self.port}"
        self.proc: subprocess.Popen[bytes] | None = None
        self.log = None

    def start(self) -> None:
        self.log_path.parent.mkdir(parents=True, exist_ok=True)
        self.log = self.log_path.open("ab")
        self.proc = subprocess.Popen(
            [
                str(self.repo / "target/debug/cortex-server"),
                str(self.db),
                f"127.0.0.1:{self.port}",
            ],
            cwd=self.repo,
            stdout=self.log,
            stderr=subprocess.STDOUT,
        )
        self.wait_ready()

    def wait_ready(self) -> None:
        deadline = time.time() + 10
        while time.time() < deadline:
            if self.proc and self.proc.poll() is not None:
                raise RuntimeError(f"server exited early with {self.proc.returncode}")
            try:
                request_json(self.base_url, "GET", "/v1/health")
                return
            except Exception:
                time.sleep(0.05)
        raise TimeoutError("server did not become ready")

    def kill(self) -> None:
        if not self.proc:
            return
        if self.proc.poll() is None:
            self.proc.kill()
            self.proc.wait(timeout=10)
        if self.log:
            self.log.close()
            self.log = None

    def terminate(self) -> None:
        if not self.proc:
            return
        if self.proc.poll() is None:
            self.proc.send_signal(signal.SIGTERM)
            try:
                self.proc.wait(timeout=3)
            except subprocess.TimeoutExpired:
                self.proc.kill()
                self.proc.wait(timeout=10)
        if self.log:
            self.log.close()
            self.log = None


def request_json(base_url: str, method: str, path: str, body: str | None = None) -> dict:
    data = body.encode("utf-8") if body is not None else None
    request = urllib.request.Request(f"{base_url}{path}", data=data, method=method)
    if body is not None:
        request.add_header("content-type", "text/plain; charset=utf-8")
    with urllib.request.urlopen(request, timeout=5) as response:
        raw = response.read().decode("utf-8")
    return json.loads(raw)


def verify_expected(base_url: str, expected: dict[int, str]) -> int:
    verified = 0
    for cell_id, payload in sorted(expected.items()):
        response = request_json(base_url, "GET", f"/v1/cell?cell_id={cell_id}")
        cell = response.get("cell")
        if not cell:
            raise AssertionError(f"cell {cell_id} missing after restart")
        if cell.get("payload") != payload:
            raise AssertionError(f"cell {cell_id} payload mismatch after restart")
        verified += 1
    return verified


def put_cell(base_url: str, cell_id: int, payload: str) -> None:
    response = request_json(base_url, "POST", f"/v1/cell?cell_id={cell_id}", payload)
    if response.get("cell_id") != cell_id:
        raise AssertionError(f"unexpected put response: {response}")


def action_report(name: str, **fields: object) -> dict:
    report = {"action": name}
    report.update(fields)
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default="target/chaos-restart")
    parser.add_argument("--report", default="target/chaos-restart/report.json")
    parser.add_argument("--steps", type=int, default=24)
    parser.add_argument("--seed", type=int, default=20260530)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    repo = Path(__file__).resolve().parents[1]
    root = Path(args.root)
    report_path = Path(args.report)
    db = root / "db"
    log_path = root / "server.log"
    cli = repo / "target/debug/cortexdb"
    rng = random.Random(args.seed)

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
            "-p",
            "cortex-cli",
            "--bin",
            "cortexdb",
        ],
        repo,
    )

    server = Server(repo, db, log_path)
    expected: dict[int, str] = {}
    actions: list[dict] = []
    kills = 0
    repair_runs = 0
    restarts = 0

    try:
        server.start()
        restarts += 1

        initial_payload = "scope=chaos\nstatus=ready\nchaos payload 1"
        put_cell(server.base_url, 1, initial_payload)
        expected[1] = initial_payload
        actions.append(action_report("put", cell_id=1))

        next_cell_id = 2
        for step in range(args.steps):
            choice = rng.choice(["put", "put", "flush", "compact", "kill_restart"])
            if choice == "put":
                payload = f"scope=chaos\nstatus=ready\nchaos payload {next_cell_id}"
                put_cell(server.base_url, next_cell_id, payload)
                expected[next_cell_id] = payload
                actions.append(action_report("put", step=step, cell_id=next_cell_id))
                next_cell_id += 1
            elif choice == "flush":
                response = request_json(server.base_url, "POST", "/v1/flush")
                actions.append(action_report("flush", step=step, response=response))
            elif choice == "compact":
                response = request_json(server.base_url, "POST", "/v1/compact")
                actions.append(action_report("compact", step=step, response=response))
            else:
                server.kill()
                kills += 1
                unlock = run([str(cli), "unlock", str(db), "--force"], repo, capture=True)
                repair = run([str(cli), "repair", str(db)], repo, capture=True)
                repair_runs += 1
                server = Server(repo, db, log_path)
                server.start()
                restarts += 1
                verified = verify_expected(server.base_url, expected)
                actions.append(
                    action_report(
                        "kill_restart",
                        step=step,
                        unlock_output=unlock.strip(),
                        repair_output=repair.strip(),
                        cells_verified=verified,
                    )
                )

        final_verified = verify_expected(server.base_url, expected)
        final_stats = request_json(server.base_url, "GET", "/v1/stats")
        final_validation = request_json(server.base_url, "GET", "/v1/validate")
        if not final_validation.get("ok"):
            raise AssertionError(f"final validation failed: {final_validation}")
    finally:
        server.kill()

    run([str(cli), "unlock", str(db), "--force"], repo, capture=True)
    repair_output = run([str(cli), "repair", str(db)], repo, capture=True)
    validate_output = run([str(cli), "validate", str(db)], repo, capture=True)

    report = {
        "status": "ok",
        "seed": args.seed,
        "steps": args.steps,
        "server_restarts": restarts,
        "forced_kills": kills,
        "repair_runs_after_forced_kill": repair_runs,
        "cells_expected": len(expected),
        "cells_verified_final": final_verified,
        "final_stats": final_stats,
        "final_validation": final_validation,
        "post_kill_repair_output": repair_output.strip(),
        "post_kill_validate_output": validate_output.strip(),
        "actions": actions,
    }
    with report_path.open("w", encoding="utf-8") as handle:
        json.dump(report, handle, indent=2, sort_keys=True)
        handle.write("\n")
    print(f"chaos restart evidence written to {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
