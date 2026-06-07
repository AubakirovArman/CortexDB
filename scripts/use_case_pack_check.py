#!/usr/bin/env python3
"""Validate and smoke-test CortexDB use-case packs."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
from pathlib import Path

from use_case_pack_epic132 import investment_task_coverage


MANIFEST = Path("examples/use_cases/packs.json")
REQUIRED_PACK_IDS = {
    "legal_policy_review",
    "financial_filing_review",
    "technical_runbook_triage",
    "investment_projects",
}


def load_json(path: Path) -> object:
    return json.loads(path.read_text(encoding="utf-8"))


def load_jsonl(path: Path) -> list[dict[str, object]]:
    rows = []
    for line_no, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        try:
            row = json.loads(line)
        except json.JSONDecodeError as exc:
            raise ValueError(f"{path}:{line_no}: invalid json: {exc}") from exc
        if not isinstance(row, dict):
            raise ValueError(f"{path}:{line_no}: expected object")
        rows.append(row)
    return rows


def run_cmd(command: list[str]) -> str:
    completed = subprocess.run(command, check=True, text=True, capture_output=True)
    return completed.stdout


def require_marker(text: str, marker: str, failures: list[str], context: str) -> None:
    if marker not in text:
        failures.append(f"{context}: missing marker {marker!r}")


def validate_pack(pack: dict[str, object], failures: list[str]) -> dict[str, object]:
    pack_id = str(pack.get("id", ""))
    for key in [
        "title",
        "domain",
        "scope",
        "fixture_path",
        "readme_path",
        "search_query",
        "context_aql",
        "verify_aql",
    ]:
        if not str(pack.get(key, "")).strip():
            failures.append(f"{pack_id}: missing {key}")

    fixture = Path(str(pack.get("fixture_path", "")))
    readme = Path(str(pack.get("readme_path", "")))
    if not fixture.is_file():
        failures.append(f"{pack_id}: missing fixture {fixture}")
        rows: list[dict[str, object]] = []
    else:
        try:
            rows = load_jsonl(fixture)
        except ValueError as exc:
            failures.append(str(exc))
            rows = []

    if len(rows) < 2:
        failures.append(f"{pack_id}: fixture must contain at least 2 cells")
    seen_cell_ids: set[int] = set()
    scope = str(pack.get("scope", ""))
    for index, row in enumerate(rows, 1):
        cell_id = row.get("cell_id")
        payload = row.get("payload")
        if not isinstance(cell_id, int) or cell_id <= 0:
            failures.append(f"{pack_id}: row {index} has invalid cell_id")
        elif cell_id in seen_cell_ids:
            failures.append(f"{pack_id}: duplicate cell_id {cell_id}")
        else:
            seen_cell_ids.add(cell_id)
        if not isinstance(payload, str) or not payload.strip():
            failures.append(f"{pack_id}: row {index} has empty payload")
            continue
        require_marker(payload, f"scope={scope}", failures, f"{pack_id}: row {index}")
        require_marker(payload, "status=ready", failures, f"{pack_id}: row {index}")
        require_marker(payload, "type=fact", failures, f"{pack_id}: row {index}")
        require_marker(payload, "source=", failures, f"{pack_id}: row {index}")

    readme_text = ""
    if not readme.is_file():
        failures.append(f"{pack_id}: missing readme {readme}")
    else:
        readme_text = readme.read_text(encoding="utf-8")
        for marker in [scope, str(fixture), "load-fixture", "context", "verify"]:
            require_marker(readme_text, marker, failures, str(readme))

    markers = pack.get("expected_markers", [])
    if not isinstance(markers, list) or not markers:
        failures.append(f"{pack_id}: expected_markers must be a non-empty list")
        markers = []

    task_coverage = {}
    if pack_id == "investment_projects":
        task_coverage = investment_task_coverage(pack, readme_text, failures)

    return {
        "id": pack_id,
        "domain": pack.get("domain"),
        "scope": scope,
        "fixture": str(fixture),
        "cell_count": len(rows),
        "marker_count": len(markers),
        "task_coverage": task_coverage,
    }


def smoke_pack(pack: dict[str, object], failures: list[str]) -> dict[str, object]:
    pack_id = str(pack["id"])
    db_path = Path("target/use-case-packs") / pack_id.replace("_", "-") / "db"
    if db_path.exists():
        shutil.rmtree(db_path)
    fixture = Path(str(pack["fixture_path"]))
    fixture_root = fixture.parent if fixture.name == "cells.jsonl" else fixture
    scope = str(pack["scope"])
    run_cmd(["cargo", "run", "-q", "-p", "cortex-cli", "--", "load-fixture", str(db_path), str(fixture_root)])
    outputs = {
        "search": run_cmd(
            [
                "cargo",
                "run",
                "-q",
                "-p",
                "cortex-cli",
                "--",
                "search",
                "--json",
                str(db_path),
                scope,
                str(pack["search_query"]),
            ]
        ),
        "context": run_cmd(
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
                scope,
                str(pack["context_aql"]),
            ]
        ),
        "verify": run_cmd(
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
                scope,
                str(pack["verify_aql"]),
            ]
        ),
    }
    combined = "\n".join(outputs.values())
    for marker in pack.get("expected_markers", []):
        if str(marker) not in combined:
            failures.append(f"{pack_id}: smoke output missing {marker!r}")
    for name, output in outputs.items():
        try:
            json.loads(output)
        except json.JSONDecodeError as exc:
            failures.append(f"{pack_id}: {name} did not emit JSON: {exc}")
    return {"id": pack_id, "db_path": str(db_path), "commands": sorted(outputs)}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--report", default="target/use-case-packs/report.json")
    parser.add_argument("--skip-smoke", action="store_true")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    failures: list[str] = []
    if not MANIFEST.is_file():
        failures.append(f"missing manifest {MANIFEST}")
        packs: list[dict[str, object]] = []
    else:
        data = load_json(MANIFEST)
        if not isinstance(data, dict) or data.get("schema_version") != "cortexdb.use_case_packs.v1":
            failures.append("manifest schema_version must be cortexdb.use_case_packs.v1")
        raw_packs = data.get("packs") if isinstance(data, dict) else None
        packs = raw_packs if isinstance(raw_packs, list) else []
        if not packs:
            failures.append("manifest must contain packs")

    pack_ids = {str(pack.get("id", "")) for pack in packs if isinstance(pack, dict)}
    if pack_ids != REQUIRED_PACK_IDS:
        failures.append(f"pack ids mismatch: expected {sorted(REQUIRED_PACK_IDS)}, got {sorted(pack_ids)}")

    validated = [validate_pack(pack, failures) for pack in packs if isinstance(pack, dict)]
    smoke = []
    if not args.skip_smoke and not failures:
        for pack in packs:
            if isinstance(pack, dict):
                smoke.append(smoke_pack(pack, failures))

    report = {
        "schema_version": "cortexdb.use_case_packs.report.v1",
        "status": "failed" if failures else "passed",
        "manifest": str(MANIFEST),
        "packs": validated,
        "smoke": smoke,
        "failures": failures,
    }
    output = Path(args.report)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if failures:
        print(f"use-case pack check failed: {output}")
        for failure in failures:
            print(f"- {failure}")
        return 1
    print(f"use-case pack check passed: {output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
