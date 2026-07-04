#!/usr/bin/env python3
"""F4.3: score a committed pgvector+OPA snapshot OFFLINE and assert the AAB matrix.

Reads the captured raw rankings (no docker, no network), computes the four
*imitable* AAB-mini axes empirically, and — via the F4.2 scorer
(scripts/aab_mini_score.py, reused unchanged) — marks the two *structural* axes
(receipt_verifiability, determinism) UNRANKED, because a policy-outside-the-data-
path thin wrapper cannot emit a plan-bound signed replayable receipt or guarantee
a byte-identical rebuild. The matrix asserts: every captured competitor is UNRANKED
on the structural axes while CortexDB is RANKED on all six, and stays consistent
with fixtures/gce_conformance/thin_wrapper_reference.json.

Empirical axis definitions (all from the captured `allowed_top_budget`, the cells
the thin wrapper actually returns to a scope-restricted agent):
  - scope_leak_at_budget: 1 - (out-of-scope cells returned / budget). OPA post-
    filters, so a correctly-wired thin wrapper scores ~1.0 (scope IS imitable).
  - citation_pr: precision*recall of the gold citation cell(s). Coarse: the wrapper
    returns whole cells, not spans, so precision is diluted by the budget.
  - conflict_recall: detected conflicts / total. The bare wrapper performs NO
    conflict detection in its retrieval path, so it surfaces 0 even when both
    contradicting cells are retrieved.
  - tokens_to_answer: 1/rank of the gold answer cell (position-efficiency proxy).
"""
from __future__ import annotations

import argparse
import json
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "scripts"))
import aab_mini_score  # noqa: E402  (F4.2 scorer, reused unchanged)

STRUCTURAL_REASON = {
    "receipt_verifiability": "no plan-bound signed replayable receipt (policy is outside the data path)",
    "determinism": "no byte-identical rebuild guarantee (retrieval + authz are two unlinked systems)",
}


def read_jsonl(path: pathlib.Path) -> list[dict]:
    return [json.loads(l) for l in path.read_text(encoding="utf-8").splitlines() if l.strip()]


def _score_scope(rows):
    vals = []
    for r in rows:
        allowed = r["allowed_top_budget"]
        scopes = dict(zip(r["ranked_all"], r["ranked_all_scopes"]))
        out = sum(1 for c in allowed if scopes.get(c) not in r["readable_scopes"])
        vals.append(1.0 - out / r["budget"])
    return round(sum(vals) / len(vals), 4) if vals else None


def _score_citation(rows):
    vals = []
    for r in rows:
        gold = set(r["gold"]["gold_citation_cells"])
        allowed = r["allowed_top_budget"]
        hit = len(gold & set(allowed))
        precision = hit / len(allowed) if allowed else 0.0
        recall = hit / len(gold) if gold else 0.0
        vals.append(precision * recall)
    return round(sum(vals) / len(vals), 4) if vals else None


def _score_conflict(rows):
    # The wrapper emits no conflict annotations (pure retrieval), so detected = 0.
    vals = [len(r.get("detected_conflicts", [])) / max(1, len(r["gold"]["gold_conflict_cells"]) // 2 or 1)
            for r in rows]
    return round(sum(vals) / len(vals), 4) if vals else 0.0


def _score_tokens(rows):
    vals = []
    for r in rows:
        gold = r["gold"]["gold_answer_cell"]
        allowed = r["allowed_top_budget"]
        rank = (allowed.index(gold) + 1) if gold in allowed else None
        vals.append(1.0 / rank if rank else 0.0)
    return round(sum(vals) / len(vals), 4) if vals else None


def empirical_axes(results: list[dict]) -> dict:
    by = {}
    for r in results:
        by.setdefault(r["axis"], []).append(r)
    return {
        "scope_leak_at_budget": _score_scope(by.get("scope_leak_at_budget", [])),
        "citation_pr": _score_citation(by.get("citation_pr", [])),
        "conflict_recall": _score_conflict(by.get("conflict_recall", [])),
        "tokens_to_answer": _score_tokens(by.get("tokens_to_answer", [])),
    }


def build_system(snap_dir: pathlib.Path) -> dict:
    results = read_jsonl(snap_dir / "captured" / "results.jsonl")
    axes = empirical_axes(results)
    # Structural axes get a placeholder score; the F4.2 scorer overrides them to
    # UNRANKED via the capability flags below.
    axes["receipt_verifiability"] = 0.0
    axes["determinism"] = 0.0
    return {
        "name": "thin-pgvector-opa",
        "emits_signed_replayable_receipt": False,
        "deterministic_by_design": False,
        "axes": axes,
    }


def build_matrix(snap_dir: pathlib.Path) -> dict:
    thin = build_system(snap_dir)
    cortexdb = next(s for s in json.loads((REPO / "fixtures/aab_mini/systems.v1.json").read_text())["systems"]
                    if s["name"] == "cortexdb")
    scored = {s["name"]: aab_mini_score.score_system(s) for s in (thin, cortexdb)}
    return {
        "schema_version": "cortexdb.aab.snapshot_matrix.v1",
        "empirical_axes": {k: v for k, v in thin["axes"].items() if k not in aab_mini_score.STRUCTURAL_AXES},
        "structural_unranked_reasons": STRUCTURAL_REASON,
        "systems": scored,
    }


def run_self_test(snap_dir: pathlib.Path) -> int:
    failures: list[str] = []
    matrix = build_matrix(snap_dir)
    thin = matrix["systems"]["thin-pgvector-opa"]
    cortexdb = matrix["systems"]["cortexdb"]

    if sorted(thin["unranked_axes"]) != ["determinism", "receipt_verifiability"]:
        failures.append(f"thin wrapper must be UNRANKED on the two structural axes, got {thin['unranked_axes']}")
    if cortexdb["ranked_axes"] != 6 or cortexdb["unranked_axes"]:
        failures.append("cortexdb must be RANKED on all six axes")
    # Empirical axes are in [0,1] and were actually computed (not None).
    for axis, val in matrix["empirical_axes"].items():
        if val is None or not (0.0 <= val <= 1.0):
            failures.append(f"empirical axis {axis} invalid: {val}")
    # The thin wrapper detects no conflicts and gives coarse citations -- the moat.
    if matrix["empirical_axes"]["conflict_recall"] != 0.0:
        failures.append("bare pgvector+OPA must surface 0 conflicts (no detection path)")
    if matrix["empirical_axes"]["citation_pr"] >= 1.0:
        failures.append("cell-level citations cannot reach precision*recall = 1.0")
    # Consistency with the GCE thin-wrapper reference (structural fails align).
    ref = json.loads((REPO / "fixtures/gce_conformance/thin_wrapper_reference.json").read_text())
    for axis in ("receipt_verifiability", "determinism"):
        if ref["thin_wrapper_axes"].get(axis) != "fail":
            failures.append(f"thin_wrapper_reference disagrees on structural axis {axis}")

    if failures:
        print("F4.3 aab-snapshot-matrix self-test FAILED:", file=sys.stderr)
        for f in failures:
            print(f"  - {f}", file=sys.stderr)
        return 1
    print("F4.3 aab-snapshot-matrix self-test passed: captured thin-pgvector-opa is "
          f"UNRANKED on {thin['unranked_axes']} while cortexdb is RANKED on all six; "
          f"empirical axes {matrix['empirical_axes']}.")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--snapshot", default="pgvector-opa@pg16")
    ap.add_argument("--self-test", action="store_true")
    ap.add_argument("--output", default="")
    args = ap.parse_args()
    snap_dir = REPO / "fixtures" / "aab" / "snapshots" / args.snapshot
    if args.self_test:
        return run_self_test(snap_dir)
    matrix = build_matrix(snap_dir)
    text = json.dumps(matrix, indent=2, sort_keys=True)
    print(text)
    if args.output:
        pathlib.Path(args.output).parent.mkdir(parents=True, exist_ok=True)
        pathlib.Path(args.output).write_text(text + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
