#!/usr/bin/env python3
"""Headless release-gate smoke test for the RAG demo.

This script intentionally avoids FastAPI and vLLM. It proves the CortexDB side
of the demo loop:

data ingest -> search -> AQL retrieve -> ContextPack -> VERIFY FACT -> prompt
"""

from __future__ import annotations

import json
import os
import socket
import subprocess
import sys
import tempfile
import time
import urllib.parse
import urllib.request
from pathlib import Path
from typing import Any


DEMO_DIR = Path(__file__).resolve().parent
REPO_ROOT = DEMO_DIR.parents[1]
DATA_DIR = DEMO_DIR / "data"
EXPECTED_OUTPUT = DEMO_DIR / "expected_output.json"


def fail(message: str) -> None:
    raise RuntimeError(message)


def choose_port() -> int:
    configured = os.environ.get("CORTEX_RAG_DEMO_SMOKE_PORT")
    if configured:
        return int(configured)
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        return int(sock.getsockname()[1])


def request_json(method: str, url: str, body: str = "", timeout: int = 15) -> dict[str, Any]:
    req = urllib.request.Request(
        url,
        data=body.encode("utf-8") if method != "GET" else None,
        headers={"Content-Type": "text/plain"},
        method=method,
    )
    with urllib.request.urlopen(req, timeout=timeout) as response:
        return json.loads(response.read().decode("utf-8"))


def wait_for_health(base_url: str, proc: subprocess.Popen[str], deadline_seconds: float = 15.0) -> None:
    deadline = time.monotonic() + deadline_seconds
    last_error = ""
    while time.monotonic() < deadline:
        if proc.poll() is not None:
            fail(f"cortex-server exited early with status {proc.returncode}")
        try:
            health = request_json("GET", f"{base_url}/v1/health", timeout=2)
            if health.get("status") == "ok":
                return
        except Exception as exc:  # noqa: BLE001 - surfaced below with timeout context.
            last_error = str(exc)
        time.sleep(0.2)
    fail(f"cortex-server did not become healthy: {last_error}")


def build_server() -> Path:
    server = REPO_ROOT / "target" / "debug" / "cortex-server"
    if server.exists():
        return server
    subprocess.run(
        ["cargo", "build", "-q", "--bin", "cortex-server"],
        cwd=REPO_ROOT,
        check=True,
    )
    if not server.exists():
        fail("cargo build finished but target/debug/cortex-server is missing")
    return server


def start_server(db_dir: Path, port: int, log_path: Path) -> tuple[subprocess.Popen[str], str]:
    server = build_server()
    base_url = f"http://127.0.0.1:{port}"
    log = log_path.open("w", encoding="utf-8")
    proc = subprocess.Popen(
        [str(server), str(db_dir), f"127.0.0.1:{port}"],
        cwd=REPO_ROOT,
        stdout=log,
        stderr=subprocess.STDOUT,
        text=True,
    )
    wait_for_health(base_url, proc)
    return proc, base_url


def stop_server(proc: subprocess.Popen[str]) -> None:
    if proc.poll() is not None:
        return
    proc.terminate()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait(timeout=5)


def iter_records() -> list[dict[str, Any]]:
    records: list[dict[str, Any]] = []
    for path in sorted(DATA_DIR.rglob("*.jsonl")):
        with path.open("r", encoding="utf-8") as handle:
            for line_number, line in enumerate(handle, start=1):
                stripped = line.strip()
                if not stripped:
                    continue
                try:
                    record = json.loads(stripped)
                except json.JSONDecodeError as exc:
                    fail(f"invalid JSON in {path}:{line_number}: {exc}")
                payload = record.get("payload_text")
                if not isinstance(payload, str) or not payload.strip():
                    fail(f"missing payload_text in {path}:{line_number}")
                records.append(record)
    return records


def ingest_records(base_url: str) -> int:
    records = iter_records()
    for index, record in enumerate(records, start=1):
        payload = record["payload_text"]
        response = request_json("POST", f"{base_url}/v1/cell?cell_id={index}", payload)
        if response.get("cell_id") != index:
            fail(f"unexpected put response for cell {index}: {response}")
    return len(records)


def post_endpoint(base_url: str, path: str, query: dict[str, str], body: str = "") -> dict[str, Any]:
    encoded = urllib.parse.urlencode(query)
    return request_json("POST", f"{base_url}{path}?{encoded}", body)


def build_prompt(question: str, context_response: dict[str, Any]) -> str:
    cells = context_response.get("cells")
    if not isinstance(cells, list) or not cells:
        fail(f"context response has no cells: {context_response}")
    context_lines = []
    for cell in cells[:6]:
        citation = cell.get("citation") or "unknown-source"
        payload = str(cell.get("payload_text", ""))
        body = payload.split("\n\n", 1)[-1].strip()
        if body:
            context_lines.append(f"[source: {citation}]\n{body}")
    if not context_lines:
        fail("context cells did not contain readable payload bodies")
    context = "\n\n---\n\n".join(context_lines)
    return (
        "System: Answer only from the CortexDB context and cite sources.\n\n"
        f"Context:\n{context}\n\n"
        f"User question: {question}\n"
    )


def load_expected_output() -> dict[str, Any]:
    with EXPECTED_OUTPUT.open("r", encoding="utf-8") as handle:
        value = json.load(handle)
    if not isinstance(value, dict):
        fail(f"{EXPECTED_OUTPUT}: expected JSON object")
    return value


def assert_expected_output(summary: dict[str, Any]) -> None:
    expected = load_expected_output()
    if summary.get("verify_verdict") != expected.get("verify_verdict"):
        fail(f"unexpected verify verdict: {summary}")
    if summary.get("ingested_records") != expected.get("ingested_records"):
        fail(f"unexpected ingested record count: {summary}")
    minimums = {
        "search_results": "min_search_results",
        "aql_cells": "min_aql_cells",
        "context_cells": "min_context_cells",
        "prompt_chars": "min_prompt_chars",
    }
    for field, expected_field in minimums.items():
        if int(summary.get(field, 0)) < int(expected.get(expected_field, 0)):
            fail(f"{field} below expected minimum: {summary}")


def assert_pipeline(base_url: str) -> dict[str, Any]:
    question = "Какой бюджет у Финансового департамента на 2024 год?"
    aql = (
        f'RETRIEVE CONTEXT FOR TASK "{question}" IN BRAIN default '
        'WHERE space = finance AND status = "ready" LIMIT 10 CANDIDATES;'
    )
    verify_aql = (
        'VERIFY FACT "Годовой бюджет Финансового департамента на 2024 год '
        'утверждён в размере 450 млн тенге." IN BRAIN default;'
    )

    search = post_endpoint(
        base_url,
        "/v1/search",
        {"scope": "finance", "q": "Финансовый департамент бюджет"},
    )
    search_results = search.get("results")
    if not isinstance(search_results, list) or not search_results:
        fail(f"search returned no finance results: {search}")

    aql_result = post_endpoint(base_url, "/v1/aql", {"scope": "finance"}, aql)
    aql_cells = aql_result.get("cells")
    if not isinstance(aql_cells, list) or not aql_cells:
        fail(f"AQL returned no cells: {aql_result}")

    context = post_endpoint(base_url, "/v1/context", {"scope": "finance"}, aql)
    prompt = build_prompt(question, context)
    if "450" not in prompt or "budget_approval_2024.xlsx" not in prompt:
        fail("assembled prompt is missing the expected budget evidence")

    verify = post_endpoint(base_url, "/v1/verify", {"scope": "finance"}, verify_aql)
    if verify.get("verdict") != "mixed_evidence":
        fail(f"VERIFY FACT did not return mixed evidence: {verify}")

    return {
        "search_results": len(search_results),
        "aql_cells": len(aql_cells),
        "context_cells": len(context["cells"]),
        "verify_verdict": verify.get("verdict"),
        "prompt_chars": len(prompt),
    }


def main() -> int:
    port = choose_port()
    with tempfile.TemporaryDirectory(prefix="cortexdb-rag-demo-") as tmp:
        tmp_path = Path(tmp)
        proc, base_url = start_server(tmp_path / "db", port, tmp_path / "server.log")
        try:
            ingested = ingest_records(base_url)
            if ingested < 70:
                fail(f"expected at least 70 demo records, ingested {ingested}")
            summary = assert_pipeline(base_url)
            summary["ingested_records"] = ingested
            assert_expected_output(summary)
            print(json.dumps({"ok": True, **summary}, ensure_ascii=False, indent=2))
        finally:
            stop_server(proc)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:  # noqa: BLE001 - CLI smoke should print concise failure.
        print(f"RAG demo smoke failed: {exc}", file=sys.stderr)
        raise SystemExit(1)
