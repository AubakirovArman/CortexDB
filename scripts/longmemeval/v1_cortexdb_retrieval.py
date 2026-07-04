#!/usr/bin/env python3
"""Run CortexDB retrieval on official LongMemEval v1 data."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import shutil
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any

from v1_context_modes import session_text
from v1_official_metrics import evaluate_entry, should_skip_for_aggregate, summarize


def q15_literal(vector: list[float]) -> str:
    """Unit-normalize a float embedding and scale to the i16 Q15 comma-separated
    `vector=` literal the engine parses (`parse_vector_literal`)."""
    norm = math.sqrt(sum(x * x for x in vector)) or 1.0
    q15 = [max(-32767, min(32767, round(x / norm * 32767))) for x in vector]
    return ",".join(str(v) for v in q15)


class Embedder:
    """A6.3: OpenAI-compatible embedding client for LongMemEval hybrid retrieval.

    Embeds text via the `CORTEXDB_EMBEDDING_*` endpoint, unit-normalizes, and
    scales to i16 Q15 comma-separated form (the `vector=` payload literal the
    engine parses). Persists a per-split cache (sha256(text) -> literal) so
    re-runs are cheap + deterministic. Only used in `--retrieval-mode hybrid`;
    keyword mode never constructs one, so its rankings stay byte-identical.
    """

    def __init__(self, url: str, key: str, model: str, cache_path: Path | None) -> None:
        self.url = url
        self.key = key
        self.model = model
        self.cache_path = cache_path
        self.cache: dict[str, str] = {}
        if cache_path and cache_path.exists():
            self.cache = json.loads(cache_path.read_text(encoding="utf-8"))

    def literal(self, text: str) -> str:
        digest = hashlib.sha256(text.encode("utf-8")).hexdigest()
        cached = self.cache.get(digest)
        if cached is not None:
            return cached
        body = json.dumps({"model": self.model, "input": text}).encode("utf-8")
        req = urllib.request.Request(
            self.url,
            data=body,
            headers={"Authorization": "Bearer " + self.key, "Content-Type": "application/json"},
        )
        vec = None
        for attempt in range(4):
            try:
                with urllib.request.urlopen(req, timeout=60) as response:
                    vec = json.loads(response.read().decode("utf-8"))["data"][0]["embedding"]
                break
            except urllib.error.HTTPError as exc:
                if exc.code in {429, 500, 502, 503, 504} and attempt < 3:
                    time.sleep(min(20, 2 ** attempt))
                    continue
                raise
        literal = q15_literal(vec)
        self.cache[digest] = literal
        return literal

    def flush(self) -> None:
        if self.cache_path:
            self.cache_path.parent.mkdir(parents=True, exist_ok=True)
            self.cache_path.write_text(json.dumps(self.cache), encoding="utf-8")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


def load_json_list(path: Path) -> list[dict[str, Any]]:
    value = json.loads(path.read_text(encoding="utf-8"))
    require(isinstance(value, list), f"{path}: expected JSON list")
    return value


def corpus_for_entry(
    entry: dict[str, Any],
    granularity: str,
    index_mode: str,
    context_mode: str,
    max_turn_chars: int,
    max_session_chars: int,
) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for session_id, session, timestamp in zip(
        entry["haystack_session_ids"],
        entry["haystack_sessions"],
        entry["haystack_dates"],
    ):
        require(isinstance(session, list), f"{entry['question_id']}: session must be list")
        if granularity == "session":
            corpus_id = session_id
            turns = [turn for turn in session if turn.get("role") == "user"]
            if "answer" in corpus_id and all(not turn.get("has_answer", False) for turn in turns):
                corpus_id = corpus_id.replace("answer", "noans")
            rows.append(
                {
                    "corpus_id": corpus_id,
                    "index_text": session_text(
                        session, index_mode, max_turn_chars, max_session_chars
                    ),
                    "text": session_text(session, context_mode, max_turn_chars, max_session_chars),
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
                text = str(turn.get("content", "")).strip()
                rows.append(
                    {
                        "corpus_id": corpus_id,
                        "index_text": text,
                        "text": text,
                        "timestamp": timestamp,
                        "raw_session_id": session_id,
                    }
                )
            continue
        raise RuntimeError(f"unsupported granularity: {granularity}")
    return [row for row in rows if row["text"]]


def payload_for_cell(
    question_id: str, row: dict[str, Any], vector_literal: str | None = None
) -> str:
    header = [
        "scope=longmemeval",
        "status=ready",
        "type=memory",
        f"source=longmemeval:{question_id}:{row['corpus_id']}",
    ]
    # A6.3: hybrid mode appends the dense `vector=` line the engine parses. Keyword
    # mode passes `vector_literal=None` -> header is byte-identical to pre-A6.3.
    if vector_literal is not None:
        header.append(f"vector={vector_literal}")
    return "\n".join(
        header
        + [
            "",
            f"LONGMEMEVAL_CORPUS_ID: {row['corpus_id']}",
            f"LONGMEMEVAL_TIMESTAMP: {row['timestamp']}",
            "",
            row["index_text"],
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


def write_fixture(
    fixture_dir: Path,
    question_id: str,
    corpus: list[dict[str, Any]],
    vectors: dict[str, str] | None = None,
) -> None:
    fixture_dir.mkdir(parents=True, exist_ok=True)
    with (fixture_dir / "cells.jsonl").open("w", encoding="utf-8") as handle:
        for index, row in enumerate(corpus, start=1):
            literal = vectors.get(row["corpus_id"]) if vectors else None
            payload = payload_for_cell(question_id, row, literal)
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
    mode: str = "keyword",
    query_vector_literal: str | None = None,
) -> list[str]:
    command = [
        str(cortexdb_bin),
        "--json",
        "search",
        str(db_path),
        "longmemeval",
        query,
        "--mode",
        mode,
    ]
    # A6.3: hybrid mode fuses the keyword score with dense cosine over the query
    # vector (embedded in the harness for deterministic, cache-backed re-runs).
    if mode == "hybrid" and query_vector_literal is not None:
        # `=` form so a leading-negative literal (e.g. -2667,...) is parsed as the
        # value, not mistaken for a CLI flag.
        command.append(f"--vector={query_vector_literal}")
    raw = run_command(command)
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
    parser.add_argument("--index-mode", choices=["user", "conversation", "compact"], default="user")
    parser.add_argument("--context-mode", choices=["user", "conversation", "compact"], default="user")
    parser.add_argument("--max-turn-chars", type=int, default=900)
    parser.add_argument("--max-session-chars", type=int, default=4000)
    parser.add_argument("--top-k", type=int, default=10)
    parser.add_argument("--limit", type=int)
    parser.add_argument("--keep-workdir", action="store_true")
    # A6.3: dense-hybrid retrieval. Default `keyword` keeps rankings byte-identical
    # to the committed F3.1 baseline (no vectors, --mode keyword).
    parser.add_argument("--retrieval-mode", choices=["keyword", "hybrid"], default="keyword")
    parser.add_argument("--embedding-cache", type=Path, default=None)
    args = parser.parse_args(argv)

    require(args.cortexdb_bin.exists(), f"missing cortexdb binary: {args.cortexdb_bin}")
    require(args.top_k > 0, "--top-k must be positive")
    embedder: Embedder | None = None
    if args.retrieval_mode == "hybrid":
        url = os.environ.get("CORTEXDB_EMBEDDING_URL", "")
        key = os.environ.get("CORTEXDB_EMBEDDING_API_KEY", "")
        model = os.environ.get("CORTEXDB_EMBEDDING_MODEL", "")
        require(bool(url and model), "hybrid mode needs CORTEXDB_EMBEDDING_URL + _MODEL")
        cache = args.embedding_cache or (args.output_dir / "embedding_cache.json")
        embedder = Embedder(url, key, model, cache)
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
        corpus = corpus_for_entry(
            entry,
            args.granularity,
            args.index_mode,
            args.context_mode,
            args.max_turn_chars,
            args.max_session_chars,
        )
        q_root = work_root / question_id
        fixture_dir = q_root / "fixture"
        db_path = q_root / "db"
        vectors: dict[str, str] | None = None
        query_vector_literal: str | None = None
        if embedder is not None:
            vectors = {row["corpus_id"]: embedder.literal(row["index_text"]) for row in corpus}
            query_vector_literal = embedder.literal(str(entry["question"]))
        write_fixture(fixture_dir, question_id, corpus, vectors)
        run_command([str(args.cortexdb_bin), "load-fixture", str(db_path), str(fixture_dir)])
        ranked_ids = search_cortexdb(
            args.cortexdb_bin,
            db_path,
            str(entry["question"]),
            args.top_k,
            mode=args.retrieval_mode,
            query_vector_literal=query_vector_literal,
        )
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
        if embedder is not None:
            embedder.flush()
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
        "retrieval_mode": args.retrieval_mode,
        "index_mode": args.index_mode,
        "context_mode": args.context_mode,
        "max_turn_chars": args.max_turn_chars,
        "max_session_chars": args.max_session_chars,
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
                f"- index mode: `{args.index_mode}`",
                f"- context mode: `{args.context_mode}`",
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
