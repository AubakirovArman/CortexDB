#!/usr/bin/env python3
"""F1.2: committed benchmark registry — summarize + verify.

The registry (fixtures/benchmarks/registry/*.json) is the single machine-readable
source of truth for every benchmark number CortexDB publishes. Each entry pins a
benchmark to its metrics, its judge, and the committed evidence the numbers come
from. This tool renders the summary table AND enforces the invariants that keep
the registry honest:

  1. json-anchored metrics must MATCH the committed source file at the given path
     (a registry number can never drift from the evidence it cites);
  2. doc-anchored metric values must literally appear in the cited doc;
  3. no entry may claim `leaderboard_official: true` unless its judge is official
     (the anti-overclaim guard — an interim in-house number can never masquerade
     as a leaderboard-comparable result);
  4. every cited source file must exist.

Dependency-free (stdlib only); deterministic; no network, no wall clock.
"""

from __future__ import annotations

import json
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent.parent
REGISTRY = REPO / "fixtures" / "benchmarks" / "registry"
EPS = 1e-9


def dig(obj, dotted: str):
    cur = obj
    for part in dotted.split("."):
        cur = cur[part]
    return cur


def load_entries() -> list[dict]:
    return [json.loads(p.read_text()) for p in sorted(REGISTRY.glob("*.json"))]


def verify(entries: list[dict]) -> list[str]:
    errors: list[str] = []
    for e in entries:
        bid = e.get("benchmark_id", "?")
        # (3) anti-overclaim.
        if e.get("leaderboard_official") and not e.get("judge", {}).get("official"):
            errors.append(f"{bid}: leaderboard_official=true but judge.official is not true")
        src = e.get("source", {})
        path = REPO / src.get("file", "")
        # (4) source exists.
        if not path.exists():
            errors.append(f"{bid}: source file '{src.get('file')}' does not exist")
            continue
        kind = src.get("kind")
        if kind == "json":
            doc = json.loads(path.read_text())
            for m in e.get("metrics", []):
                try:
                    actual = float(dig(doc, m["path"]))
                except Exception as ex:  # noqa: BLE001
                    errors.append(f"{bid}: metric '{m['name']}' path '{m.get('path')}' unreadable: {ex}")
                    continue
                if abs(actual - float(m["value"])) > EPS:
                    errors.append(
                        f"{bid}: metric '{m['name']}' = {m['value']} but source has {actual} "
                        f"(drift from {src['file']})"
                    )
        elif kind == "doc":
            text = path.read_text()
            for m in e.get("metrics", []):
                # Accept the value with or without a trailing zero (0.766 vs 0.7660).
                v = m["value"]
                needles = {str(v), repr(v)}
                if isinstance(v, float):
                    needles.add(f"{v:.4f}")
                    needles.add(f"{v:.4f}".rstrip("0"))
                if not any(n in text for n in needles):
                    errors.append(
                        f"{bid}: doc-anchored metric '{m['name']}' value {v} not found in {src['file']}"
                    )
        else:
            errors.append(f"{bid}: unknown source.kind '{kind}'")
    return errors


def render(entries: list[dict]) -> str:
    lines = ["| Benchmark | Status | Judge (official) | Metrics | Source |",
             "| --- | --- | --- | --- | --- |"]
    for e in sorted(entries, key=lambda x: x["benchmark_id"]):
        judge = e.get("judge", {})
        jm = judge.get("model") or "—"
        jo = "yes" if judge.get("official") else "no"
        metrics = e.get("metrics", [])
        mtxt = ", ".join(f"{m['name']}={m['value']}" for m in metrics) if metrics else "(none claimed)"
        lines.append(
            f"| {e['title']} | {e['status']} | {jm} ({jo}) | {mtxt} | `{e['source']['file']}` |"
        )
    return "\n".join(lines)


def main() -> int:
    args = sys.argv[1:]
    report_path = None
    do_summarize = "--summarize" in args
    for i, a in enumerate(args):
        if a == "--report" and i + 1 < len(args):
            report_path = pathlib.Path(args[i + 1])

    if not REGISTRY.exists():
        print(f"registry dir {REGISTRY} missing")
        return 1
    entries = load_entries()
    errors = verify(entries)
    passed = not errors and len(entries) > 0

    total_metrics = sum(len(e.get("metrics", [])) for e in entries)
    report = {
        "schema_version": "cortexdb.benchmark_registry_check.v1",
        "status": "passed" if passed else "failed",
        "entries": len(entries),
        "metrics": total_metrics,
        "errors": errors,
    }
    if report_path is not None:
        report_path.parent.mkdir(parents=True, exist_ok=True)
        report_path.write_text(json.dumps(report, indent=2) + "\n")

    if do_summarize:
        print(render(entries))
        print()

    if not passed:
        print("benchmark-registry-check FAILED")
        for e in errors:
            print(f"  {e}")
        return 1
    print(
        f"benchmark-registry-check passed: {len(entries)} benchmark(s), {total_metrics} metric(s) "
        f"all trace to committed evidence; no entry overclaims leaderboard-official"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
