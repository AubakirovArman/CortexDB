"""Shared helpers for CortexDB process-level reliability checks."""

from __future__ import annotations

import json
import signal
import socket
import subprocess
import time
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
        self.close_log()

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
        self.close_log()

    def close_log(self) -> None:
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


def put_cell(base_url: str, cell_id: int, payload: str) -> None:
    response = request_json(base_url, "POST", f"/v1/cell?cell_id={cell_id}", payload)
    if response.get("cell_id") != cell_id:
        raise AssertionError(f"unexpected put response: {response}")


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
