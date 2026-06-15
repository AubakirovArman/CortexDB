#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def read(rel: str) -> str:
    return (ROOT / rel).read_text(encoding="utf-8")


def check_markers(errors: list[str]) -> None:
    required = {
        "docs/AGENT_TRANSACTION_SEMANTICS.md": [
            "Status: accepted research design and guarded prototype.",
            "optimistic same-cell isolation",
            "AgentTransactionRequest",
            "AgentTransactionOutcome::Conflict",
            "make agent-transaction-semantics-check",
        ],
        "crates/cortex-engine/src/options.rs": [
            "pub struct AgentTransactionOptions",
            "pub agent_transactions: AgentTransactionOptions",
        ],
        "crates/cortex-engine/src/config.rs": [
            "CORTEXDB_AGENT_TRANSACTIONS",
        ],
        "crates/cortex-engine/src/agent_transaction.rs": [
            "pub struct AgentTransactionRequest",
            "pub enum AgentTransactionOutcome",
            "commit_agent_transaction",
            "AgentTransactionConflictKind::StaleCell",
            "AgentTransactionConflictKind::TombstonedCell",
        ],
        "crates/cortex-engine/tests/agent_transactions.rs": [
            "concurrent_agent_transactions_conflict_on_stale_same_cell_write",
            "concurrent_agent_transactions_allow_disjoint_cells_and_read_your_writes",
            "agent_transaction_rejects_scope_mismatch_before_commit",
        ],
    }
    for rel, markers in required.items():
        text = read(rel)
        for marker in markers:
            if marker not in text:
                errors.append(f"{rel}: missing marker {marker!r}")


def run(command: list[str]) -> dict[str, object]:
    completed = subprocess.run(
        command,
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    return {
        "command": command,
        "returncode": completed.returncode,
        "output_tail": completed.stdout[-4000:],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", default="target/agent-transaction-semantics/report.json")
    args = parser.parse_args()

    errors: list[str] = []
    check_markers(errors)

    command = [
        "cargo",
        "test",
        "-p",
        "cortex-engine",
        "--test",
        "agent_transactions",
        "--all-features",
    ]
    result = run(command)
    if result["returncode"] != 0:
        errors.append(f"command failed: {' '.join(command)}")

    report = {
        "status": "ok" if not errors else "failed",
        "errors": errors,
        "commands": [result],
    }
    report_path = ROOT / args.report
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    if errors:
        for error in errors:
            print(error, file=sys.stderr)
        return 1
    print(f"agent transaction semantics check passed: {report_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
