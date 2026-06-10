#!/usr/bin/env python3
"""Validate official-clean EnterpriseRAG-Bench run artifacts."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from official_clean import ORACLE_FIELDS, assert_clean_retrieval, read_jsonl, write_json
from progress_logging import ProgressLogger


ALLOWED_QUESTION_FIELDS = {"question", "question_id"}
LOGGER = ProgressLogger("official-clean-gate")


def log(message: str) -> None:
    LOGGER.log(message)


def load_json(path: Path) -> dict[str, Any]:
    payload = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError(f"{path} must contain a JSON object")
    return payload


def require(condition: bool, message: str, errors: list[str]) -> None:
    if not condition:
        errors.append(message)


def validate_clean_questions(path: Path, expected_count: int | None) -> tuple[int, list[str]]:
    errors: list[str] = []
    rows = read_jsonl(path)
    if expected_count is not None:
        require(len(rows) == expected_count, f"clean question count {len(rows)} != {expected_count}", errors)
    for index, row in enumerate(rows, 1):
        keys = set(row)
        forbidden = keys - ALLOWED_QUESTION_FIELDS
        oracle = keys & ORACLE_FIELDS
        require(not forbidden, f"question row {index} has extra fields {sorted(forbidden)}", errors)
        require(not oracle, f"question row {index} has oracle fields {sorted(oracle)}", errors)
        require(bool(row.get("question_id")), f"question row {index} missing question_id", errors)
        require(bool(row.get("question")), f"question row {index} missing question", errors)
    return len(rows), errors


def validate_clean_retrieval(path: Path) -> tuple[int, list[str]]:
    rows = read_jsonl(path)
    assert_clean_retrieval(rows)
    return len(rows), []


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-report", type=Path, required=True)
    parser.add_argument("--report", type=Path)
    parser.add_argument("--expected-split")
    parser.add_argument("--expected-questions-file", type=Path)
    parser.add_argument("--require-retrieval", action="store_true")
    parser.add_argument("--log-file", type=Path)
    parser.add_argument("--status-file", type=Path)
    args = parser.parse_args()
    global LOGGER
    LOGGER = ProgressLogger(
        "official-clean-gate",
        log_file=args.log_file,
        status_file=args.status_file,
    )

    log(f"loading run report {args.run_report}")
    LOGGER.status(
        stage="official_clean_gate",
        state="running",
        step=1,
        total_steps=4,
        run_report=str(args.run_report),
        report=str(args.report) if args.report else None,
    )
    run_report = load_json(args.run_report)
    errors: list[str] = []

    if args.expected_split:
        require(
            run_report.get("split_name") == args.expected_split,
            f"split_name {run_report.get('split_name')!r} != {args.expected_split!r}",
            errors,
        )
    if args.expected_questions_file:
        actual = Path(str(run_report.get("questions_file") or ""))
        require(
            actual == args.expected_questions_file,
            f"questions_file {actual} != {args.expected_questions_file}",
            errors,
        )

    clean_questions = Path(str(run_report.get("clean_questions") or ""))
    require(clean_questions.exists(), f"missing clean_questions {clean_questions}", errors)
    question_count = 0
    if clean_questions.exists():
        log(f"validating clean questions {clean_questions}")
        LOGGER.status(
            stage="official_clean_gate",
            state="running",
            step=2,
            total_steps=4,
            clean_questions=str(clean_questions),
        )
        count, question_errors = validate_clean_questions(clean_questions, int(run_report.get("size", 0)) or None)
        question_count = count
        errors.extend(question_errors)

    clean_retrieval = Path(str(run_report.get("clean_retrieval") or ""))
    retrieval_count: int | None = None
    if clean_retrieval.exists():
        log(f"validating clean retrieval {clean_retrieval}")
        LOGGER.status(
            stage="official_clean_gate",
            state="running",
            step=3,
            total_steps=4,
            clean_retrieval=str(clean_retrieval),
        )
        retrieval_count, retrieval_errors = validate_clean_retrieval(clean_retrieval)
        errors.extend(retrieval_errors)
    elif args.require_retrieval:
        errors.append(f"missing clean_retrieval {clean_retrieval}")

    policy = run_report.get("inference_oracle_policy")
    require(isinstance(policy, dict), "missing inference_oracle_policy", errors)
    if isinstance(policy, dict):
        forbidden = set(policy.get("forbidden_question_fields", []))
        require(ORACLE_FIELDS <= forbidden, "inference_oracle_policy does not list all oracle fields", errors)
        require(policy.get("gold_usage") == "judge-only", "gold_usage must be judge-only", errors)

    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.official_clean_gate.v1",
        "status": "passed" if not errors else "failed",
        "run_report": str(args.run_report),
        "split_name": run_report.get("split_name"),
        "questions_file": run_report.get("questions_file"),
        "clean_questions": str(clean_questions),
        "clean_question_count": question_count,
        "clean_retrieval": str(clean_retrieval) if clean_retrieval.exists() else None,
        "clean_retrieval_count": retrieval_count,
        "errors": errors,
    }
    if args.report:
        write_json(args.report, report)
        log(f"wrote official-clean gate report {args.report}")
    log(f"official-clean gate {report['status']} errors={len(errors)}")
    LOGGER.status(
        stage="official_clean_gate",
        state=report["status"],
        step=4,
        total_steps=4,
        clean_question_count=question_count,
        clean_retrieval_count=retrieval_count,
        errors=len(errors),
        report=str(args.report) if args.report else None,
    )
    print(json.dumps(report, indent=2, sort_keys=True))
    return 0 if not errors else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        LOGGER.status(stage="official_clean_gate", state="failed", error=str(error))
        raise
