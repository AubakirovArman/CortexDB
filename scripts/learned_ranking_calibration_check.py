#!/usr/bin/env python3
"""Offline learned-ranking calibration gate for CortexDB.

The gate intentionally uses a tiny checked-in fixture. It validates the
contract: train and heldout splits do not overlap, linear weights are selected
only from train rows, and heldout ranking is compared against deterministic
balanced ranking before any profile is accepted.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from pathlib import Path


BASELINE_PROFILE = {"lexical_weight": 2, "vector_weight": 2}
GRID = [
    {"lexical_weight": 1, "vector_weight": 4},
    {"lexical_weight": 2, "vector_weight": 3},
    {"lexical_weight": 2, "vector_weight": 2},
    {"lexical_weight": 3, "vector_weight": 2},
    {"lexical_weight": 4, "vector_weight": 1},
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--fixture", required=True)
    parser.add_argument("--report", required=True)
    parser.add_argument("--min-heldout-mrr-lift-bps", type=int, default=2500)
    parser.add_argument("--min-heldout-win-rate-pct", type=int, default=75)
    parser.add_argument("--compiled-artifact-template")
    parser.add_argument("--compiled-artifact")
    return parser.parse_args()


def read_rows(path: Path) -> list[dict]:
    rows = []
    for index, line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        line = line.strip()
        if not line:
            continue
        try:
            row = json.loads(line)
        except json.JSONDecodeError as error:
            raise ValueError(f"{path}:{index}: invalid JSONL: {error}") from error
        validate_row(row, path, index)
        rows.append(row)
    if not rows:
        raise ValueError(f"{path}: fixture is empty")
    return rows


def validate_row(row: dict, path: Path, line: int) -> None:
    for key in ("split", "question_id", "question_type", "expected_doc_ids", "candidates"):
        if key not in row:
            raise ValueError(f"{path}:{line}: missing {key}")
    if row["split"] not in {"train", "heldout"}:
        raise ValueError(f"{path}:{line}: split must be train or heldout")
    if not row["expected_doc_ids"]:
        raise ValueError(f"{path}:{line}: expected_doc_ids is empty")
    if not row["candidates"]:
        raise ValueError(f"{path}:{line}: candidates is empty")
    for candidate in row["candidates"]:
        for key in ("document_id", "base_score", "lexical_score", "vector_score"):
            if key not in candidate:
                raise ValueError(f"{path}:{line}: candidate missing {key}")


def group_by_type(rows: list[dict]) -> dict[str, list[dict]]:
    grouped: dict[str, list[dict]] = {}
    for row in rows:
        grouped.setdefault(row["question_type"], []).append(row)
    return grouped


def candidate_score(candidate: dict, profile: dict) -> int:
    return (
        int(candidate["base_score"])
        + int(candidate["lexical_score"]) * int(profile["lexical_weight"])
        + int(candidate["vector_score"]) * int(profile["vector_weight"])
    )


def ranked_doc_ids(row: dict, profile: dict) -> list[str]:
    ranked = sorted(
        row["candidates"],
        key=lambda candidate: (-candidate_score(candidate, profile), candidate["document_id"]),
    )
    return [candidate["document_id"] for candidate in ranked]


def reciprocal_rank(row: dict, profile: dict) -> float:
    expected = set(row["expected_doc_ids"])
    for index, document_id in enumerate(ranked_doc_ids(row, profile), start=1):
        if document_id in expected:
            return 1.0 / index
    return 0.0


def mrr_bps(rows: list[dict], profile_by_type: dict[str, dict]) -> int:
    if not rows:
        return 0
    total = 0.0
    for row in rows:
        profile = profile_by_type.get(row["question_type"], BASELINE_PROFILE)
        total += reciprocal_rank(row, profile)
    return round(total * 10_000 / len(rows))


def train_profiles(train_rows: list[dict]) -> dict[str, dict]:
    profiles = {}
    for question_type, rows in group_by_type(train_rows).items():
        best_profile = GRID[0]
        best_mrr = -1
        for profile in GRID:
            score = mrr_bps(rows, {question_type: profile})
            if score > best_mrr:
                best_mrr = score
                best_profile = profile
        profiles[question_type] = {
            **best_profile,
            "train_mrr_bps": best_mrr,
            "train_rows": len(rows),
        }
    return profiles


def split_leakage(train_rows: list[dict], heldout_rows: list[dict]) -> list[str]:
    train_question_ids = {row["question_id"] for row in train_rows}
    heldout_question_ids = {row["question_id"] for row in heldout_rows}
    errors = [
        f"question_id overlap: {sorted(train_question_ids & heldout_question_ids)}"
    ] if train_question_ids & heldout_question_ids else []
    train_docs = {candidate["document_id"] for row in train_rows for candidate in row["candidates"]}
    heldout_docs = {
        candidate["document_id"] for row in heldout_rows for candidate in row["candidates"]
    }
    if train_docs & heldout_docs:
        errors.append(f"document_id overlap: {sorted(train_docs & heldout_docs)}")
    return errors


def build_report(rows: list[dict], args: argparse.Namespace) -> dict:
    train_rows = [row for row in rows if row["split"] == "train"]
    heldout_rows = [row for row in rows if row["split"] == "heldout"]
    errors = []
    if not train_rows:
        errors.append("missing train split")
    if not heldout_rows:
        errors.append("missing heldout split")
    errors.extend(split_leakage(train_rows, heldout_rows))

    learned_profiles = train_profiles(train_rows) if train_rows else {}
    baseline_by_type = {row["question_type"]: BASELINE_PROFILE for row in heldout_rows}
    heldout_baseline_mrr_bps = mrr_bps(heldout_rows, baseline_by_type)
    heldout_learned_mrr_bps = mrr_bps(heldout_rows, learned_profiles)
    lift_bps = heldout_learned_mrr_bps - heldout_baseline_mrr_bps

    details = []
    wins = 0
    regressions = []
    for row in heldout_rows:
        baseline_rr = reciprocal_rank(row, BASELINE_PROFILE)
        profile = learned_profiles.get(row["question_type"], BASELINE_PROFILE)
        learned_rr = reciprocal_rank(row, profile)
        if learned_rr > baseline_rr:
            wins += 1
        if learned_rr < baseline_rr:
            regressions.append(row["question_id"])
        details.append(
            {
                "question_id": row["question_id"],
                "question_type": row["question_type"],
                "baseline_ranked_doc_ids": ranked_doc_ids(row, BASELINE_PROFILE),
                "learned_ranked_doc_ids": ranked_doc_ids(row, profile),
                "expected_doc_ids": row["expected_doc_ids"],
            }
        )

    win_rate_pct = round(wins * 100 / len(heldout_rows)) if heldout_rows else 0
    if lift_bps < args.min_heldout_mrr_lift_bps:
        errors.append(
            f"heldout_mrr_lift_bps {lift_bps} < {args.min_heldout_mrr_lift_bps}"
        )
    if win_rate_pct < args.min_heldout_win_rate_pct:
        errors.append(f"heldout_win_rate_pct {win_rate_pct} < {args.min_heldout_win_rate_pct}")
    if regressions:
        errors.append(f"heldout regressions: {regressions}")

    return {
        "schema_version": "cortexdb.learned_ranking_calibration.v1",
        "fixture": args.fixture,
        "baseline_profile": BASELINE_PROFILE,
        "learned_profiles": learned_profiles,
        "train_rows": len(train_rows),
        "heldout_rows": len(heldout_rows),
        "heldout_baseline_mrr_bps": heldout_baseline_mrr_bps,
        "heldout_learned_mrr_bps": heldout_learned_mrr_bps,
        "heldout_mrr_lift_bps": lift_bps,
        "heldout_win_rate_pct": win_rate_pct,
        "policy_regressions": regressions,
        "details": details,
        "status": "passed" if not errors else "failed",
        "errors": errors,
    }


def artifact_bytes(artifact: dict) -> str:
    def dump(value, indent: int) -> str:
        if isinstance(value, dict):
            lines = ["{"]
            items = list(value.items())
            for index, (key, child) in enumerate(items):
                suffix = "," if index + 1 < len(items) else ""
                lines.append(f'{" " * (indent + 2)}{json.dumps(key)}: {dump(child, indent + 2)}{suffix}')
            lines.append(f'{" " * indent}}}')
            return "\n".join(lines)
        if isinstance(value, list):
            return "[" + ", ".join(dump_scalar(child) for child in value) + "]"
        return dump_scalar(value)

    return dump(artifact, 0) + "\n"


def dump_scalar(value) -> str:
    if value is None:
        return "null"
    return json.dumps(value)


def compile_frozen_artifact(template_path: Path, learned_profiles: dict[str, dict]) -> tuple[dict, str]:
    artifact = json.loads(template_path.read_text(encoding="utf-8"))
    calibration = artifact.get("calibration")
    if not isinstance(calibration, dict):
        raise ValueError(f"{template_path}: missing calibration object")
    for question_type, profile in sorted(learned_profiles.items()):
        if question_type not in calibration:
            continue
        values = list(calibration[question_type])
        if len(values) != 8:
            raise ValueError(f"{template_path}: calibration.{question_type} must have 8 fields")
        values[0] = int(profile["lexical_weight"])
        values[1] = int(profile["vector_weight"])
        calibration[question_type] = values
    text = artifact_bytes(artifact)
    return artifact, hashlib.sha256(text.encode("utf-8")).hexdigest()


def maybe_write_compiled_artifact(report: dict, args: argparse.Namespace) -> None:
    if not args.compiled_artifact:
        return
    if not args.compiled_artifact_template:
        raise ValueError("--compiled-artifact requires --compiled-artifact-template")
    artifact, content_sha256 = compile_frozen_artifact(
        Path(args.compiled_artifact_template),
        report["learned_profiles"],
    )
    output = Path(args.compiled_artifact)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(artifact_bytes(artifact), encoding="utf-8")
    report["compiled_artifact"] = {
        "path": args.compiled_artifact,
        "template": args.compiled_artifact_template,
        "content_sha256": content_sha256,
        "updated_question_types": sorted(report["learned_profiles"]),
    }


def main() -> int:
    args = parse_args()
    try:
        rows = read_rows(Path(args.fixture))
        report = build_report(rows, args)
        maybe_write_compiled_artifact(report, args)
        output = Path(args.report)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    except Exception as error:
        print(error, file=sys.stderr)
        return 1
    summary = {
        "status": report["status"],
        "heldout_baseline_mrr_bps": report["heldout_baseline_mrr_bps"],
        "heldout_learned_mrr_bps": report["heldout_learned_mrr_bps"],
        "heldout_mrr_lift_bps": report["heldout_mrr_lift_bps"],
        "heldout_win_rate_pct": report["heldout_win_rate_pct"],
        "report": args.report,
    }
    print(json.dumps(summary, sort_keys=True))
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
