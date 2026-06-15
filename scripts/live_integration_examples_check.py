#!/usr/bin/env python3
"""Run smoke checks for the three live integration examples."""

from __future__ import annotations

import argparse
import json
import os
import subprocess
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[1]
EXAMPLES = {
    "llm_tool_calling": ROOT / "examples/integrations/llm_tool_calling/demo.py",
    "langchain_retriever": ROOT / "examples/integrations/langchain_retriever/demo.py",
    "memory_chat_agent": ROOT / "examples/integrations/memory_chat_agent/demo.py",
}

REQUIRED_MARKERS = {
    "examples/integrations/README.md": [
        "llm_tool_calling",
        "langchain_retriever",
        "memory_chat_agent",
        "make live-integration-examples-check",
    ],
    "examples/integrations/llm_tool_calling/README.md": [
        "OpenAI",
        "Anthropic",
        "retrieve_context",
        "verify_fact",
        "--self-test",
    ],
    "examples/integrations/langchain_retriever/README.md": [
        "LangChain",
        "CortexRetriever",
        "Document",
        "--self-test",
    ],
    "examples/integrations/memory_chat_agent/README.md": [
        "REMEMBER",
        "TTL 3600 SECONDS",
        "VERIFY FACT",
        "--self-test",
    ],
    ".github/workflows/rust.yml": [
        "make live-integration-examples-check",
    ],
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", default="target/live-integration-examples/report.json")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    failures: list[str] = []
    validate_markers(failures)
    compile_examples(failures)
    cortexdb_bin = build_cli(failures) if not failures else None
    runs = run_examples(cortexdb_bin, failures) if cortexdb_bin else {}

    report = {
        "schema_version": "cortexdb.live_integration_examples.report.v1",
        "status": "failed" if failures else "passed",
        "examples": sorted(EXAMPLES),
        "runs": runs,
        "failures": failures,
    }
    output = ROOT / args.report
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    if failures:
        print(f"live integration examples check failed: {output}")
        for failure in failures:
            print(f"- {failure}")
        return 1
    print(f"live integration examples check passed: {output}")
    return 0


def validate_markers(failures: list[str]) -> None:
    for relative, markers in REQUIRED_MARKERS.items():
        path = ROOT / relative
        if not path.is_file():
            failures.append(f"missing {relative}")
            continue
        text = path.read_text(encoding="utf-8")
        for marker in markers:
            if marker not in text:
                failures.append(f"{relative}: missing {marker!r}")


def compile_examples(failures: list[str]) -> None:
    paths = [str(path.relative_to(ROOT)) for path in EXAMPLES.values()]
    paths.append("examples/integrations/common/cortex_cli.py")
    result = subprocess.run(
        ["python3", "-m", "py_compile", *paths],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    if result.returncode != 0:
        failures.append("py_compile failed:\n" + result.stdout)


def build_cli(failures: list[str]) -> Path | None:
    result = subprocess.run(
        ["cargo", "build", "-p", "cortex-cli", "--bin", "cortexdb"],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
    )
    if result.returncode != 0:
        failures.append("cortexdb build failed:\n" + result.stdout[-4000:])
        return None
    binary = ROOT / "target/debug/cortexdb"
    if not binary.is_file():
        failures.append("missing target/debug/cortexdb after build")
        return None
    return binary


def run_examples(cortexdb_bin: Path, failures: list[str]) -> dict[str, Any]:
    runs: dict[str, Any] = {}
    env = os.environ.copy()
    env["CORTEXDB_BIN"] = str(cortexdb_bin)
    for name, script in EXAMPLES.items():
        db_path = ROOT / "target/live-integration-examples" / name / "db"
        result = subprocess.run(
            ["python3", str(script.relative_to(ROOT)), "--self-test", "--db", str(db_path)],
            cwd=ROOT,
            env=env,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )
        if result.returncode != 0:
            failures.append(f"{name} self-test failed:\n{result.stdout[-4000:]}")
            continue
        try:
            runs[name] = json.loads(result.stdout)
        except json.JSONDecodeError as exc:
            failures.append(f"{name} emitted invalid json: {exc}\n{result.stdout[-1000:]}")
    return runs


if __name__ == "__main__":
    raise SystemExit(main())
