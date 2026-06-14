#!/usr/bin/env python3
"""Audit EnterpriseRAG-Bench files for official-clean oracle boundaries."""

from __future__ import annotations

import argparse
import ast
import json
from pathlib import Path
from typing import Any

from official_clean import ORACLE_FIELDS, read_jsonl, write_json


ROOT = Path(__file__).resolve().parents[2]
SCRIPT_ROOT = ROOT / "scripts/enterprise_rag_bench"

STRICT_CLEAN_FILES = {
    "build_official_clean_vectors.py",
    "official_clean_gate.py",
    "prepare_official_clean_inputs.py",
    "run_official_clean_answers.py",
    "run_official_clean_benchmark.py",
}

GUARD_FILES = {
    "official_clean.py",
    "run_deepseek_answers.py",
}

JUDGE_ONLY_FILES = {
    "run_official_clean_judge.py",
    "run_deepseek_answer_metrics.py",
}

LEGACY_OR_ANALYSIS_HINTS = (
    "analyze",
    "balanced",
    "bottleneck",
    "candidate",
    "check",
    "compare",
    "coverage",
    "dashboard",
    "diagnostics",
    "evaluate",
    "gold",
    "hybrid_rerank",
    "missing",
    "postprocess",
    "preflight",
    "prompts",
    "project_chain",
    "quality",
    "regression",
    "route",
    "routed",
    "selector",
    "slot",
    "summarize",
    "sweep",
    "type_topk",
    "view",
)

ALLOWED_STRICT_FIELD_REFERENCES: dict[str, set[str]] = {
    "official_clean_gate.py": set(ORACLE_FIELDS),
    "prepare_official_clean_inputs.py": set(ORACLE_FIELDS),
    "run_official_clean_benchmark.py": set(ORACLE_FIELDS),
}


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def string_literals(path: Path) -> list[str]:
    try:
        tree = ast.parse(read_text(path))
    except SyntaxError:
        return []
    values: list[str] = []
    for node in ast.walk(tree):
        if isinstance(node, ast.Constant) and isinstance(node.value, str):
            values.append(node.value)
    return values


def referenced_oracle_fields(path: Path) -> set[str]:
    values = string_literals(path)
    refs: set[str] = set()
    for value in values:
        for field in ORACLE_FIELDS:
            if field in value:
                refs.add(field)
    return refs


def classify(path: Path, refs: set[str]) -> str:
    name = path.name
    if name in STRICT_CLEAN_FILES:
        allowed = ALLOWED_STRICT_FIELD_REFERENCES.get(name, set())
        return "official_clean" if refs <= allowed else "official_clean_violation"
    if name in GUARD_FILES:
        return "guard"
    if name in JUDGE_ONLY_FILES:
        return "judge_only"
    if refs and any(hint in name for hint in LEGACY_OR_ANALYSIS_HINTS):
        return "analysis_or_legacy"
    if refs:
        return "unclassified_oracle_reference"
    return "no_oracle_reference"


def validate_jsonl_clean(path: Path) -> dict[str, Any]:
    rows = read_jsonl(path)
    violations: list[dict[str, Any]] = []
    for index, row in enumerate(rows, 1):
        oracle = sorted(set(row) & ORACLE_FIELDS)
        if oracle:
            violations.append({"row": index, "fields": oracle})
    return {
        "path": str(path),
        "rows": len(rows),
        "violations": violations[:25],
        "violation_count": len(violations),
    }


def audit_scripts() -> tuple[list[dict[str, Any]], list[str]]:
    entries: list[dict[str, Any]] = []
    errors: list[str] = []
    for path in sorted(SCRIPT_ROOT.glob("*.py")):
        refs = referenced_oracle_fields(path)
        category = classify(path, refs)
        entry = {
            "file": str(path.relative_to(ROOT)),
            "category": category,
            "oracle_fields": sorted(refs),
        }
        entries.append(entry)
        if category in {"official_clean_violation", "unclassified_oracle_reference"}:
            errors.append(
                f"{path.relative_to(ROOT)} has unapproved oracle references: {sorted(refs)}"
            )
    return entries, errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--clean-questions", type=Path)
    parser.add_argument("--clean-retrieval", type=Path)
    parser.add_argument("--answers-file", type=Path)
    parser.add_argument("--report", type=Path, default=ROOT / "target/enterprise-rag-bench/official-clean/oracle_usage_audit.json")
    args = parser.parse_args()

    scripts, errors = audit_scripts()
    artifact_reports = []
    for path in (args.clean_questions, args.clean_retrieval, args.answers_file):
        if path is None:
            continue
        report = validate_jsonl_clean(path)
        artifact_reports.append(report)
        if report["violation_count"]:
            errors.append(f"{path} contains oracle fields in {report['violation_count']} rows")

    category_counts: dict[str, int] = {}
    for entry in scripts:
        category = str(entry["category"])
        category_counts[category] = category_counts.get(category, 0) + 1

    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.oracle_usage_audit.v1",
        "status": "passed" if not errors else "failed",
        "oracle_fields": sorted(ORACLE_FIELDS),
        "script_category_counts": dict(sorted(category_counts.items())),
        "scripts": scripts,
        "artifacts": artifact_reports,
        "errors": errors,
    }
    write_json(args.report, report)
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0 if not errors else 1


if __name__ == "__main__":
    raise SystemExit(main())
