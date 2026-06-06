#!/usr/bin/env python3
"""Validate CortexDB LongMemEval v1 end-to-end adapter evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
from collections import defaultdict
from pathlib import Path
from typing import Any

PACKAGE_SCHEMA = "cortexdb.longmemeval.v1.submission_package.v1"
DATA_SCHEMA = "cortexdb.longmemeval.v1.official_data_manifest.v1"
RETRIEVAL_SCHEMA = "cortexdb.longmemeval.v1.retrieval_report.v1"
ADAPTER_SCHEMA = "cortexdb.longmemeval.v1.retrieval_adapter_check.v1"
EXPECTED_DATASET = "xiaowu0162/longmemeval-cleaned"


def fail(message: str) -> None:
    raise RuntimeError(message)


def load_json(path: Path) -> Any:
    if not path.exists():
        fail(f"missing file: {path}")
    return json.loads(path.read_text(encoding="utf-8"))


def read_jsonl(path: Path, require_question_id: bool = True) -> list[dict[str, Any]]:
    if not path.exists():
        fail(f"missing file: {path}")
    rows: list[dict[str, Any]] = []
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        row = json.loads(line)
        if not isinstance(row, dict):
            fail(f"{path}:{line_number}: expected JSON object")
        if require_question_id and not row.get("question_id"):
            fail(f"{path}:{line_number}: missing question_id")
        rows.append(row)
    return rows


def read_reference_rows(path: Path) -> list[dict[str, Any]]:
    text = path.read_text(encoding="utf-8")
    if text.lstrip().startswith("["):
        rows = json.loads(text)
        if not isinstance(rows, list):
            fail(f"{path}: expected JSON array")
        for index, row in enumerate(rows, 1):
            if not isinstance(row, dict) or not row.get("question_id"):
                fail(f"{path}:{index}: expected object with question_id")
        return rows
    return read_jsonl(path)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def package_file(package_dir: Path, relative_path: str) -> Path:
    path = package_dir.parent / relative_path
    if path.exists():
        return path
    fallback = package_dir / Path(relative_path).name
    if fallback.exists():
        return fallback
    fail(f"missing packaged artifact: {relative_path}")


def validate_artifacts(package_dir: Path, artifacts: dict[str, Any]) -> dict[str, Any]:
    if not artifacts:
        fail("package manifest missing artifacts")
    result: dict[str, Any] = {}
    for name, entry in artifacts.items():
        if not isinstance(entry, dict):
            fail(f"artifact {name} is not an object")
        path = package_file(package_dir, str(entry.get("path", "")))
        actual_hash = sha256(path)
        expected_hash = entry.get("sha256")
        if actual_hash != expected_hash:
            fail(f"artifact {name} sha256 mismatch")
        actual_bytes = path.stat().st_size
        if isinstance(entry.get("bytes"), int) and entry["bytes"] != actual_bytes:
            fail(f"artifact {name} byte size mismatch")
        result[name] = {"path": str(path), "bytes": actual_bytes, "sha256": actual_hash}
    return result


def validate_data_manifest(data: dict[str, Any], min_rows: int) -> dict[str, Any]:
    if data.get("schema_version") != DATA_SCHEMA:
        fail("unexpected data manifest schema")
    if data.get("dataset") != EXPECTED_DATASET or data.get("split") != "s":
        fail("unexpected LongMemEval dataset or split")
    files = data.get("files")
    if not isinstance(files, list) or not files:
        fail("data manifest missing files")
    rows = int(files[0].get("rows", 0))
    if rows < min_rows:
        fail(f"data manifest rows {rows} below required {min_rows}")
    if not files[0].get("sha256"):
        fail("data manifest missing dataset sha256")
    return {"dataset": data["dataset"], "split": data["split"], "rows": rows}


def score_summary(eval_rows: list[dict[str, Any]], references: list[dict[str, Any]]) -> dict[str, Any]:
    qid_to_type = {str(row["question_id"]): str(row["question_type"]) for row in references}
    by_type: dict[str, list[int]] = defaultdict(list)
    correct = 0
    for row in eval_rows:
        question_id = str(row.get("question_id", ""))
        if question_id not in qid_to_type:
            fail(f"eval row references unknown question_id: {question_id}")
        autoeval = row.get("autoeval_label")
        if not isinstance(autoeval, dict) or not isinstance(autoeval.get("label"), bool):
            fail(f"eval row {question_id} missing bool autoeval label")
        value = 1 if autoeval["label"] else 0
        correct += value
        by_type[qid_to_type[question_id]].append(value)
    return {
        "questions": len(eval_rows),
        "correct": correct,
        "accuracy": correct / len(eval_rows) if eval_rows else 0.0,
        "by_question_type": {
            name: {
                "accuracy": sum(values) / len(values),
                "count": len(values),
                "correct": sum(values),
            }
            for name, values in sorted(by_type.items())
        },
    }


def assert_score_matches(manifest_score: dict[str, Any], computed: dict[str, Any]) -> None:
    for name in ["questions", "correct"]:
        if int(manifest_score.get(name, -1)) != int(computed[name]):
            fail(f"package score mismatch for {name}")
    if abs(float(manifest_score.get("accuracy", -1.0)) - float(computed["accuracy"])) > 1e-9:
        fail("package score mismatch for accuracy")
    by_type = manifest_score.get("by_question_type")
    if not isinstance(by_type, dict):
        fail("package score missing by_question_type")
    for question_type, expected in computed["by_question_type"].items():
        actual = by_type.get(question_type)
        if not isinstance(actual, dict):
            fail(f"package score missing question type {question_type}")
        for name in ["count", "correct"]:
            if int(actual.get(name, -1)) != int(expected[name]):
                fail(f"package score {name} mismatch for {question_type}")
        if abs(float(actual.get("accuracy", -1.0)) - float(expected["accuracy"])) > 1e-9:
            fail(f"package score accuracy mismatch for {question_type}")


def parse_official_metrics(path: Path) -> dict[str, float]:
    metrics: dict[str, float] = {}
    for name, value in re.findall(r"([a-z_]+@\d+)\s*=\s*([0-9.]+)", path.read_text()):
        metrics[f"session {name}"] = float(value)
    for name in ["session recall_all@10", "session ndcg_any@10"]:
        if name not in metrics:
            fail(f"{path}: missing {name}")
    return metrics


def validate_retrieval_report(report: dict[str, Any], min_rows: int) -> dict[str, Any]:
    if report.get("schema_version") != RETRIEVAL_SCHEMA or report.get("status") != "passed":
        fail("retrieval report did not pass")
    summary = report.get("summary")
    if not isinstance(summary, dict):
        fail("retrieval report missing summary")
    question_count = int(summary.get("question_count", 0))
    if question_count < min_rows:
        fail(f"retrieval question_count {question_count} below required {min_rows}")
    metrics = summary.get("metrics")
    if not isinstance(metrics, dict):
        fail("retrieval report missing metrics")
    return {
        "question_count": question_count,
        "aggregate_count": int(summary.get("aggregate_count", 0)),
        "granularity": str(summary.get("granularity", "")),
        "recall_all@10": float(metrics["recall_all@10"]),
        "ndcg_any@10": float(metrics["ndcg_any@10"]),
    }


def validate_retrieval_adapter(report: dict[str, Any], min_rows: int) -> dict[str, Any]:
    if report.get("schema_version") != ADAPTER_SCHEMA or report.get("status") != "passed":
        fail("retrieval adapter did not pass")
    cortexdb = report.get("cortexdb")
    if not isinstance(cortexdb, dict):
        fail("retrieval adapter missing cortexdb section")
    rows = int(cortexdb.get("retrieval_log_rows", 0))
    if rows < min_rows:
        fail(f"retrieval adapter rows {rows} below required {min_rows}")
    boundary = report.get("claims_boundary")
    if not isinstance(boundary, list) or not any("retrieval-only" in str(item) for item in boundary):
        fail("retrieval adapter does not preserve retrieval-only claim boundary")
    return {"retrieval_log_rows": rows, "top_k": int(cortexdb.get("top_k", 0))}


def validate_package_notes(manifest: dict[str, Any]) -> None:
    notes = " ".join(str(item) for item in manifest.get("notes", []))
    required = ["not a published LongMemEval leaderboard entry", "full official local LongMemEval v1 run"]
    for text in required:
        if text not in notes:
            fail(f"package notes missing: {text}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--package-dir", type=Path, required=True)
    parser.add_argument("--reference-file", type=Path, required=True)
    parser.add_argument("--retrieval-adapter-report", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--min-rows", type=int, default=500)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.min_rows <= 0:
        fail("--min-rows must be positive")
    manifest = load_json(args.package_dir / "manifest.json")
    if manifest.get("schema_version") != PACKAGE_SCHEMA:
        fail("unexpected package manifest schema")
    validate_package_notes(manifest)
    artifacts = manifest.get("artifacts")
    if not isinstance(artifacts, dict):
        fail("package manifest missing artifacts")
    artifact_summary = validate_artifacts(args.package_dir, artifacts)

    paths = {
        name: package_file(args.package_dir, artifacts[name]["path"])
        for name in ["hypotheses", "eval_results", "data_manifest", "retrieval_report", "official_retrieval_metrics"]
    }
    hypotheses = read_jsonl(paths["hypotheses"])
    eval_rows = read_jsonl(paths["eval_results"])
    if len(hypotheses) < args.min_rows:
        fail(f"hypotheses rows {len(hypotheses)} below required {args.min_rows}")
    if len(eval_rows) != len(hypotheses):
        fail("hypotheses and eval row counts differ")
    if {row["question_id"] for row in hypotheses} != {row["question_id"] for row in eval_rows}:
        fail("hypotheses and eval question_id sets differ")

    computed_score = score_summary(eval_rows, read_reference_rows(args.reference_file))
    assert_score_matches(manifest.get("score", {}), computed_score)
    report = {
        "schema_version": "cortexdb.longmemeval.v1.e2e_adapter_check.v1",
        "status": "passed",
        "official": {
            "dataset": validate_data_manifest(load_json(paths["data_manifest"]), args.min_rows),
            "qa_evaluator": "LongMemEval/src/evaluation/evaluate_qa.py",
            "retrieval_metric_script": "LongMemEval/src/evaluation/print_retrieval_metrics.py",
            "retrieval_metrics": parse_official_metrics(paths["official_retrieval_metrics"]),
        },
        "cortexdb": {
            "package_dir": str(args.package_dir),
            "models": manifest.get("models", {}),
            "hypotheses_rows": len(hypotheses),
            "eval_rows": len(eval_rows),
            "score": computed_score,
            "retrieval_report": validate_retrieval_report(load_json(paths["retrieval_report"]), args.min_rows),
            "retrieval_adapter": validate_retrieval_adapter(load_json(args.retrieval_adapter_report), args.min_rows),
            "artifacts": artifact_summary,
        },
        "claims_boundary": [
            "end-to-end local LongMemEval v1 evidence",
            "retrieval metrics and QA accuracy are separate claims",
            "not a published LongMemEval leaderboard entry until submitted",
            "no API calls are made by this checker",
        ],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(report, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
