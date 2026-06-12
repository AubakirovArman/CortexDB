"""Artifact path construction and run summary helpers."""

from __future__ import annotations

from pathlib import Path

from .constants import OUT_ROOT
from .status import read_json_snapshot


def size_limit(size: int) -> str:
    if size <= 0:
        raise ValueError("--size must be positive")
    return str(size)


def paths(
    size: int,
    answer_provider: str,
    judge_provider: str,
    db_root: Path | None = None,
    run_label: str | None = None,
) -> dict[str, Path]:
    base = OUT_ROOT / f"{size}"
    if run_label:
        base = base / run_label
    run = base / f"answer-{answer_provider}"
    judge = run / f"judge-{judge_provider}"
    return {
        "base": base,
        "clean_questions": base / "questions.clean.jsonl",
        "prepare_report": base / "prepare_report.json",
        "raw_retrieval": base / "retrieval.raw.jsonl",
        "clean_retrieval": base / "retrieval.clean.jsonl",
        "clean_retrieval_wide": base / "retrieval.clean.wide.jsonl",
        "embedding_rerank_report": base / "embedding_rerank_report.json",
        "embedding_cache": base / "embedding_cache.jsonl",
        "embedding_rerank_log": base / "embedding_rerank.log",
        "retrieval_report": base / "retrieval_report.json",
        "retrieval_clean_report": base / "retrieval_clean_report.json",
        "retrieval_log": base / "retrieval_progress.log",
        "retrieval_status": base / "retrieval_status.json",
        "db_root": db_root or base / "cortexdb",
        "prepare_log": base / "prepare.log",
        "prepare_status": base / "prepare_status.json",
        "sanitize_log": base / "sanitize_retrieval.log",
        "sanitize_status": base / "sanitize_retrieval_status.json",
        "answer_root": run,
        "answers": run / "answers.jsonl",
        "answer_log": run / "answer_progress.log",
        "answer_status": run / "answer_status.json",
        "judge_root": judge,
        "judge_results": judge / "results.json",
        "judge_rows": judge / "judgments.jsonl",
        "judge_log": judge / "judge_progress.log",
        "judge_status": judge / "judge_status.json",
        "run_report": run / "official_clean_run_report.json",
        "run_log": base / "official_clean_run.log",
        "status": base / "official_clean_status.json",
    }

def stage_artifacts(stage: str, p: dict[str, Path]) -> dict[str, Path]:
    if stage == "prepare":
        return {
            "clean_questions": p["clean_questions"],
            "prepare_report": p["prepare_report"],
            "prepare_log": p["prepare_log"],
            "prepare_status": p["prepare_status"],
        }
    if stage == "retrieve":
        return {
            "raw_retrieval": p["raw_retrieval"],
            "clean_retrieval": p["clean_retrieval"],
            "retrieval_report": p["retrieval_report"],
            "retrieval_clean_report": p["retrieval_clean_report"],
            "retrieval_log": p["retrieval_log"],
            "retrieval_status": p["retrieval_status"],
            "db_root": p["db_root"],
        }
    if stage == "answer":
        return {
            "answers": p["answers"],
            "answer_log": p["answer_log"],
            "answer_status": p["answer_status"],
        }
    if stage == "judge":
        return {
            "judge_results": p["judge_results"],
            "judge_rows": p["judge_rows"],
            "judge_log": p["judge_log"],
            "judge_status": p["judge_status"],
        }
    return {}


def stage_child_status(stage: str, p: dict[str, Path]) -> Path | None:
    if stage == "prepare":
        return p["prepare_status"]
    if stage == "answer":
        return p["answer_status"]
    if stage == "judge":
        return p["judge_status"]
    if stage == "retrieve":
        return p["retrieval_status"]
    return None


def run_progress_artifacts(p: dict[str, Path]) -> dict[str, str]:
    return {
        "run_log": str(p["run_log"]),
        "run_status": str(p["status"]),
        "prepare_log": str(p["prepare_log"]),
        "prepare_status": str(p["prepare_status"]),
        "retrieval_log": str(p["retrieval_log"]),
        "retrieval_status": str(p["retrieval_status"]),
        "sanitize_log": str(p["sanitize_log"]),
        "sanitize_status": str(p["sanitize_status"]),
        "answer_log": str(p["answer_log"]),
        "answer_status": str(p["answer_status"]),
        "judge_log": str(p["judge_log"]),
        "judge_status": str(p["judge_status"]),
    }


def run_summary(p: dict[str, Path]) -> dict[str, object]:
    retrieval_report = read_json_snapshot(p["retrieval_report"]) or {}
    answer_report = read_json_snapshot(p["answer_root"] / "answer_generation_report.json") or {}
    judge_report = read_json_snapshot(p["judge_results"]) or {}
    judge_stats = judge_report.get("aggregate_stats")
    if not isinstance(judge_stats, dict):
        judge_stats = {}

    retrieval_performance = retrieval_report.get("performance")
    if not isinstance(retrieval_performance, dict):
        retrieval_performance = {}

    return {
        "retrieval": {
            "report": str(p["retrieval_report"]),
            "clean_retrieval": str(p["clean_retrieval"]),
            "performance": retrieval_performance,
        },
        "answer": {
            "report": str(p["answer_root"] / "answer_generation_report.json"),
            "answers": str(p["answers"]),
            "questions": answer_report.get("questions"),
            "prompt_tokens": answer_report.get("prompt_tokens"),
            "completion_tokens": answer_report.get("completion_tokens"),
            "total_tokens": answer_report.get("total_tokens"),
            "wall_elapsed_ms": answer_report.get("wall_elapsed_ms"),
        },
        "judge": {
            "results": str(p["judge_results"]),
            "judgments": str(p["judge_rows"]),
            "overall": judge_stats.get("combined_correctness_completeness_score"),
            "answer_correctness_pct": judge_stats.get("average_correctness_pct"),
            "answer_completeness_pct": judge_stats.get("average_completeness_pct"),
            "document_recall_pct": judge_stats.get("average_recall_pct")
            or judge_stats.get("average_document_recall_pct"),
            "invalid_extra_docs": judge_stats.get("average_invalid_extra_docs"),
            "prompt_tokens": judge_report.get("prompt_tokens"),
            "completion_tokens": judge_report.get("completion_tokens"),
            "total_tokens": judge_report.get("total_tokens"),
        },
    }
