#!/usr/bin/env python3
"""Support-ticket use-case pack acceptance checks."""

from __future__ import annotations

import json
from pathlib import Path


def optional_path(pack: dict[str, object], key: str) -> Path | None:
    value = str(pack.get(key, "")).strip()
    return Path(value) if value else None


def require_marker(text: str, marker: str, failures: list[str], context: str) -> None:
    if marker not in text:
        failures.append(f"{context}: missing marker {marker!r}")


def path_exists(
    pack_id: str,
    pack: dict[str, object],
    key: str,
    failures: list[str],
    *,
    directory: bool = False,
) -> bool:
    path = optional_path(pack, key)
    if path is None:
        return False
    ok = path.is_dir() if directory else path.is_file()
    expected = "directory" if directory else "file"
    if not ok:
        failures.append(f"{pack_id}: {key} must point to an existing {expected}: {path}")
    return ok


def load_jsonl(path: Path) -> list[dict[str, object]]:
    rows = []
    for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        row = json.loads(line)
        if not isinstance(row, dict):
            raise ValueError(f"{path}:{line_no}: expected object")
        rows.append(row)
    return rows


def corpus_is_valid(pack_id: str, corpus_root: Path, failures: list[str]) -> bool:
    required = {
        "documents": corpus_root / "corpus" / "documents.jsonl",
        "chunks": corpus_root / "corpus" / "chunks.jsonl",
        "queries": corpus_root / "queries" / "queries.jsonl",
        "ground_truth": corpus_root / "queries" / "ground_truth.jsonl",
    }
    missing = [path for path in required.values() if not path.is_file()]
    for path in missing:
        failures.append(f"{pack_id}: missing corpus file {path}")
    if missing:
        return False
    try:
        counts = {label: len(load_jsonl(path)) for label, path in required.items()}
    except (json.JSONDecodeError, ValueError) as exc:
        failures.append(str(exc))
        return False
    minimums = {"documents": 10, "chunks": 20, "queries": 20, "ground_truth": 20}
    ok = True
    for label, minimum in minimums.items():
        if counts[label] < minimum:
            failures.append(f"{pack_id}: {label} count below {minimum}: {counts[label]}")
            ok = False
    return ok


def support_ticket_task_coverage(
    pack: dict[str, object],
    readme_text: str,
    failures: list[str],
) -> dict[str, bool]:
    pack_id = str(pack.get("id", ""))
    corpus_root = optional_path(pack, "corpus_path")
    corpus_ok = corpus_root is not None and corpus_root.is_dir() and corpus_is_valid(
        pack_id, corpus_root, failures
    )
    demo_ok = path_exists(pack_id, pack, "demo_path", failures)
    aql_ok = path_exists(pack_id, pack, "aql_examples_path", failures, directory=True)

    issue_retrieval_ok = (
        corpus_ok
        and "search --json" in readme_text
        and "Customer Issue Retrieval" in readme_text
        and "authentication outage" in str(pack.get("search_query", ""))
    )
    memory_update_ok = (
        "REMEMBER" in str(pack.get("remember_aql", ""))
        and "workflow_result" in str(pack.get("remember_aql", ""))
        and "Memory Update" in readme_text
    )
    resolution_verification_ok = (
        "VERIFY FACT" in str(pack.get("verify_aql", ""))
        and "Resolution Verification" in readme_text
        and "signing key" in str(pack.get("verify_aql", ""))
    )

    if demo_ok:
        demo_text = optional_path(pack, "demo_path").read_text(encoding="utf-8")
        for marker in ["Customer issue retrieval", "Memory update", "Resolution verification"]:
            require_marker(demo_text, marker, failures, str(optional_path(pack, "demo_path")))

    coverage = {
        "customer_issue_retrieval": demo_ok and issue_retrieval_ok,
        "memory_updates": aql_ok and memory_update_ok,
        "resolution_verification": resolution_verification_ok,
    }
    for task, ok in coverage.items():
        if not ok:
            failures.append(f"{pack_id}: Epic 134 task not covered: {task}")
    return coverage
