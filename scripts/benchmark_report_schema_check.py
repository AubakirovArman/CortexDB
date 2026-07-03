#!/usr/bin/env python3
"""F1.1: validate benchmark reports against the frozen benchmark_report.v1 schema.

Every committed benchmark result must be a well-formed document under
schemas/benchmark_report.v1.schema.json — a leaderboard number that isn't backed
by a schema-valid artifact doesn't ship. Uses a minimal dependency-free validator
(the required-field + type + range subset of JSON Schema that the schema uses) so
the gate has no third-party requirement.
"""

from __future__ import annotations

import json
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
SCHEMA = REPO / "schemas" / "benchmark_report.v1.schema.json"
# Committed benchmark reports that must always conform.
TARGETS = [REPO / "erb-submission" / "official_results.json"]


def _type_ok(value, spec) -> bool:
    types = spec if isinstance(spec, list) else [spec]
    for t in types:
        if t == "object" and isinstance(value, dict):
            return True
        if t == "array" and isinstance(value, list):
            return True
        if t == "integer" and isinstance(value, int) and not isinstance(value, bool):
            return True
        if t == "number" and isinstance(value, (int, float)) and not isinstance(value, bool):
            return True
        if t == "string" and isinstance(value, str):
            return True
        if t == "null" and value is None:
            return True
    return False


def validate(obj, schema, path: str, errors: list[str]) -> None:
    stype = schema.get("type")
    if stype and not _type_ok(obj, stype):
        errors.append(f"{path}: expected type {stype}, got {type(obj).__name__}")
        return
    if isinstance(obj, dict):
        for req in schema.get("required", []):
            if req not in obj:
                errors.append(f"{path}: missing required field '{req}'")
        props = schema.get("properties", {})
        for key, sub in props.items():
            if key in obj:
                validate(obj[key], sub, f"{path}.{key}", errors)
        add = schema.get("additionalProperties")
        if isinstance(add, dict):
            for key, val in obj.items():
                if key not in props:
                    validate(val, add, f"{path}.{key}", errors)
    if isinstance(obj, (int, float)) and not isinstance(obj, bool):
        if "minimum" in schema and obj < schema["minimum"]:
            errors.append(f"{path}: {obj} < minimum {schema['minimum']}")
        if "maximum" in schema and obj > schema["maximum"]:
            errors.append(f"{path}: {obj} > maximum {schema['maximum']}")


def main() -> int:
    report_path = None
    args = sys.argv[1:]
    for i, a in enumerate(args):
        if a == "--report" and i + 1 < len(args):
            report_path = pathlib.Path(args[i + 1])

    schema = json.loads(SCHEMA.read_text())
    checked = []
    all_errors: list[str] = []
    for target in TARGETS:
        if not target.exists():
            all_errors.append(f"{target}: missing")
            continue
        errors: list[str] = []
        validate(json.loads(target.read_text()), schema, target.name, errors)
        checked.append({"file": str(target.relative_to(REPO)), "errors": errors})
        all_errors.extend(errors)

    passed = not all_errors and len(checked) > 0
    report = {
        "schema_version": "cortexdb.benchmark_report_schema_check.v1",
        "status": "passed" if passed else "failed",
        "schema": str(SCHEMA.relative_to(REPO)),
        "checked": checked,
    }
    if report_path is not None:
        report_path.parent.mkdir(parents=True, exist_ok=True)
        report_path.write_text(json.dumps(report, indent=2) + "\n")

    if not passed:
        print("benchmark-report-schema-check FAILED")
        for e in all_errors:
            print("  " + e)
        return 1
    print(f"benchmark-report-schema-check passed: {len(checked)} report(s) conform to benchmark_report.v1")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
