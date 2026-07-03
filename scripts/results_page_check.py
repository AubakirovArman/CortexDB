#!/usr/bin/env python3
"""F5.1: machine-verifiable results page.

docs/RESULTS.md states benchmark headline numbers. Each headline carries a
machine-checkable annotation:

    <!-- verify: <committed-json-path> :: <dotted.key.path> == <expected> -->

This checker parses every such annotation, reads the value from the committed
artifact, and fails if any claimed number does not match its source. So a
published result can never drift from the committed evidence that produced it —
the page is verified, not asserted.
"""

from __future__ import annotations

import json
import pathlib
import re
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
RESULTS = REPO / "docs" / "RESULTS.md"
ANNOT = re.compile(
    r"<!--\s*verify:\s*(?P<path>[^\s:]+)\s*::\s*(?P<key>[\w.]+)\s*==\s*(?P<val>[-\d.]+)\s*-->"
)


def dig(obj, dotted: str):
    cur = obj
    for part in dotted.split("."):
        if isinstance(cur, list):
            cur = cur[int(part)]
        else:
            cur = cur[part]
    return cur


def main() -> int:
    report_path = None
    args = sys.argv[1:]
    for i, a in enumerate(args):
        if a == "--report" and i + 1 < len(args):
            report_path = pathlib.Path(args[i + 1])

    if not RESULTS.exists():
        print("docs/RESULTS.md missing")
        return 1
    text = RESULTS.read_text(encoding="utf-8")
    checks = list(ANNOT.finditer(text))
    results = []
    ok = True
    for m in checks:
        path, key, expected = m.group("path"), m.group("key"), float(m.group("val"))
        artifact = REPO / path
        entry = {"path": path, "key": key, "expected": expected}
        try:
            actual = float(dig(json.loads(artifact.read_text(encoding="utf-8")), key))
            entry["actual"] = actual
            entry["match"] = abs(actual - expected) < 1e-6
        except Exception as e:  # noqa: BLE001
            entry["error"] = str(e)
            entry["match"] = False
        ok = ok and entry["match"]
        results.append(entry)

    passed = ok and len(checks) > 0
    report = {
        "schema_version": "cortexdb.results_page.report.v1",
        "status": "passed" if passed else "failed",
        "checks": results,
        "verified_claims": len(checks),
    }
    if report_path is not None:
        report_path.parent.mkdir(parents=True, exist_ok=True)
        report_path.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")

    if not passed:
        print(f"results-page-check FAILED ({len(checks)} claims)")
        for e in results:
            if not e.get("match"):
                print(f"  MISMATCH {e['path']}::{e['key']} expected {e['expected']} got {e.get('actual', e.get('error'))}")
        return 1
    print(f"results-page-check passed: {len(checks)} headline claims verified against committed evidence")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
