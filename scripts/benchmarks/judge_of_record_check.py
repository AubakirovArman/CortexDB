#!/usr/bin/env python3
"""F2.0: judge-of-record consistency gate.

Asserts the fixed judge-of-record decision is documented AND that the committed
registry cannot violate it:
  1. docs/PUBLIC_CLAIMS_POLICY.md and docs/BENCHMARK_EVIDENCE.md both declare the
     official judge of record (gpt-5.4) and the interim judge (gemini-3.5-flash).
  2. Every committed registry entry that claims `leaderboard_official: true` is
     judged by the official judge of record; an interim-judged entry may never be
     leaderboard-official.

The decision itself is fixed policy (see BENCHMARK_EVIDENCE.md); this gate keeps
the docs and the machine state from drifting apart.

Dependency-free (stdlib only); deterministic; no network, no wall clock, no LLM.
"""

from __future__ import annotations

import argparse
import json
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[2]
POLICY = REPO / "docs" / "PUBLIC_CLAIMS_POLICY.md"
EVIDENCE = REPO / "docs" / "BENCHMARK_EVIDENCE.md"
REGISTRY_DIR = REPO / "fixtures" / "benchmarks" / "registry"

OFFICIAL_JUDGE = {"model": "gpt-5.4", "provider": "openai"}
INTERIM_JUDGE = {"model": "gemini-3.5-flash", "provider": "google"}


def check_docs() -> list[str]:
    errors = []
    for doc in (POLICY, EVIDENCE):
        if not doc.exists():
            errors.append(f"missing {doc.relative_to(REPO)}")
            continue
        text = doc.read_text()
        if OFFICIAL_JUDGE["model"] not in text:
            errors.append(f"{doc.name}: does not name the official judge {OFFICIAL_JUDGE['model']}")
        if INTERIM_JUDGE["model"] not in text:
            errors.append(f"{doc.name}: does not name the interim judge {INTERIM_JUDGE['model']}")
        if "leaderboard" not in text.lower():
            errors.append(f"{doc.name}: does not state the leaderboard rule")
    return errors


def check_entry(bid: str, entry: dict) -> list[str]:
    """An entry that claims leaderboard_official must be judged by the official
    judge of record; an interim judge may never be leaderboard-official."""
    errors = []
    judge = entry.get("judge") or {}
    is_official_judge = (
        judge.get("model") == OFFICIAL_JUDGE["model"] and judge.get("official") is True
    )
    if entry.get("leaderboard_official") is True and not is_official_judge:
        errors.append(
            f"{bid}: leaderboard_official=true but judge is "
            f"{judge.get('model')} (official={judge.get('official')}), not the "
            f"judge of record {OFFICIAL_JUDGE['model']}"
        )
    return errors


def check_registry() -> list[str]:
    errors = []
    if not REGISTRY_DIR.exists():
        return [f"missing registry dir {REGISTRY_DIR.relative_to(REPO)}"]
    for path in sorted(REGISTRY_DIR.glob("*.json")):
        entry = json.loads(path.read_text())
        errors.extend(check_entry(entry.get("benchmark_id", path.stem), entry))
    return errors


def self_test() -> int:
    errors = []
    official_ok = {
        "judge": {"model": "gpt-5.4", "provider": "openai", "official": True},
        "leaderboard_official": True,
    }
    interim_ok = {
        "judge": {"model": "gemini-3.5-flash", "provider": "google", "official": False},
        "leaderboard_official": False,
    }
    interim_bad = {
        "judge": {"model": "gemini-3.5-flash", "provider": "google", "official": False},
        "leaderboard_official": True,
    }
    if check_entry("official_ok", official_ok):
        errors.append("official judge + leaderboard_official must pass")
    if check_entry("interim_ok", interim_ok):
        errors.append("interim judge + non-leaderboard must pass")
    if not check_entry("interim_bad", interim_bad):
        errors.append("interim judge + leaderboard_official must be rejected")
    if errors:
        print("judge-of-record self-test FAILED")
        for e in errors:
            print(f"  {e}")
        return 1
    print(
        "judge-of-record self-test passed: official (gpt-5.4) leaderboard ok; interim "
        "(gemini-3.5-flash) non-leaderboard ok; interim-claiming-leaderboard rejected"
    )
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--self-test", action="store_true")
    args = ap.parse_args()
    if args.self_test:
        return self_test()

    errors = check_docs() + check_registry()
    if errors:
        print("judge-of-record-check FAILED")
        for e in errors:
            print(f"  {e}")
        return 1
    print(
        f"judge-of-record-check passed: docs declare {OFFICIAL_JUDGE['model']} official + "
        f"{INTERIM_JUDGE['model']} interim; no committed registry entry claims "
        "leaderboard-official without the judge of record"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
