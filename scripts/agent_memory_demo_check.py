#!/usr/bin/env python3
"""Run the Agent Memory v2 local demo and regression gates."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
from pathlib import Path


DOC_MARKERS = {
    "docs/AGENT_MEMORY.md": [
        "Agent Memory v2",
        "## Epic 141 Agent Memory v2 Contract",
        "Add long-term memory",
        "Add working memory",
        "Add private/shared memory",
        "Add TTL/decay",
        "Add feedback",
        "## Memory Classes",
        "Long-Term Memory",
        "Working Memory",
        "Private And Shared Memory",
        "make agent-memory-demo-check",
        "memory_decay_scores",
        "Feedback is stored",
        "examples/demo/agent_memory",
        "not enterprise RBAC",
    ],
    "examples/demo/agent_memory/README.md": [
        "REMEMBER",
        "durable memory cell with TTL",
        "ContextPack retrieve",
        "VERIFY FACT",
        "make agent-memory-demo-check",
    ],
    "examples/demo/agent_memory/run.sh": [
        "REMEMBER \"Prefer cited budget evidence\"",
        "context --format json",
        "verify --format json",
    ],
    "README.md": [
        "examples/demo/agent_memory/README.md",
    ],
}


def run_cmd(command: list[str]) -> str:
    completed = subprocess.run(command, check=True, text=True, capture_output=True)
    return completed.stdout


def parse_json_output(name: str, output: str, failures: list[str]) -> object:
    try:
        return json.loads(output)
    except json.JSONDecodeError as exc:
        failures.append(f"{name}: invalid json output: {exc}")
        return {}


def validate_docs(failures: list[str]) -> None:
    for file_name, markers in DOC_MARKERS.items():
        path = Path(file_name)
        if not path.is_file():
            failures.append(f"missing {file_name}")
            continue
        text = path.read_text(encoding="utf-8")
        for marker in markers:
            if marker not in text:
                failures.append(f"{file_name}: missing {marker!r}")


def run_demo(failures: list[str]) -> dict[str, object]:
    db_path = Path("target/agent-memory-demo/db")
    if db_path.exists():
        shutil.rmtree(db_path)
    remember = run_cmd(
        [
            "cargo",
            "run",
            "-q",
            "-p",
            "cortex-cli",
            "--",
            "--json",
            "remember",
            str(db_path),
            "project:investments",
            'REMEMBER "Prefer cited budget evidence" IN SCOPE project:investments AS TYPE decision TTL 3600 SECONDS;',
        ]
    )
    context = run_cmd(
        [
            "cargo",
            "run",
            "-q",
            "-p",
            "cortex-cli",
            "--",
            "context",
            "--format",
            "json",
            str(db_path),
            "project:investments",
            'RETRIEVE CONTEXT FOR TASK "memory preference" IN BRAIN default WHERE scope = project:investments AND type = "memory" AND memory_type = "decision" REQUIRE citations LIMIT 10 CANDIDATES;',
        ]
    )
    verify = run_cmd(
        [
            "cargo",
            "run",
            "-q",
            "-p",
            "cortex-cli",
            "--",
            "verify",
            "--format",
            "json",
            str(db_path),
            "project:investments",
            'VERIFY FACT "Prefer cited budget evidence" IN BRAIN default;',
        ]
    )
    remember_json = parse_json_output("remember", remember, failures)
    context_json = parse_json_output("context", context, failures)
    verify_json = parse_json_output("verify", verify, failures)
    combined = "\n".join([remember, context, verify])
    for marker in ["ttl_seconds", "Prefer cited budget evidence", "memory_type=decision", "supported"]:
        if marker not in combined:
            failures.append(f"demo output missing {marker!r}")
    return {
        "db_path": str(db_path),
        "remember_cell_id": remember_json.get("cell_id") if isinstance(remember_json, dict) else None,
        "context_cell_count": len(context_json.get("cells", [])) if isinstance(context_json, dict) else None,
        "verify_verdict": verify_json.get("verdict") if isinstance(verify_json, dict) else None,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", default="target/agent-memory-demo/report.json")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    failures: list[str] = []
    validate_docs(failures)
    test_cmd = "cargo test -p cortex-engine --test memory_tests --test memory_lifecycle_tests --test feedback_tests".split()
    if not failures:
        run_cmd(test_cmd)
    demo = run_demo(failures) if not failures else {}
    report = {
        "schema_version": "cortexdb.agent_memory_demo.report.v2",
        "status": "failed" if failures else "passed",
        "tests": [" ".join(test_cmd)],
        "demo": demo,
        "failures": failures,
    }
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if failures:
        print(f"agent memory demo check failed: {output}")
        for failure in failures:
            print(f"- {failure}")
        return 1
    print(f"agent memory demo check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
