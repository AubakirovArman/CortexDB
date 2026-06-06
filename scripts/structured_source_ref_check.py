#!/usr/bin/env python3
"""Validate Structured SourceRef v1 evidence wiring."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


REQUIRED_MARKERS = {
    "engine_metadata": [
        ("crates/cortex-engine/src/query/metadata.rs", "pub row: Option<u32>"),
        ("crates/cortex-engine/src/query/metadata.rs", "strip_prefix(\"row=\")"),
        ("crates/cortex-engine/src/query/metadata.rs", "strip_prefix(\"row_number=\")"),
        ("crates/cortex-engine/src/query/metadata_validation.rs", "row_number="),
    ],
    "ingestion_adapters": [
        ("crates/cortex-engine/src/ingestion/cells.rs", "SourceRefHeaders"),
        ("crates/cortex-engine/src/ingestion/adapters.rs", "json_path: Some(&key)"),
        ("crates/cortex-engine/src/ingestion/adapters.rs", "row: Some(source_row)"),
        ("crates/cortex-engine/src/ingestion/adapters.rs", "cell_range: Some(&cell_range)"),
        ("crates/cortex-engine/src/ingestion/adapters.rs", "page: options.page"),
    ],
    "public_reports": [
        ("crates/cortex-engine/src/ingestion/report.rs", "pub source_url: Option<String>"),
        ("crates/cortex-engine/src/ingestion/report.rs", "pub page: Option<u32>"),
        ("crates/cortex-engine/src/ingestion/report.rs", "pub row: Option<u32>"),
        ("crates/cortex-engine/src/ingestion/report.rs", "pub json_path: Option<String>"),
        ("crates/cortex-server/src/responses.rs", "pub row: Option<u32>"),
        ("crates/cortex-sdk/src/types.rs", "pub row: Option<u32>"),
    ],
    "contracts_and_docs": [
        ("docs/openapi.yaml", "- row"),
        ("docs/CELL_METADATA_MODEL.md", "row=<row_number>"),
        ("docs/INGESTION.md", "json_path=<flattened.path>"),
        ("docs/API.md", "\"row\": 2"),
        ("docs/API_JSON_SCHEMAS.md", "`page`, `row`, `cell_range`, `json_path`"),
    ],
    "dashboard": [
        ("web/dashboard/src/reporting_ingest.js", "\"JSON path\""),
        ("web/dashboard/src/reporting_ingest.js", "item.source_url"),
        ("web/dashboard/src/reporting_ingest.js", "item.row"),
    ],
}


def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as error:
        raise RuntimeError(f"failed to read {path}: {error}") from error


def validate() -> dict[str, object]:
    failures: list[str] = []
    checks: dict[str, bool] = {}
    for name, markers in REQUIRED_MARKERS.items():
        ok = True
        for file_name, marker in markers:
            if marker not in read(Path(file_name)):
                failures.append(f"{name}: marker {marker!r} missing from {file_name}")
                ok = False
        checks[name] = ok
    return {
        "schema_version": 1,
        "status": "passed" if not failures else "failed",
        "failures": failures,
        "checks": checks,
    }


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", required=True)
    args = parser.parse_args(argv)
    try:
        report = validate()
    except RuntimeError as error:
        print(f"structured SourceRef check failed: {error}", file=sys.stderr)
        return 1
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if report["status"] != "passed":
        for failure in report["failures"]:
            print(f"error: {failure}", file=sys.stderr)
        return 1
    print(f"structured SourceRef check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
