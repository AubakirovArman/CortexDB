#!/usr/bin/env python3
"""F4.2: AAB-mini six-axis anti-absorption scorer (single implementation).

Scores a system on the six anti-absorption axes and — crucially — marks the two
axes a thin wrapper is *structurally* incapable of (receipt-verifiability,
determinism) as **UNRANKED**, not zero. That distinction is the whole argument:
a Postgres+pgvector+OPA shim can imitate scoped retrieval and context assembly
(axes 1-4), but it cannot emit a receipt whose access is proven by re-executing a
signed bitmap program, and it cannot guarantee byte-identical determinism —
policy lives outside its data path. So CortexDB is RANKED on all six; the thin
wrapper is honestly UNRANKED on two. Deterministic, dependency-free.
"""

from __future__ import annotations

import json
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parent.parent
FIXTURE = REPO / "fixtures" / "aab_mini" / "systems.v1.json"

# The six axes. The first four are imitable by a thin wrapper; the last two are
# structural — a system without a plan-bound signed receipt / determinism
# guarantee is UNRANKED there, not scored zero.
AXES = [
    "scope_leak_at_budget",   # lower is better (leaked cells at budget); reported as a score
    "citation_pr",            # citation precision*recall, [0,1]
    "conflict_recall",        # surfaced conflicts / total, [0,1]
    "tokens_to_answer",       # answer-token efficiency score, [0,1]
    "receipt_verifiability",  # STRUCTURAL: signed, re-executable receipt
    "determinism",            # STRUCTURAL: byte-identical rebuild
]
STRUCTURAL_AXES = {"receipt_verifiability", "determinism"}
CAPABILITY_FOR_AXIS = {
    "receipt_verifiability": "emits_signed_replayable_receipt",
    "determinism": "deterministic_by_design",
}


def score_system(system: dict) -> dict:
    axes = {}
    for axis in AXES:
        if axis in STRUCTURAL_AXES and not system.get(CAPABILITY_FOR_AXIS[axis], False):
            axes[axis] = {"rank": "UNRANKED", "reason": "structurally incapable"}
        else:
            axes[axis] = {"rank": "RANKED", "score": float(system["axes"][axis])}
    ranked = [a for a, v in axes.items() if v["rank"] == "RANKED"]
    return {
        "system": system["name"],
        "axes": axes,
        "ranked_axes": len(ranked),
        "unranked_axes": sorted(a for a, v in axes.items() if v["rank"] == "UNRANKED"),
    }


def score_all(fixture: dict) -> list[dict]:
    return [score_system(s) for s in sorted(fixture["systems"], key=lambda s: s["name"])]


def self_test() -> int:
    fixture = json.loads(FIXTURE.read_text()) if FIXTURE.exists() else _default_fixture()
    result = score_all(fixture)
    by_name = {r["system"]: r for r in result}

    assert "cortexdb" in by_name, "cortexdb missing"
    assert by_name["cortexdb"]["ranked_axes"] == 6, "CortexDB must be RANKED on all six axes"
    assert by_name["cortexdb"]["unranked_axes"] == [], "CortexDB has no UNRANKED axes"

    thin = next((r for r in result if "pgvector" in r["system"] or "thin" in r["system"]), None)
    assert thin is not None, "thin-wrapper baseline missing"
    assert thin["unranked_axes"] == ["determinism", "receipt_verifiability"], (
        f"thin wrapper must be UNRANKED on exactly the two structural axes, got {thin['unranked_axes']}"
    )
    assert thin["ranked_axes"] == 4, "thin wrapper is RANKED on the four imitable axes"

    # Determinism of the scorer itself.
    assert score_all(fixture) == result, "scorer is not deterministic"
    print(
        "aab-mini self-test passed: cortexdb RANKED 6/6; thin pgvector+OPA UNRANKED on "
        "receipt_verifiability + determinism (structural), RANKED on the four imitable axes"
    )
    return 0


def _default_fixture() -> dict:
    return {
        "systems": [
            {
                "name": "cortexdb",
                "emits_signed_replayable_receipt": True,
                "deterministic_by_design": True,
                "axes": {
                    "scope_leak_at_budget": 1.0,
                    "citation_pr": 0.82,
                    "conflict_recall": 0.9,
                    "tokens_to_answer": 0.8,
                    "receipt_verifiability": 1.0,
                    "determinism": 1.0,
                },
            },
            {
                "name": "thin-pgvector-opa",
                "emits_signed_replayable_receipt": False,
                "deterministic_by_design": False,
                "axes": {
                    "scope_leak_at_budget": 0.9,
                    "citation_pr": 0.7,
                    "conflict_recall": 0.55,
                    "tokens_to_answer": 0.75,
                    "receipt_verifiability": 0.0,
                    "determinism": 0.0,
                },
            },
        ]
    }


def main() -> int:
    args = sys.argv[1:]
    report_path = None
    for i, a in enumerate(args):
        if a == "--report" and i + 1 < len(args):
            report_path = pathlib.Path(args[i + 1])
    if "--self-test" in args:
        return self_test()
    fixture = json.loads(FIXTURE.read_text()) if FIXTURE.exists() else _default_fixture()
    result = score_all(fixture)
    if report_path is not None:
        report_path.parent.mkdir(parents=True, exist_ok=True)
        report_path.write_text(json.dumps({"schema_version": "cortexdb.aab_mini.v1", "systems": result}, indent=2) + "\n")
    print(json.dumps(result, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
