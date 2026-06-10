#!/usr/bin/env python3
"""Run the oracle-free EnterpriseRAG-Bench pipeline end to end."""

from __future__ import annotations

import argparse
import datetime as dt
import json
import subprocess
import time
from pathlib import Path
from typing import Callable

from progress_logging import format_duration


ROOT = Path(__file__).resolve().parents[2]
BENCH_ROOT = ROOT / "target/external-benchmarks/EnterpriseRAG-Bench"
OUT_ROOT = ROOT / "target/enterprise-rag-bench/official-clean"
RUN_LOG: Path | None = None
RUN_STATUS: Path | None = None
RUN_STARTED_PERF: float | None = None
RUN_STARTED_AT: str | None = None
RUN_SPLIT_NAME: str | None = None
RUN_QUESTIONS_FILE: Path | None = None
CURRENT_STAGE: str | None = None
CURRENT_STEP: int | None = None
CURRENT_TOTAL_STEPS: int | None = None
MAX_LOG_LINE_CHARS = 4_000


def log(message: str) -> None:
    stamp = dt.datetime.now(dt.UTC).isoformat(timespec="seconds")
    line = f"[official-clean {stamp}] {message}"
    print(line, flush=True)
    append_run_log(line)


def append_run_log(line: str) -> None:
    if RUN_LOG is not None:
        RUN_LOG.parent.mkdir(parents=True, exist_ok=True)
        if len(line) > MAX_LOG_LINE_CHARS:
            line = line[:MAX_LOG_LINE_CHARS] + " ... [truncated]"
        with RUN_LOG.open("a", encoding="utf-8") as handle:
            handle.write(line + "\n")


def set_run_log(path: Path) -> None:
    global RUN_LOG
    RUN_LOG = path
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("", encoding="utf-8")


def set_run_status(path: Path, started_at: str) -> None:
    global RUN_STATUS, RUN_STARTED_PERF, RUN_STARTED_AT
    RUN_STATUS = path
    RUN_STARTED_PERF = time.perf_counter()
    RUN_STARTED_AT = started_at
    path.parent.mkdir(parents=True, exist_ok=True)


def set_stage_context(stage: str, step: int, total_steps: int) -> None:
    global CURRENT_STAGE, CURRENT_STEP, CURRENT_TOTAL_STEPS
    CURRENT_STAGE = stage
    CURRENT_STEP = step
    CURRENT_TOTAL_STEPS = total_steps


def elapsed_seconds() -> float:
    if RUN_STARTED_PERF is None:
        return 0.0
    return max(0.0, time.perf_counter() - RUN_STARTED_PERF)


def read_status_snapshot(path: Path | None) -> dict[str, object] | None:
    if path is None or not path.exists():
        return None
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    return payload if isinstance(payload, dict) else None


def read_json_snapshot(path: Path | None) -> dict[str, object] | None:
    if path is None or not path.exists():
        return None
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return None
    return payload if isinstance(payload, dict) else None


def write_current_status(
    *,
    state: str,
    subprocess_label: str | None = None,
    command: list[str] | None = None,
    pid: int | None = None,
    last_output_line: str | None = None,
    error: str | None = None,
    artifacts: dict[str, Path] | None = None,
    child_status: Path | None = None,
) -> None:
    if RUN_STATUS is None:
        return
    payload: dict[str, object] = {
        "schema_version": "cortexdb.enterprise_rag_bench.official_clean_status.v2",
        "stage": CURRENT_STAGE or "unknown",
        "step": CURRENT_STEP or 0,
        "total_steps": CURRENT_TOTAL_STEPS or 0,
        "state": state,
        "started_at": RUN_STARTED_AT,
        "updated_at": dt.datetime.now(dt.UTC).isoformat(timespec="seconds"),
        "elapsed_seconds": round(elapsed_seconds(), 1),
        "elapsed": format_duration(elapsed_seconds()),
        "run_log": str(RUN_LOG) if RUN_LOG else None,
        "split_name": RUN_SPLIT_NAME,
        "questions_file": str(RUN_QUESTIONS_FILE) if RUN_QUESTIONS_FILE else None,
    }
    if subprocess_label:
        payload["subprocess_label"] = subprocess_label
    if command:
        payload["command"] = " ".join(str(item) for item in command)
    if pid:
        payload["pid"] = pid
    if last_output_line:
        payload["last_output_line"] = last_output_line[-500:]
    if error:
        payload["error"] = error
    if artifacts:
        payload["artifacts"] = {key: str(value) for key, value in artifacts.items()}
    child = read_status_snapshot(child_status)
    if child is not None:
        payload["child_status"] = child
    write_json(RUN_STATUS, payload)


def run_cmd(
    cmd: list[str],
    *,
    cwd: Path = ROOT,
    label: str,
    child_status: Path | None = None,
    artifacts: dict[str, Path] | None = None,
) -> None:
    started = time.perf_counter()
    log(f"begin {label}")
    log("+ " + " ".join(str(item) for item in cmd))
    write_current_status(
        state="running",
        subprocess_label=label,
        command=cmd,
        child_status=child_status,
        artifacts=artifacts,
    )
    process = subprocess.Popen(
        cmd,
        cwd=cwd,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        bufsize=1,
    )
    assert process.stdout is not None
    for raw_line in process.stdout:
        line = raw_line.rstrip("\n")
        print(line, flush=True)
        append_run_log(line)
        write_current_status(
            state="running",
            subprocess_label=label,
            command=cmd,
            pid=process.pid,
            last_output_line=line,
            child_status=child_status,
            artifacts=artifacts,
        )
    return_code = process.wait()
    if return_code != 0:
        write_current_status(
            state="failed",
            subprocess_label=label,
            command=cmd,
            pid=process.pid,
            error=f"return_code={return_code}",
            child_status=child_status,
            artifacts=artifacts,
        )
        raise subprocess.CalledProcessError(return_code, cmd)
    elapsed = time.perf_counter() - started
    log(f"done {label} elapsed={elapsed:.1f}s")
    write_current_status(
        state="running",
        subprocess_label=f"{label}: done",
        command=cmd,
        child_status=child_status,
        artifacts=artifacts,
    )


def write_json(path: Path, payload: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


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


def prepare(args: argparse.Namespace, p: dict[str, Path]) -> None:
    log(
        "prepare config "
        f"questions_file={args.questions_file} output_questions={p['clean_questions']} "
        f"limit={args.size}"
    )
    cmd = [
        "python3",
        "scripts/enterprise_rag_bench/prepare_official_clean_inputs.py",
        "--questions-file",
        str(args.questions_file),
        "--output-questions",
        str(p["clean_questions"]),
        "--report",
        str(p["prepare_report"]),
        "--limit",
        size_limit(args.size),
        "--log-file",
        str(p["prepare_log"]),
        "--status-file",
        str(p["prepare_status"]),
    ]
    run_cmd(
        cmd,
        label="prepare clean questions",
        child_status=p["prepare_status"],
        artifacts={
            "clean_questions": p["clean_questions"],
            "prepare_report": p["prepare_report"],
            "prepare_log": p["prepare_log"],
        },
    )


def retrieve(args: argparse.Namespace, p: dict[str, Path]) -> None:
    retrieval_top_k = (
        args.embedding_rerank_candidates if args.embedding_rerank else args.top_k
    )
    sanitize_target = (
        p["clean_retrieval_wide"] if args.embedding_rerank else p["clean_retrieval"]
    )
    log(
        "retrieve config "
        f"mode={args.retrieval_mode} rerank={args.rerank} "
        f"embedding_rerank={args.embedding_rerank} top_k={retrieval_top_k} "
        f"batch_size={args.batch_size} "
        f"reuse_db={args.reuse_db} db_root={p['db_root']} "
        f"query_vectors={args.query_vectors} document_vectors={args.document_vectors} "
        f"progress_every={args.retrieval_progress_every}"
    )
    run_cmd(
        ["cargo", "build", "--release", "-p", "cortex-engine", "--bin", "enterprise_rag_bench_retrieval"],
        label="build retrieval binary",
        artifacts={"retrieval_binary": ROOT / "target/release/enterprise_rag_bench_retrieval"},
    )
    cmd = [
        "./target/release/enterprise_rag_bench_retrieval",
        "--questions",
        str(p["clean_questions"]),
        "--uuid-index",
        str(args.uuid_index),
        "--sources-dir",
        str(args.sources_dir),
        "--db-root",
        str(p["db_root"]),
        "--output",
        str(p["raw_retrieval"]),
        "--report",
        str(p["retrieval_report"]),
        "--top-k",
        str(retrieval_top_k),
        "--batch-size",
        str(args.batch_size),
        "--progress-every",
        str(args.retrieval_progress_every),
        "--retrieval-mode",
        args.retrieval_mode,
        "--rerank",
        args.rerank,
        "--official-clean",
        "--log-file",
        str(p["retrieval_log"]),
        "--status-file",
        str(p["retrieval_status"]),
    ]
    if args.max_documents:
        cmd.extend(["--max-documents", str(args.max_documents)])
    if args.query_vectors:
        cmd.extend(["--query-vectors", str(args.query_vectors)])
    if args.document_vectors:
        cmd.extend(["--document-vectors", str(args.document_vectors)])
    if args.reuse_db:
        cmd.append("--skip-ingest")
    else:
        cmd.append("--reset-db")
    run_cmd(
        cmd,
        label="retrieve with CortexDB",
        child_status=p["retrieval_status"],
        artifacts={
            "raw_retrieval": p["raw_retrieval"],
            "retrieval_report": p["retrieval_report"],
            "retrieval_log": p["retrieval_log"],
            "retrieval_status": p["retrieval_status"],
            "db_root": p["db_root"],
        },
    )
    run_cmd(
        [
            "python3",
            "scripts/enterprise_rag_bench/prepare_official_clean_inputs.py",
            "--questions-file",
            str(p["clean_questions"]),
            "--output-questions",
            str(p["clean_questions"]),
            "--report",
            str(p["retrieval_clean_report"]),
            "--retrieval-file",
            str(p["raw_retrieval"]),
            "--output-retrieval",
            str(sanitize_target),
            "--log-file",
            str(p["sanitize_log"]),
            "--status-file",
            str(p["sanitize_status"]),
        ],
        label="sanitize retrieval output",
        child_status=p["sanitize_status"],
        artifacts={
            "clean_retrieval": sanitize_target,
            "retrieval_clean_report": p["retrieval_clean_report"],
            "sanitize_log": p["sanitize_log"],
        },
    )
    if args.embedding_rerank:
        embedding_rerank(args, p)


def embedding_rerank(args: argparse.Namespace, p: dict[str, Path]) -> None:
    """Rerank the wide clean candidate set with an external embedding model.

    This is the EPIC-04 external-model reranker. It reads only the question text
    and candidate document bodies (no oracle labels), embeds them, and keeps the
    cosine-nearest `--top-k` documents. The output keeps the clean row shape, so
    the answer-stage guard still passes.
    """
    log(
        "embedding-rerank config "
        f"candidates={args.embedding_rerank_candidates} final_top_k={args.top_k} "
        f"env_file={args.env_file} cache={p['embedding_cache']}"
    )
    run_cmd(
        [
            "python3",
            "scripts/enterprise_rag_bench/rerank_with_embeddings.py",
            "--retrieval-file",
            str(p["clean_retrieval_wide"]),
            "--uuid-index",
            str(args.uuid_index),
            "--sources-dir",
            str(args.sources_dir),
            "--output",
            str(p["clean_retrieval"]),
            "--report",
            str(p["embedding_rerank_report"]),
            "--cache-file",
            str(p["embedding_cache"]),
            "--env-file",
            str(args.env_file),
            "--top-k",
            str(args.top_k),
            "--max-chars-per-doc",
            str(args.max_chars_per_doc),
        ],
        label="embedding rerank wide candidates",
        artifacts={
            "clean_retrieval": p["clean_retrieval"],
            "embedding_rerank_report": p["embedding_rerank_report"],
            "embedding_rerank_log": p["embedding_rerank_log"],
        },
    )


def answer(args: argparse.Namespace, p: dict[str, Path]) -> None:
    log(
        "answer config "
        f"provider={args.answer_provider} context_mode={args.context_mode} "
        f"top_k_context={args.top_k_context} max_chars_per_doc={args.max_chars_per_doc} "
        f"max_tokens={args.max_tokens} workers={args.answer_workers} "
        f"progress_every={args.progress_every}"
    )
    cmd = [
        "python3",
        "scripts/enterprise_rag_bench/run_official_clean_answers.py",
        "--retrieval-file",
        str(p["clean_retrieval"]),
        "--uuid-index",
        str(args.uuid_index),
        "--sources-dir",
        str(args.sources_dir),
        "--output-root",
        str(p["answer_root"]),
        "--provider",
        args.answer_provider,
        "--top-k-context",
        str(args.top_k_context),
        "--max-chars-per-doc",
        str(args.max_chars_per_doc),
        "--max-tokens",
        str(args.max_tokens),
        "--context-mode",
        args.context_mode,
        "--workers",
        str(args.answer_workers),
        "--progress-every",
        str(args.progress_every),
        "--log-file",
        str(p["answer_log"]),
        "--status-file",
        str(p["answer_status"]),
    ]
    run_cmd(
        cmd,
        label=f"answer questions with {args.answer_provider}",
        child_status=p["answer_status"],
        artifacts={
            "answers": p["answers"],
            "answer_log": p["answer_log"],
            "answer_status": p["answer_status"],
        },
    )


def judge(args: argparse.Namespace, p: dict[str, Path]) -> None:
    log(
        "judge config "
        f"provider={args.judge_provider} workers={args.judge_workers} "
        f"timeout_seconds={args.judge_timeout_seconds} progress_every={args.progress_every}"
    )
    cmd = [
        "python3",
        "scripts/enterprise_rag_bench/run_official_clean_judge.py",
        "--answers-file",
        str(p["answers"]),
        "--questions-file",
        str(args.questions_file),
        "--results-file",
        str(p["judge_results"]),
        "--judgments-file",
        str(p["judge_rows"]),
        "--provider",
        args.judge_provider,
        "--workers",
        str(args.judge_workers),
        "--timeout-seconds",
        str(args.judge_timeout_seconds),
        "--limit",
        size_limit(args.size),
        "--progress-every",
        str(args.progress_every),
        "--log-file",
        str(p["judge_log"]),
        "--status-file",
        str(p["judge_status"]),
    ]
    run_cmd(
        cmd,
        label=f"judge answers with {args.judge_provider}",
        child_status=p["judge_status"],
        artifacts={
            "judge_results": p["judge_results"],
            "judge_rows": p["judge_rows"],
            "judge_log": p["judge_log"],
            "judge_status": p["judge_status"],
        },
    )


def selected_stages(stage: str) -> list[tuple[str, Callable[[argparse.Namespace, dict[str, Path]], None]]]:
    stages: list[tuple[str, Callable[[argparse.Namespace, dict[str, Path]], None]]] = [
        ("prepare", prepare),
        ("retrieve", retrieve),
        ("answer", answer),
        ("judge", judge),
    ]
    if stage == "all":
        return stages
    if stage == "retrieval":
        return stages[:2]
    return [(name, runner) for name, runner in stages if name == stage]


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
            "document_recall_pct": judge_stats.get("average_document_recall_pct"),
            "invalid_extra_docs": judge_stats.get("average_invalid_extra_docs"),
            "prompt_tokens": judge_report.get("prompt_tokens"),
            "completion_tokens": judge_report.get("completion_tokens"),
            "total_tokens": judge_report.get("total_tokens"),
        },
    }


def write_status(
    path: Path,
    *,
    stage: str,
    step: int,
    total_steps: int,
    state: str,
    started_at: str,
    error: str | None = None,
    artifacts: dict[str, Path] | None = None,
    child_status: Path | None = None,
) -> None:
    payload: dict[str, object] = {
        "schema_version": "cortexdb.enterprise_rag_bench.official_clean_status.v2",
        "stage": stage,
        "step": step,
        "total_steps": total_steps,
        "state": state,
        "started_at": started_at,
        "updated_at": dt.datetime.now(dt.UTC).isoformat(timespec="seconds"),
        "run_log": str(RUN_LOG) if RUN_LOG else None,
        "elapsed_seconds": round(elapsed_seconds(), 1),
        "elapsed": format_duration(elapsed_seconds()),
        "split_name": RUN_SPLIT_NAME,
        "questions_file": str(RUN_QUESTIONS_FILE) if RUN_QUESTIONS_FILE else None,
    }
    if error:
        payload["error"] = error
    if artifacts:
        payload["artifacts"] = {key: str(value) for key, value in artifacts.items()}
    child = read_status_snapshot(child_status)
    if child is not None:
        payload["child_status"] = child
    write_json(path, payload)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--size", type=int, required=True)
    parser.add_argument(
        "--run-label",
        help="Optional artifact namespace under official-clean/<size>/ for mode comparisons.",
    )
    parser.add_argument("--answer-provider", choices=["gemma", "gemini", "deepseek"], required=True)
    parser.add_argument("--judge-provider", choices=["gemma", "gemini", "deepseek"], required=True)
    parser.add_argument("--questions-file", type=Path, default=BENCH_ROOT / "questions.jsonl")
    parser.add_argument(
        "--split-name",
        default="primary",
        help="Logical benchmark split name recorded in reports, for example primary or heldout.",
    )
    parser.add_argument("--uuid-index", type=Path, default=BENCH_ROOT / "generated_data/uuid_index.json")
    parser.add_argument("--sources-dir", type=Path, default=BENCH_ROOT / "generated_data/sources")
    parser.add_argument(
        "--db-root",
        type=Path,
        help="Existing CortexDB corpus root to use with --reuse-db, or target root for ingest.",
    )
    parser.add_argument("--top-k", type=int, default=10)
    parser.add_argument("--batch-size", type=int, default=1000)
    parser.add_argument(
        "--max-documents",
        type=int,
        help="Limit corpus ingest for smoke runs; omit for honest full-corpus evaluation.",
    )
    parser.add_argument(
        "--retrieval-mode",
        choices=["cached-lexical", "engine-keyword", "engine-hybrid"],
        default="cached-lexical",
    )
    parser.add_argument(
        "--rerank",
        choices=["none", "weighted"],
        default="none",
        help="Engine rerank stage applied to engine-keyword/engine-hybrid candidates.",
    )
    parser.add_argument(
        "--embedding-rerank",
        action="store_true",
        help="Rerank a wide candidate set with an external embedding model (bge-m3).",
    )
    parser.add_argument(
        "--embedding-rerank-candidates",
        type=int,
        default=50,
        help="Candidate pool size retrieved before embedding rerank narrows to --top-k.",
    )
    parser.add_argument(
        "--env-file",
        type=Path,
        default=ROOT / ".env",
        help="Env file with CORTEXDB_EMBEDDING_URL/MODEL/API_KEY for embedding rerank.",
    )
    parser.add_argument(
        "--query-vectors",
        type=Path,
        help="JSONL with {question_id, vector}; required for engine-hybrid.",
    )
    parser.add_argument(
        "--document-vectors",
        type=Path,
        help="JSONL with {doc_id, vector}; required for dense engine-hybrid corpus ingest.",
    )
    parser.add_argument("--top-k-context", type=int, default=8)
    parser.add_argument("--max-chars-per-doc", type=int, default=2200)
    parser.add_argument("--max-tokens", type=int, default=420)
    parser.add_argument("--context-mode", default="question-window-digest-ranked")
    parser.add_argument("--answer-workers", type=int, default=2)
    parser.add_argument("--judge-workers", type=int, default=2)
    parser.add_argument("--judge-timeout-seconds", type=float, default=180.0)
    parser.add_argument("--progress-every", type=int, default=10)
    parser.add_argument("--retrieval-progress-every", type=int, default=50_000)
    parser.add_argument(
        "--reuse-db",
        action="store_true",
        help="Reuse an existing indexed CortexDB root and skip corpus ingest.",
    )
    parser.add_argument(
        "--stage",
        choices=["all", "retrieval", "prepare", "retrieve", "answer", "judge"],
        default="all",
    )
    args = parser.parse_args()
    if args.size <= 0:
        parser.error("--size must be positive")
    if args.retrieval_mode == "engine-hybrid" and not args.query_vectors:
        parser.error("--retrieval-mode engine-hybrid requires --query-vectors")
    if args.rerank == "weighted" and args.retrieval_mode == "cached-lexical":
        parser.error("--rerank weighted requires --retrieval-mode engine-keyword or engine-hybrid")
    if args.embedding_rerank and args.embedding_rerank_candidates <= args.top_k:
        parser.error("--embedding-rerank-candidates must exceed --top-k to have any effect")
    if args.max_documents is not None and args.max_documents <= 0:
        parser.error("--max-documents must be positive")
    if args.run_label and (
        "/" in args.run_label or "\\" in args.run_label or args.run_label in {".", ".."}
    ):
        parser.error("--run-label must be a simple path segment")
    if not args.split_name.strip():
        parser.error("--split-name must not be empty")

    p = paths(
        args.size,
        args.answer_provider,
        args.judge_provider,
        args.db_root,
        args.run_label,
    )
    global RUN_SPLIT_NAME, RUN_QUESTIONS_FILE
    RUN_SPLIT_NAME = args.split_name.strip()
    RUN_QUESTIONS_FILE = args.questions_file
    set_run_log(p["run_log"])
    started_at = dt.datetime.now(dt.UTC).isoformat(timespec="seconds")
    set_run_status(p["status"], started_at)
    log(
        "start run "
        f"size={args.size} answer_provider={args.answer_provider} "
        f"judge_provider={args.judge_provider} stage={args.stage} reuse_db={args.reuse_db} "
        f"split={RUN_SPLIT_NAME} questions_file={args.questions_file} "
        f"run_log={p['run_log']} status={p['status']}"
    )
    stages = selected_stages(args.stage)
    for index, (name, runner) in enumerate(stages, 1):
        set_stage_context(name, index, len(stages))
        log(f"step {index}/{len(stages)} {name}: start")
        write_status(
            p["status"],
            stage=name,
            step=index,
            total_steps=len(stages),
            state="running",
            started_at=started_at,
            artifacts=stage_artifacts(name, p),
            child_status=stage_child_status(name, p),
        )
        try:
            runner(args, p)
        except Exception as error:
            write_status(
                p["status"],
                stage=name,
                step=index,
                total_steps=len(stages),
                state="failed",
                started_at=started_at,
                error=str(error),
                artifacts=stage_artifacts(name, p),
                child_status=stage_child_status(name, p),
            )
            log(f"step {index}/{len(stages)} {name}: failed error={error}")
            raise
        write_status(
            p["status"],
            stage=name,
            step=index,
            total_steps=len(stages),
            state="done",
            started_at=started_at,
            artifacts=stage_artifacts(name, p),
            child_status=stage_child_status(name, p),
        )
        log(f"step {index}/{len(stages)} {name}: done")

    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.official_clean_run.v1",
        "size": args.size,
        "answer_provider": args.answer_provider,
        "judge_provider": args.judge_provider,
        "run_label": args.run_label,
        "split_name": RUN_SPLIT_NAME,
        "questions_file": str(args.questions_file),
        "stage": args.stage,
        "reuse_db": args.reuse_db,
        "retrieval_mode": args.retrieval_mode,
        "rerank": args.rerank,
        "embedding_rerank": args.embedding_rerank,
        "embedding_rerank_candidates": args.embedding_rerank_candidates if args.embedding_rerank else None,
        "query_vectors": str(args.query_vectors) if args.query_vectors else None,
        "document_vectors": str(args.document_vectors) if args.document_vectors else None,
        "max_documents": args.max_documents,
        "clean_questions": str(p["clean_questions"]),
        "clean_retrieval": str(p["clean_retrieval"]),
        "retrieval_status": str(p["retrieval_status"]),
        "answers": str(p["answers"]),
        "judge_results": str(p["judge_results"]),
        "run_log": str(p["run_log"]),
        "status": str(p["status"]),
        "prepare_status": str(p["prepare_status"]),
        "answer_status": str(p["answer_status"]),
        "judge_status": str(p["judge_status"]),
        "progress_artifacts": run_progress_artifacts(p),
        "summary": run_summary(p),
        "rule": "Inference input is question_id/question plus retrieved document_ids only; gold labels are used only by the judge stage.",
        "inference_oracle_policy": {
            "allowed_question_fields": ["question_id", "question"],
            "forbidden_question_fields": [
                "answer_facts",
                "expected_doc_ids",
                "gold_answer",
                "question_type",
                "source_types",
            ],
            "gold_usage": "judge-only",
        },
    }
    write_json(p["run_report"], report)
    set_stage_context("finished", len(stages), len(stages))
    write_current_status(
        state="done",
        subprocess_label="finished run",
        artifacts={
            "run_report": p["run_report"],
            "run_log": p["run_log"],
            "clean_retrieval": p["clean_retrieval"],
            "answers": p["answers"],
            "judge_results": p["judge_results"],
        },
    )
    summary = report["summary"]
    if isinstance(summary, dict):
        judge_summary = summary.get("judge")
        answer_summary = summary.get("answer")
        if isinstance(judge_summary, dict) or isinstance(answer_summary, dict):
            log(
                "final summary "
                f"overall={judge_summary.get('overall') if isinstance(judge_summary, dict) else None} "
                f"correctness={judge_summary.get('answer_correctness_pct') if isinstance(judge_summary, dict) else None} "
                f"completeness={judge_summary.get('answer_completeness_pct') if isinstance(judge_summary, dict) else None} "
                f"answer_tokens={answer_summary.get('total_tokens') if isinstance(answer_summary, dict) else None} "
                f"judge_tokens={judge_summary.get('total_tokens') if isinstance(judge_summary, dict) else None}"
            )
    log(f"wrote run report {p['run_report']}")
    print(json.dumps(report, sort_keys=True))
    log("finished run")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
