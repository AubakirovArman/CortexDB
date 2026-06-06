#!/usr/bin/env python3
"""Check deterministic public output guardrails for engine-facing surfaces."""

from __future__ import annotations

import argparse
import json
from pathlib import Path


DEFAULT_PATHS = [
    "crates/cortex-engine/src/search.rs",
    "crates/cortex-engine/src/search/database.rs",
    "crates/cortex-engine/src/search/persisted.rs",
    "crates/cortex-engine/src/context",
    "crates/cortex-engine/src/verification.rs",
    "crates/cortex-engine/src/verification",
    "crates/cortex-server/src/responses.rs",
    "crates/cortex-cli/src/cli_json.rs",
    "crates/cortex-cli/src/cli_json_types.rs",
]

FORBIDDEN_TOKENS = ["HashMap", "HashSet"]

REQUIRED_DOC_TOKENS = [
    "Status: Epic 32 engine determinism audit.",
    "Search outputs:",
    "ContextPack outputs:",
    "Verification outputs:",
    "make engine-determinism-check",
]

REQUIRED_TEST_TOKENS = [
    "search_results_are_repeatable_and_tie_break_by_cell_id",
    "context_pack_output_is_repeatable_and_snapshotted",
    "verification_report_output_is_repeatable_and_snapshotted",
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--doc", default="docs/ENGINE_DETERMINISM.md")
    parser.add_argument("--test", default="crates/cortex-engine/tests/determinism.rs")
    parser.add_argument("--report", default="target/engine-determinism/report.json")
    parser.add_argument("--path", action="append", dest="paths", default=[])
    return parser.parse_args()


def iter_files(path: Path) -> list[Path]:
    if not path.exists():
        return []
    if path.is_file():
        return [path]
    return sorted(
        candidate
        for candidate in path.rglob("*")
        if candidate.is_file() and candidate.suffix in {".rs", ".md"}
    )


def scan_for_forbidden_tokens(paths: list[Path]) -> list[dict[str, object]]:
    violations: list[dict[str, object]] = []
    for root in paths:
        for path in iter_files(root):
            text = path.read_text(encoding="utf-8", errors="replace")
            for line_no, line in enumerate(text.splitlines(), start=1):
                for token in FORBIDDEN_TOKENS:
                    if token in line:
                        violations.append(
                            {
                                "path": str(path),
                                "line": line_no,
                                "token": token,
                                "text": line.strip(),
                            }
                        )
    return violations


def require_tokens(path: Path, tokens: list[str], label: str) -> list[str]:
    if not path.exists():
        return [f"missing {label}: {path}"]
    text = path.read_text(encoding="utf-8", errors="replace")
    return [f"{label} missing token: {token}" for token in tokens if token not in text]


def main() -> int:
    args = parse_args()
    report_path = Path(args.report)
    scan_paths = [Path(path) for path in (args.paths or DEFAULT_PATHS)]

    errors = []
    errors.extend(require_tokens(Path(args.doc), REQUIRED_DOC_TOKENS, "determinism doc"))
    errors.extend(require_tokens(Path(args.test), REQUIRED_TEST_TOKENS, "determinism test"))

    violations = scan_for_forbidden_tokens(scan_paths)
    errors.extend(
        f"{item['path']}:{item['line']}: forbidden nondeterministic collection {item['token']}"
        for item in violations
    )

    report = {
        "ok": not errors,
        "paths_checked": [str(path) for path in scan_paths],
        "forbidden_tokens": FORBIDDEN_TOKENS,
        "violations": violations,
        "errors": errors,
    }
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    if errors:
        for error in errors:
            print(f"engine determinism check failed: {error}")
        return 1
    print(f"engine determinism check passed: {report_path.resolve()}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
