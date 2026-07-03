#!/usr/bin/env python3
"""F2.1: ERB gpt-5.4 re-judge — target + ready/blocked gate.

Before any judge tokens are spent, this gate (a) pins the official re-judge target
(the leaderboard-official ERB judge of record is gpt-5.4 — see F2.0) and (b)
verifies the integrity of the answers to be re-judged: the committed
`erb-submission/answers.jsonl` must match the SHA-256 recorded in
`fixtures/benchmarks/erb/official_rejudge_target.v1.json`. If the answers file
drifted, the gate FAILS — you must not re-judge a changed answer set against the
recorded target.

Then it reports readiness deterministically:
  - READY   — the judge API key env var is set (a real re-judge can proceed).
  - BLOCKED — the key is absent; the gate exits 0 with `status: "blocked"` and
    spends NO tokens. This is the expected state in this environment (no gpt-5.4
    key), and it is not a failure — it is the gate doing its job.

The actual token-spending re-judge is a separate runner
(`official_rejudge.py`) that this gate guards; it only runs when READY.

Dependency-free (stdlib only); deterministic; no network unless a real re-judge
is invoked (which this gate never does).
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib

REPO = pathlib.Path(__file__).resolve().parents[2]
TARGET = REPO / "fixtures" / "benchmarks" / "erb" / "official_rejudge_target.v1.json"


def sha256_of(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def evaluate(target: dict, answers_sha: str, key_present: bool) -> dict:
    """Pure ready/blocked decision. `integrity_ok` gates everything: a drifted
    answers file is a hard FAIL regardless of key presence."""
    integrity_ok = answers_sha == target["expected_answers_sha256"]
    if not integrity_ok:
        status = "integrity_failed"
    elif key_present:
        status = "ready"
    else:
        status = "blocked"
    return {
        "schema_version": "cortexdb.erb.official_rejudge_readiness.v1",
        "judge_of_record": target["judge_of_record"],
        "answers_file": target["answers_file"],
        "expected_answers_sha256": target["expected_answers_sha256"],
        "actual_answers_sha256": answers_sha,
        "integrity_ok": integrity_ok,
        "judge_key_present": key_present,
        "status": status,
    }


def self_test() -> int:
    target = json.loads(TARGET.read_text())
    good = target["expected_answers_sha256"]
    errors = []

    r_ready = evaluate(target, good, key_present=True)
    if r_ready["status"] != "ready":
        errors.append("matching SHA + key present must be READY")

    r_blocked = evaluate(target, good, key_present=False)
    if r_blocked["status"] != "blocked" or not r_blocked["integrity_ok"]:
        errors.append("matching SHA + no key must be BLOCKED (not a failure)")

    r_drift = evaluate(target, "0" * 64, key_present=True)
    if r_drift["status"] != "integrity_failed":
        errors.append("a drifted answers SHA must fail integrity even with a key")

    # The committed answers file must currently match the recorded SHA.
    answers = REPO / target["answers_file"]
    if answers.exists() and sha256_of(answers) != good:
        errors.append(f"committed {target['answers_file']} SHA != recorded target SHA")

    if errors:
        print("erb-official-rejudge-ready self-test FAILED")
        for e in errors:
            print(f"  {e}")
        return 1
    print(
        "erb-official-rejudge-ready self-test passed: ready/blocked/integrity decisions verified; "
        f"committed answers.jsonl matches the recorded target SHA (judge of record "
        f"{target['judge_of_record']['model']})"
    )
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--self-test", action="store_true")
    ap.add_argument("--report", type=pathlib.Path)
    ap.add_argument(
        "--strict-ready",
        action="store_true",
        help="exit non-zero unless READY (for a lane that requires a live judge key)",
    )
    args = ap.parse_args()
    if args.self_test:
        return self_test()

    target = json.loads(TARGET.read_text())
    answers = REPO / target["answers_file"]
    if not answers.exists():
        print(f"missing answers file {answers}")
        return 1
    key_present = bool(os.environ.get(target["judge_env_var"]))
    result = evaluate(target, sha256_of(answers), key_present)

    text = json.dumps(result, indent=2, sort_keys=True) + "\n"
    if args.report is not None:
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(text)
    print(text, end="")

    if result["status"] == "integrity_failed":
        print("FAILED: answers.jsonl drifted from the recorded target SHA; do not re-judge")
        return 1
    if result["status"] == "blocked":
        print(
            f"BLOCKED: no {target['judge_env_var']} — re-judge against "
            f"{target['judge_of_record']['model']} is deferred; no tokens spent (not a failure)"
        )
        return 2 if args.strict_ready else 0
    print(f"READY: re-judge against {target['judge_of_record']['model']} may proceed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
