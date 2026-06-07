#!/usr/bin/env python3
"""Investment-project use-case pack acceptance checks."""

from __future__ import annotations

from pathlib import Path


def optional_path(pack: dict[str, object], key: str) -> Path | None:
    value = str(pack.get(key, "")).strip()
    return Path(value) if value else None


def validate_optional_path(
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
    exists = path.is_dir() if directory else path.is_file()
    expected = "directory" if directory else "file"
    if not exists:
        failures.append(f"{pack_id}: {key} must point to an existing {expected}: {path}")
    return exists


def require_marker(text: str, marker: str, failures: list[str], context: str) -> None:
    if marker not in text:
        failures.append(f"{context}: missing marker {marker!r}")


def investment_task_coverage(
    pack: dict[str, object],
    readme_text: str,
    failures: list[str],
) -> dict[str, bool]:
    pack_id = str(pack.get("id", ""))
    demo_ok = validate_optional_path(pack_id, pack, "demo_path", failures)
    aql_ok = validate_optional_path(pack_id, pack, "aql_examples_path", failures, directory=True)
    queries_ok = validate_optional_path(pack_id, pack, "domain_queries_path", failures)
    benchmark_ok = validate_optional_path(pack_id, pack, "benchmark_report_path", failures)

    context_ok = "RETRIEVE CONTEXT" in str(pack.get("context_aql", "")) and "ContextPack" in readme_text
    verify_ok = "VERIFY FACT" in str(pack.get("verify_aql", "")) and "VERIFY" in readme_text
    polished_demo_ok = demo_ok and "Demo" in readme_text and "Benchmark" in readme_text

    benchmark_path = optional_path(pack, "benchmark_report_path")
    if benchmark_path is not None and benchmark_path.is_file():
        benchmark_text = benchmark_path.read_text(encoding="utf-8")
        for marker in ["run_id:", "production_safe:", "Repeatable Checks"]:
            require_marker(benchmark_text, marker, failures, str(benchmark_path))

    coverage = {
        "polish_demo": polished_demo_ok,
        "add_queries": aql_ok and queries_ok,
        "contextpack_examples": context_ok,
        "verify_examples": verify_ok,
        "benchmark_report": benchmark_ok,
    }
    for task, ok in coverage.items():
        if not ok:
            failures.append(f"{pack_id}: Epic 132 task not covered: {task}")
    return coverage
