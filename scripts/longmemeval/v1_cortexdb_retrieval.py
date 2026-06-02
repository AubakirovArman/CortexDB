#!/usr/bin/env python3
"""Run CortexDB retrieval on official LongMemEval v1 data."""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any

from v1_official_metrics import evaluate_entry, should_skip_for_aggregate, summarize


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


def load_json_list(path: Path) -> list[dict[str, Any]]:
    value = json.loads(path.read_text(encoding="utf-8"))
    require(isinstance(value, list), f"{path}: expected JSON list")
    return value


def user_turns(session: list[dict[str, Any]]) -> list[dict[str, Any]]:
    return [turn for turn in session if turn.get("role") == "user"]


def session_text(session: list[dict[str, Any]]) -> str:
    return " ".join(str(turn.get("content", "")) for turn in user_turns(session)).strip()


def corpus_for_entry(entry: dict[str, Any], granularity: str) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for session_id, session, timestamp in zip(
        entry["haystack_session_ids"],
        entry["haystack_sessions"],
        entry["haystack_dates"],
    ):
        require(isinstance(session, list), f"{entry['question_id']}: session must be list")
        if granularity == "session":
            corpus_id = session_id
            turns = user_turns(session)
            if "answer" in corpus_id and all(not turn.get("has_answer", False) for turn in turns):
                corpus_id = corpus_id.replace("answer", "noans")
            rows.append(
                {
                    "corpus_id": corpus_id,
                    "text": session_text(session),
                    "timestamp": timestamp,
                    "raw_session_id": session_id,
                }
            )
            continue
        if granularity == "turn":
            for index, turn in enumerate(session):
                if turn.get("role") != "user":
                    continue
                corpus_id = f"{session_id}_{index + 1}"
                if "answer" in session_id and not turn.get("has_answer", False):
                    corpus_id = corpus_id.replace("answer", "noans")
                rows.append(
                    {
                        "corpus_id": corpus_id,
                        "text": str(turn.get("content", "")).strip(),
                        "timestamp": timestamp,
                        "raw_session_id": session_id,
                    }
                )
            continue
        raise RuntimeError(f"unsupported granularity: {granularity}")
    return [row for row in rows if row["text"]]


def payload_for_cell(question_id: str, row: dict[str, Any]) -> str:
    return "\n".join(
        [
            "scope=longmemeval",
            "status=ready",
            "type=memory",
            f"source=longmemeval:{question_id}:{row['corpus_id']}",
            "",
            f"LONGMEMEVAL_CORPUS_ID: {row['corpus_id']}",
            f"LONGMEMEVAL_TIMESTAMP: {row['timestamp']}",
            "",
            row["text"],
        ]
    )


def run_command(command: list[str], cwd: Path | None = None) -> str:
    result = subprocess.run(command, cwd=cwd, text=True, capture_output=True, check=False)
    if result.returncode != 0:
        raise RuntimeError(
            f"command failed ({result.returncode}): {' '.join(command)}\n"
            f"stdout={result.stdout}\nstderr={result.stderr}"
        )
    return result.stdout


def write_fixture(fixture_dir: Path, question_id: str, corpus: list[dict[str, Any]]) -> None:
    fixture_dir.mkdir(parents=True, exist_ok=True)
    with (fixture_dir / "cells.jsonl").open("w", encoding="utf-8") as handle:
        for index, row in enumerate(corpus, start=1):
            payload = payload_for_cell(question_id, row)
            handle.write(json.dumps({"cell_id": index, "payload": payload}) + "\n")


def extract_corpus_id(payload: str) -> str | None:
    for line in payload.splitlines():
        if line.startswith("LONGMEMEVAL_CORPUS_ID:"):
            return line.split(":", 1)[1].strip()
    return None


def search_cortexdb(
    cortexdb_bin: Path,
    db_path: Path,
    query: str,
    top_k: int,
) -> list[str]:
    raw = run_command(
        [
            str(cortexdb_bin),
            "--json",
            "search",
            str(db_path),
            "longmemeval",
            query,
            "--mode",
            "keyword",
        ]
    )
    parsed = json.loads(raw)
    results = parsed.get("results", [])
    require(isinstance(results, list), "CLI search JSON missing results list")
    ranked: list[str] = []
    for result in results[:top_k]:
        if not isinstance(result, dict):
            continue
        payload = result.get("payload")
        if isinstance(payload, str):
            corpus_id = extract_corpus_id(payload)
            if corpus_id:
                ranked.append(corpus_id)
    return ranked


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--data-file", type=Path, required=True)
    parser.add_argument("--cortexdb-bin", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--granularity", choices=["session", "turn"], default="session")
    parser.add_argument("--top-k", type=int, default=10)
    parser.add_argument("--limit", type=int)
    parser.add_argument("--keep-workdir", action="store_true")
    args = parser.parse_args(argv)

    require(args.cortexdb_bin.exists(), f"missing cortexdb binary: {args.cortexdb_bin}")
    require(args.top_k > 0, "--top-k must be positive")
    data = load_json_list(args.data_file)
    if args.limit is not None:
        data = data[: args.limit]
    args.output_dir.mkdir(parents=True, exist_ok=True)
    work_root = args.output_dir / "work"
    if work_root.exists() and not args.keep_workdir:
        shutil.rmtree(work_root)
    work_root.mkdir(parents=True, exist_ok=True)

    results: list[dict[str, Any]] = []
    for index, entry in enumerate(data, start=1):
        question_id = str(entry["question_id"])
        corpus = corpus_for_entry(entry, args.granularity)
        q_root = work_root / question_id
        fixture_dir = q_root / "fixture"
        db_path = q_root / "db"
        write_fixture(fixture_dir, question_id, corpus)
        run_command([str(args.cortexdb_bin), "load-fixture", str(db_path), str(fixture_dir)])
        ranked_ids = search_cortexdb(args.cortexdb_bin, db_path, str(entry["question"]), args.top_k)
        skip, reason = should_skip_for_aggregate(entry, corpus)
        corpus_by_id = {row["corpus_id"]: row for row in corpus}
        row = {
            "question_id": question_id,
            "question_type": entry["question_type"],
            "question": entry["question"],
            "answer": entry["answer"],
            "question_date": entry["question_date"],
            "haystack_dates": entry["haystack_dates"],
            "haystack_sessions": entry["haystack_sessions"],
            "haystack_session_ids": entry["haystack_session_ids"],
            "answer_session_ids": entry["answer_session_ids"],
            "aggregate_skip_reason": reason if skip else "",
            "retrieval_results": {
                "query": entry["question"],
                "ranked_items": [
                    {
                        "corpus_id": row["corpus_id"],
                        "text": row["text"],
                        "timestamp": row["timestamp"],
                    }
                    for corpus_id in ranked_ids
                    for row in [corpus_by_id.get(corpus_id)]
                    if row is not None
                ],
                "metrics": evaluate_entry(entry, corpus, ranked_ids, args.granularity),
            },
        }
        results.append(row)
        print(f"{index}/{len(data)} {question_id} ranked={len(ranked_ids)}", file=sys.stderr)

    log_path = args.output_dir / f"{args.data_file.stem}_cortexdb_{args.granularity}_retrieval.jsonl"
    with log_path.open("w", encoding="utf-8") as handle:
        for row in results:
            handle.write(json.dumps(row, ensure_ascii=True) + "\n")
    report = {
        "schema_version": "cortexdb.longmemeval.v1.retrieval_report.v1",
        "status": "passed",
        "official_dataset_file": str(args.data_file),
        "retrieval_log": str(log_path),
        "top_k": args.top_k,
        "summary": summarize(results, args.granularity),
    }
    report_path = args.output_dir / "report.json"
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    summary_path = args.output_dir / "summary.md"
    metrics = report["summary"]["metrics"]
    summary_path.write_text(
        "\n".join(
            [
                "# CortexDB LongMemEval v1 Official Retrieval Run",
                "",
                f"- dataset: `{args.data_file}`",
                f"- retrieval log: `{log_path}`",
                f"- questions: `{report['summary']['question_count']}`",
                f"- aggregate questions: `{report['summary']['aggregate_count']}`",
                f"- granularity: `{args.granularity}`",
                f"- recall_all@10: `{metrics.get('recall_all@10', 0.0):.4f}`",
                f"- ndcg_any@10: `{metrics.get('ndcg_any@10', 0.0):.4f}`",
                "",
                "This is a retrieval log for the official LongMemEval v1 data. "
                "Final QA accuracy still requires the official evaluator with GPT-4o.",
                "",
            ]
        ),
        encoding="utf-8",
    )
    print(json.dumps(report, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
