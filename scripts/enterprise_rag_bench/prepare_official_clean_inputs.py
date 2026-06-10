#!/usr/bin/env python3
"""Prepare oracle-free EnterpriseRAG-Bench question/retrieval JSONL files."""

from __future__ import annotations

import argparse
import json
from pathlib import Path

from official_clean import (
    ORACLE_FIELDS,
    clean_questions,
    clean_retrieval,
    read_jsonl,
    write_json,
    write_jsonl,
)
from progress_logging import ProgressLogger


LOGGER = ProgressLogger("official-clean-prepare")


def log(message: str) -> None:
    LOGGER.log(message)


def stripped_counts(rows: list[dict[str, object]]) -> dict[str, int]:
    counts = {field: 0 for field in sorted(ORACLE_FIELDS)}
    for row in rows:
        for field in counts:
            if field in row:
                counts[field] += 1
    return {field: count for field, count in counts.items() if count}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--output-questions", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--limit", type=int)
    parser.add_argument("--retrieval-file", type=Path)
    parser.add_argument("--output-retrieval", type=Path)
    parser.add_argument("--log-file", type=Path)
    parser.add_argument("--status-file", type=Path)
    args = parser.parse_args()
    global LOGGER
    LOGGER = ProgressLogger(
        "official-clean-prepare",
        log_file=args.log_file,
        status_file=args.status_file,
    )

    if args.limit is not None and args.limit <= 0:
        parser.error("--limit must be positive")
    if bool(args.retrieval_file) != bool(args.output_retrieval):
        parser.error("--retrieval-file and --output-retrieval must be provided together")

    stage = "sanitize_retrieval" if args.retrieval_file else "prepare"
    total_steps = 4 if args.retrieval_file else 3

    LOGGER.step(
        stage=stage,
        state="running",
        step=1,
        total_steps=total_steps,
        message=f"load questions from {args.questions_file}",
    )
    try:
        log(f"load questions {args.questions_file}")
        question_rows = read_jsonl(args.questions_file)
        log(f"loaded question rows={len(question_rows)} limit={args.limit}")
        clean_question_rows = clean_questions(question_rows, args.limit)
        write_jsonl(args.output_questions, clean_question_rows)
        log(f"wrote clean questions rows={len(clean_question_rows)} path={args.output_questions}")
        LOGGER.step(
            stage=stage,
            state="running",
            step=2,
            total_steps=total_steps,
            message=f"wrote clean questions to {args.output_questions}",
            input_questions=len(question_rows),
            output_questions=len(clean_question_rows),
        )

        report = {
            "schema_version": "cortexdb.enterprise_rag_bench.official_clean_inputs.v1",
            "questions_file": str(args.questions_file),
            "output_questions": str(args.output_questions),
            "input_questions": len(question_rows),
            "output_questions_count": len(clean_question_rows),
            "question_oracle_fields_stripped": stripped_counts(question_rows),
        }

        if args.retrieval_file and args.output_retrieval:
            log(f"load retrieval {args.retrieval_file}")
            retrieval_rows = read_jsonl(args.retrieval_file)
            log(f"loaded retrieval rows={len(retrieval_rows)}")
            clean_retrieval_rows = clean_retrieval(retrieval_rows)
            write_jsonl(args.output_retrieval, clean_retrieval_rows)
            log(
                "wrote clean retrieval "
                f"rows={len(clean_retrieval_rows)} path={args.output_retrieval}"
            )
            LOGGER.step(
                stage=stage,
                state="running",
                step=3,
                total_steps=total_steps,
                message=f"wrote clean retrieval to {args.output_retrieval}",
                input_retrieval_rows=len(retrieval_rows),
                output_retrieval_rows=len(clean_retrieval_rows),
            )
            report.update(
                {
                    "retrieval_file": str(args.retrieval_file),
                    "output_retrieval": str(args.output_retrieval),
                    "input_retrieval_rows": len(retrieval_rows),
                    "output_retrieval_rows": len(clean_retrieval_rows),
                    "retrieval_oracle_fields_stripped": stripped_counts(retrieval_rows),
                    "retrieval_non_clean_fields_stripped": sorted(
                        {
                            key
                            for row in retrieval_rows
                            for key in row
                            if key not in {"answer", "document_ids", "question", "question_id"}
                        }
                    ),
                }
            )

        write_json(args.report, report)
        log(f"wrote report {args.report}")
        LOGGER.step(
            stage=stage,
            state="done",
            step=total_steps,
            total_steps=total_steps,
            message=f"wrote report {args.report}",
            input_questions=len(question_rows),
            output_questions=len(clean_question_rows),
            report=str(args.report),
        )
        print(json.dumps(report, sort_keys=True))
        return 0
    except Exception as error:
        LOGGER.status(stage=stage, state="failed", step=total_steps, total_steps=total_steps, error=str(error))
        raise


if __name__ == "__main__":
    raise SystemExit(main())
