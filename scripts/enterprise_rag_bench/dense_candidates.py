#!/usr/bin/env python3
"""Dense / hybrid candidate generation for EnterpriseRAG-Bench (EPIC-01).

Turns the corpus document vectors (from ``embed_corpus.py``) into retrieval
candidates:

1. embed each question with the same embedding endpoint (cached),
2. cosine-search the question vector against every corpus document vector,
3. optionally fuse the dense ranking with a lexical retrieval list via RRF,
4. write a **clean** retrieval JSONL (``question_id`` / ``question`` / ``answer`` /
   ``document_ids``) that the official-clean answer stage can read directly.

Unlike embedding *rerank* (which only reorders lexical hits), this finds
semantically-relevant documents the lexical index missed entirely — the piece
that lifts ``semantic`` recall off zero.

Oracle-clean: reads only question text and corpus vectors, never gold labels.

Example
-------
    python3 scripts/enterprise_rag_bench/dense_candidates.py \
      --questions target/enterprise-rag-bench/official-clean/50/.../questions.clean.jsonl \
      --corpus-vectors target/enterprise-rag-bench/embeddings/corpus_bge_m3.jsonl \
      --lexical-retrieval .../retrieval.clean.jsonl \
      --output .../retrieval.dense_hybrid.jsonl \
      --env-file .env --dense-top-k 100 --top-k 50
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import time
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))

from rerank_with_embeddings import (  # noqa: E402
    embedding_request,
    load_env_file,
    read_json,
    read_jsonl,
)


def log(message: str) -> None:
    print(f"[dense {time.strftime('%H:%M:%S')}] {message}", flush=True)


def l2_normalize(matrix: np.ndarray) -> np.ndarray:
    norms = np.linalg.norm(matrix, axis=1, keepdims=True)
    norms[norms == 0] = 1.0
    return matrix / norms


def load_corpus(corpus_path: Path) -> tuple[list[str], np.ndarray]:
    """Load corpus {doc_id, vector} rows into (ids, L2-normalized matrix).

    Caches the parsed matrix next to the JSONL as ``.npy`` + ``.ids.json`` so
    repeated runs skip the multi-gigabyte parse.
    """
    npy = corpus_path.with_suffix(".npy")
    ids_path = corpus_path.with_suffix(".ids.json")
    if (
        npy.exists()
        and ids_path.exists()
        and npy.stat().st_mtime >= corpus_path.stat().st_mtime
    ):
        log(f"loading cached matrix {npy}")
        matrix = np.load(npy)
        ids = json.loads(ids_path.read_text(encoding="utf-8"))
        log(f"corpus matrix {matrix.shape} (cached)")
        return ids, matrix

    log(f"parsing corpus vectors {corpus_path} (first run; will cache .npy)")
    ids: list[str] = []
    vectors: list[list[float]] = []
    with corpus_path.open("r", encoding="utf-8") as handle:
        for line_no, line in enumerate(handle, 1):
            line = line.strip()
            if not line:
                continue
            row = json.loads(line)
            doc_id = row.get("doc_id")
            vector = row.get("vector")
            if not isinstance(doc_id, str) or not isinstance(vector, list):
                continue
            ids.append(doc_id)
            vectors.append(vector)
            if line_no % 50000 == 0:
                log(f"  parsed {line_no} vectors")
    matrix = l2_normalize(np.asarray(vectors, dtype=np.float32))
    log(f"corpus matrix {matrix.shape}; caching to {npy}")
    np.save(npy, matrix)
    ids_path.write_text(json.dumps(ids), encoding="utf-8")
    return ids, matrix


def embed_queries(
    questions: list[dict],
    *,
    url: str,
    model: str,
    api_key: str,
    timeout: float,
    batch_size: int,
    cache_path: Path | None,
) -> dict[str, np.ndarray]:
    cache: dict[str, list[float]] = {}
    if cache_path and cache_path.exists():
        for row in read_jsonl(cache_path):
            qid, vec = row.get("question_id"), row.get("vector")
            if isinstance(qid, str) and isinstance(vec, list):
                cache[qid] = vec
    missing = [q for q in questions if str(q["question_id"]) not in cache]
    if missing:
        log(f"embedding {len(missing)} query vectors")
        for start in range(0, len(missing), batch_size):
            batch = missing[start : start + batch_size]
            vectors = embedding_request(
                [str(q.get("question", "")) for q in batch],
                url=url,
                model=model,
                api_key=api_key,
                timeout=timeout,
            )
            for q, vec in zip(batch, vectors):
                cache[str(q["question_id"])] = vec
            if cache_path:
                cache_path.parent.mkdir(parents=True, exist_ok=True)
                with cache_path.open("a", encoding="utf-8") as handle:
                    for q, vec in zip(batch, vectors):
                        handle.write(
                            json.dumps({"question_id": str(q["question_id"]), "vector": vec})
                            + "\n"
                        )
    out: dict[str, np.ndarray] = {}
    for qid, vec in cache.items():
        arr = np.asarray(vec, dtype=np.float32)
        norm = np.linalg.norm(arr)
        out[qid] = arr / norm if norm else arr
    return out


def dense_topk(qvec: np.ndarray, matrix: np.ndarray, ids: list[str], k: int) -> list[str]:
    scores = matrix @ qvec
    k = min(k, len(ids))
    top = np.argpartition(-scores, k - 1)[:k]
    top = top[np.argsort(-scores[top])]
    return [ids[i] for i in top]


def rrf_fuse(rank_lists: list[tuple[list[str], float]], k: int, rrf_k: int) -> list[str]:
    """Reciprocal-rank fusion of weighted ranked doc-id lists."""
    scores: dict[str, float] = {}
    for docs, weight in rank_lists:
        for rank, doc_id in enumerate(docs):
            scores[doc_id] = scores.get(doc_id, 0.0) + weight / (rrf_k + rank + 1)
    ranked = sorted(scores.items(), key=lambda item: (-item[1], item[0]))
    return [doc_id for doc_id, _ in ranked[:k]]


def main() -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("--questions", type=Path, required=True, help="Clean questions JSONL.")
    parser.add_argument("--corpus-vectors", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True, help="Clean retrieval JSONL out.")
    parser.add_argument("--lexical-retrieval", type=Path, help="Clean lexical retrieval to fuse (RRF).")
    parser.add_argument("--env-file", type=Path, default=Path(".env"))
    parser.add_argument("--embedding-url", default=None)
    parser.add_argument("--embedding-model", default=None)
    parser.add_argument("--embedding-api-key", default=None)
    parser.add_argument("--query-cache", type=Path, help="Cache for query vectors.")
    parser.add_argument("--dense-top-k", type=int, default=100, help="Dense candidates pulled per question.")
    parser.add_argument("--top-k", type=int, default=50, help="Final candidates written per question.")
    parser.add_argument("--dense-weight", type=float, default=1.0)
    parser.add_argument("--lexical-weight", type=float, default=1.0)
    parser.add_argument("--rrf-k", type=int, default=10)
    parser.add_argument("--batch-size", type=int, default=32)
    parser.add_argument("--timeout-seconds", type=float, default=120.0)
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()

    load_env_file(args.env_file)
    url = args.embedding_url or os.environ.get("CORTEXDB_EMBEDDING_URL", "")
    model = args.embedding_model or os.environ.get("CORTEXDB_EMBEDDING_MODEL", "")
    api_key = args.embedding_api_key or os.environ.get("CORTEXDB_EMBEDDING_API_KEY", "")
    if not url or not model or not api_key:
        log("ERROR: embedding url/model/api key missing")
        return 1

    questions = read_jsonl(args.questions)
    log(f"questions={len(questions)} dense_top_k={args.dense_top_k} top_k={args.top_k}")

    ids, matrix = load_corpus(args.corpus_vectors)
    qvecs = embed_queries(
        questions,
        url=url,
        model=model,
        api_key=api_key,
        timeout=args.timeout_seconds,
        batch_size=args.batch_size,
        cache_path=args.query_cache,
    )

    lexical: dict[str, list[str]] = {}
    if args.lexical_retrieval:
        for row in read_jsonl(args.lexical_retrieval):
            lexical[str(row.get("question_id"))] = [str(d) for d in row.get("document_ids", [])]

    out_rows = []
    fused_count = 0
    for q in questions:
        qid = str(q["question_id"])
        qvec = qvecs.get(qid)
        dense = dense_topk(qvec, matrix, ids, args.dense_top_k) if qvec is not None else []
        lex = lexical.get(qid, [])
        if lex:
            docs = rrf_fuse(
                [(dense, args.dense_weight), (lex, args.lexical_weight)],
                k=args.top_k,
                rrf_k=args.rrf_k,
            )
            fused_count += 1
        else:
            docs = dense[: args.top_k]
        out_rows.append(
            {
                "answer": "",
                "document_ids": docs,
                "question": str(q.get("question", "")),
                "question_id": qid,
            }
        )

    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", encoding="utf-8") as handle:
        for row in out_rows:
            handle.write(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n")
    log(f"wrote {len(out_rows)} rows -> {args.output} (hybrid fused={fused_count}, dense-only={len(out_rows) - fused_count})")

    if args.report:
        report = {
            "schema_version": "cortexdb.enterprise_rag_bench.dense_candidates.v1",
            "questions": len(out_rows),
            "corpus_vectors": str(args.corpus_vectors),
            "corpus_size": len(ids),
            "dense_top_k": args.dense_top_k,
            "final_top_k": args.top_k,
            "mode": "hybrid_rrf" if args.lexical_retrieval else "dense_only",
            "fused_questions": fused_count,
            "dense_weight": args.dense_weight,
            "lexical_weight": args.lexical_weight,
            "rrf_k": args.rrf_k,
        }
        args.report.parent.mkdir(parents=True, exist_ok=True)
        args.report.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
