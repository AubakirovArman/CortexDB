#!/usr/bin/env python3
"""Unit tests for postprocess_retrieval_modes.py."""

from __future__ import annotations

import json
import tempfile
from pathlib import Path

import postprocess_retrieval_modes as ppm


def _write_jsonl(path: Path, rows: list[dict]) -> None:
    path.write_text("\n".join(json.dumps(row, sort_keys=True) for row in rows) + "\n", encoding="utf-8")


def test_oracle_free_abstain_keeps_high_level_documents() -> None:
    with tempfile.TemporaryDirectory() as tmpdir:
        tmp = Path(tmpdir)
        questions_file = tmp / "questions.jsonl"
        retrieval_file = tmp / "retrieval.jsonl"
        output_file = tmp / "out.jsonl"
        report_file = tmp / "report.json"
        _write_jsonl(
            questions_file,
            [
                {"question_id": "q1", "question": "What is the company mission and vision?"},
                {"question_id": "q2", "question": "What is the exact value of PROJ-123 budget?"},
            ],
        )
        _write_jsonl(
            retrieval_file,
            [
                {"question_id": "q1", "document_ids": ["d1"], "question": "What is the company mission and vision?"},
                {"question_id": "q2", "document_ids": [], "question": "What is the exact value of PROJ-123 budget?"},
            ],
        )
        args = ppm.parse_args(
            [
                "--questions-file", str(questions_file),
                "--retrieval-file", str(retrieval_file),
                "--output", str(output_file),
                "--report", str(report_file),
                "--oracle-free-abstain",
            ]
        )
        report = ppm.run(args)

        assert report["oracle_free_abstain"] is True
        assert report["changed_rows"] == 1
        assert report["abstain_reasons"] == {"no_evidence_for_literal": 1}

        rows = [json.loads(line) for line in output_file.read_text(encoding="utf-8").splitlines() if line.strip()]
        by_id = {row["question_id"]: row for row in rows}
        assert by_id["q1"]["document_ids"] == ["d1"]
        assert by_id["q2"]["document_ids"] == []
        assert by_id["q2"]["route"]["source"] == "abstain"


def test_legacy_question_type_abstain_still_works() -> None:
    with tempfile.TemporaryDirectory() as tmpdir:
        tmp = Path(tmpdir)
        questions_file = tmp / "questions.jsonl"
        retrieval_file = tmp / "retrieval.jsonl"
        output_file = tmp / "out.jsonl"
        report_file = tmp / "report.json"
        _write_jsonl(
            questions_file,
            [
                {"question_id": "q1", "question": "foo", "question_type": "basic"},
                {"question_id": "q2", "question": "bar", "question_type": "info_not_found"},
            ],
        )
        _write_jsonl(
            retrieval_file,
            [
                {"question_id": "q1", "document_ids": ["d1"], "question": "foo"},
                {"question_id": "q2", "document_ids": ["d2"], "question": "bar"},
            ],
        )
        args = ppm.parse_args(
            [
                "--questions-file", str(questions_file),
                "--retrieval-file", str(retrieval_file),
                "--output", str(output_file),
                "--report", str(report_file),
            ]
        )
        report = ppm.run(args)

        assert report["changed_rows"] == 1
        rows = [json.loads(line) for line in output_file.read_text(encoding="utf-8").splitlines() if line.strip()]
        by_id = {row["question_id"]: row for row in rows}
        assert by_id["q1"]["document_ids"] == ["d1"]
        assert by_id["q2"]["document_ids"] == []


if __name__ == "__main__":
    test_oracle_free_abstain_keeps_high_level_documents()
    test_legacy_question_type_abstain_still_works()
    print("ok")
