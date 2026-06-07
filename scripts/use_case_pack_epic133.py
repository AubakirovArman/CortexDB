#!/usr/bin/env python3
"""Legal policy use-case pack acceptance checks."""

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


def corpus_is_valid(corpus_root: Path, failures: list[str]) -> bool:
    required = {
        "documents": corpus_root / "corpus" / "documents.jsonl",
        "chunks": corpus_root / "corpus" / "chunks.jsonl",
        "queries": corpus_root / "queries" / "queries.jsonl",
        "ground_truth": corpus_root / "queries" / "ground_truth.jsonl",
    }
    ok = True
    for label, path in required.items():
        if not path.is_file():
            failures.append(f"legal_policy_review: missing {label} corpus file {path}")
            ok = False
    if not ok:
        return False
    try:
        counts = {label: len(load_jsonl(path)) for label, path in required.items()}
    except (json.JSONDecodeError, ValueError) as exc:
        failures.append(str(exc))
        return False
    minimums = {"documents": 5, "chunks": 5, "queries": 5, "ground_truth": 5}
    for label, minimum in minimums.items():
        if counts[label] < minimum:
            failures.append(f"legal_policy_review: {label} count below {minimum}: {counts[label]}")
            ok = False
    return ok


def legal_task_coverage(
    pack: dict[str, object],
    readme_text: str,
    failures: list[str],
) -> dict[str, bool]:
    pack_id = str(pack.get("id", ""))
    corpus_root = optional_path(pack, "corpus_path")
    corpus_ok = corpus_root is not None and corpus_root.is_dir() and corpus_is_valid(corpus_root, failures)
    demo_ok = path_exists(pack_id, pack, "demo_path", failures)
    aql_ok = path_exists(pack_id, pack, "aql_examples_path", failures, directory=True)
    citation = str(pack.get("citation_marker", ""))

    search_ok = "search --json" in readme_text and "Search Demo" in readme_text
    context_ok = "RETRIEVE CONTEXT" in str(pack.get("context_aql", "")) and "ContextPack Demo" in readme_text
    contradiction_ok = "VERIFY FACT" in str(pack.get("contradiction_verify_aql", "")) and "Contradiction demo" in readme_text
    citation_ok = bool(
        citation and citation in readme_text and "REQUIRE citations" in str(pack.get("context_aql", ""))
    )

    if demo_ok:
        demo_text = optional_path(pack, "demo_path").read_text(encoding="utf-8")
        for marker in ["search --json", "context --format json", "VERIFY contradiction demo"]:
            require_marker(demo_text, marker, failures, str(optional_path(pack, "demo_path")))

    coverage = {
        "add_corpus": corpus_ok,
        "search_demo": demo_ok and search_ok,
        "contextpack_demo": aql_ok and context_ok,
        "verify_contradiction_demo": contradiction_ok,
        "citation_demo": citation_ok,
    }
    for task, ok in coverage.items():
        if not ok:
            failures.append(f"{pack_id}: Epic 133 task not covered: {task}")
    return coverage
