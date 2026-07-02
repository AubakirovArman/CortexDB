#!/usr/bin/env python3
"""CortexDB user-path demo: how a user actually loads data and queries it.

Unlike the EnterpriseRAG bench binary (which uses a special retrieval index and
bypasses the product), this script exercises the *real* CortexDB surfaces:

  load   : `cortexdb load-fixture` (bulk ingest of knowledge cells)
  query  : `cortexdb context`      (ContextPack: ranked, cited, token-budgeted,
                                     permission-scoped via AgentView)
  verify : `cortexdb verify`       (deterministic numeric/lexical fact check)
  answer : the configured LLM (gemma via vLLM) answers from the ContextPack only

It demonstrates that the engine work (ContextPack, AgentView, VERIFY, budget,
citations, conflict detection) is what serves an agent — not a benchmark adapter.

Scope note (honesty): the document pool is the union of candidate doc ids for
the selected questions, ingested into a fresh DB. So this measures the
load/query *mechanics* end to end, not full-corpus recall (that needs all 511k
docs ingested). Recall here is "did ContextPack surface the answer doc from the
ingested pool".
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
import urllib.request
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
GENERATED = ROOT / "target/external-benchmarks/EnterpriseRAG-Bench/generated_data"
CORPUS = GENERATED / "sources"  # uuid_index paths are relative to sources/
UUID_INDEX = GENERATED / "uuid_index.json"
SCOPE = "docs"


def log(msg: str) -> None:
    print(f"[user-path] {msg}", flush=True)


def load_env(path: Path) -> dict[str, str]:
    env: dict[str, str] = {}
    if not path.exists():
        return env
    for line in path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if "=" in line and not line.startswith("#"):
            key, value = line.split("=", 1)
            env[key.strip()] = value.strip().strip('"')
    return env


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]


def doc_text(rel_path: str) -> str:
    """Resolve a corpus document to plain text for ingestion."""
    full = CORPUS / rel_path
    if not full.exists():
        return ""
    raw = full.read_text(encoding="utf-8", errors="ignore")
    if full.suffix == ".json":
        try:
            obj = json.loads(raw)
        except json.JSONDecodeError:
            return raw
        return json_to_text(obj)
    return raw


def json_to_text(obj: Any, depth: int = 0) -> str:
    """Flatten a structured SaaS record into readable text."""
    if depth > 6:
        return ""
    if isinstance(obj, dict):
        parts = []
        for key, value in obj.items():
            text = json_to_text(value, depth + 1)
            if text:
                parts.append(f"{key}: {text}")
        return "\n".join(parts)
    if isinstance(obj, list):
        return "\n".join(t for t in (json_to_text(v, depth + 1) for v in obj) if t)
    return str(obj)


def chunk_text(text: str, chunk_chars: int, max_chars: int) -> list[str]:
    """Split a document into overlapping line-aware chunks (small but complete).

    Small chunks let many candidates fit a token budget (recall) while each chunk
    still carries the full local fact (correctness) — breaking the recall vs
    correctness tradeoff that whole-doc truncation caused.
    """
    text = text[:max_chars]
    overlap = max(0, chunk_chars // 6)
    chunks: list[str] = []
    start = 0
    n = len(text)
    while start < n:
        end = min(n, start + chunk_chars)
        # prefer to break on a newline near the end for cleaner chunks
        nl = text.rfind("\n", start + chunk_chars // 2, end)
        if nl != -1 and end < n:
            end = nl
        chunk = text[start:end].strip()
        if chunk:
            chunks.append(chunk)
        if end >= n:
            break
        start = max(end - overlap, start + 1)
    return chunks or [text]


def aql_escape(task: str) -> str:
    # AQL task is a double-quoted string literal; keep it simple and safe.
    return task.replace("\\", " ").replace('"', "'").replace("\n", " ").strip()[:400]


CLI_BIN = ROOT / "target/debug/cortexdb"


def cli(args: list[str], timeout: int = 300, retries: int = 1) -> tuple[int, str, str]:
    # Use the prebuilt binary directly (avoids per-call cargo overhead and the
    # lock-timing jitter that caused intermittent empty results). Retry on
    # failure or empty stdout to absorb transient db.lock contention between
    # rapid sequential opens.
    cmd = [str(CLI_BIN), *args] if CLI_BIN.exists() else ["cargo", "run", "-q", "-p", "cortex-cli", "--", *args]
    rc, out, err = 1, "", ""
    for attempt in range(max(1, retries)):
        proc = subprocess.run(
            cmd, cwd=ROOT, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=timeout
        )
        rc, out, err = proc.returncode, proc.stdout, proc.stderr
        if rc == 0 and out.strip():
            return rc, out, err
        time.sleep(0.4 * (attempt + 1))
    return rc, out, err


def gemma_answer(env: dict[str, str], question: str, context: str, max_tokens: int) -> str:
    url = env["VLLM_URL"].rstrip("/")
    if not url.endswith("/chat/completions"):
        url = url + ("/chat/completions" if url.endswith("/v1") else "/v1/chat/completions")
    prompt = (
        "You answer questions using the retrieved enterprise documents below. "
        "Find and state the exact names, numbers, units, paths, dates and identifiers "
        "that answer the question; copy literal values exactly (e.g. 10 MiB, not 10MB). "
        "Answer directly even if only part of the requested detail is present. "
        "Reply exactly 'Insufficient information.' ONLY when none of the documents "
        "mention the question's topic at all.\n\n"
        f"Question:\n{question}\n\nRetrieved context:\n{context}\n\nFinal answer:"
    )
    body = json.dumps(
        {
            "model": env["VLLM_MODEL"],
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": max_tokens,
            "temperature": 0.0,
        }
    ).encode()
    req = urllib.request.Request(
        url,
        data=body,
        headers={"Authorization": f"Bearer {env['VLLM_API_KEY']}", "Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=120) as resp:
        data = json.load(resp)
    return data["choices"][0]["message"]["content"].strip()


def build_pool(questions: list[dict], retrieval: dict[str, dict], gold: dict[str, set], per_q: int) -> list[str]:
    pool: list[str] = []
    seen: set[str] = set()
    for q in questions:
        qid = q["question_id"]
        ids: list[str] = []
        row = retrieval.get(qid)
        if row:
            ids.extend(str(d) for d in row.get("document_ids", [])[:per_q])
        ids.extend(gold.get(qid, set()))  # ensure answerable docs are in the pool
        for dsid in ids:
            if dsid not in seen:
                seen.add(dsid)
                pool.append(dsid)
    return pool


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--questions", required=True, help="clean questions jsonl (question_id, question)")
    ap.add_argument("--retrieval", required=True, help="retrieval jsonl for the candidate doc pool")
    ap.add_argument("--gold", required=True, help="original questions.jsonl with expected_doc_ids")
    ap.add_argument("--db", default="target/cortexdb-user-path/db")
    ap.add_argument("--fixture", default="target/cortexdb-user-path/fixture")
    ap.add_argument("--out", default="target/cortexdb-user-path/answers.jsonl")
    ap.add_argument("--limit", type=int, default=50)
    ap.add_argument("--pool-per-q", type=int, default=20)
    ap.add_argument("--context-limit", type=int, default=10)
    ap.add_argument("--budget", type=int, default=4000, help="ContextPack token budget")
    ap.add_argument("--doc-chars", type=int, default=6000, help="max chars per ingested doc (whole or pre-chunk)")
    ap.add_argument("--chunk-chars", type=int, default=0, help="if >0, split docs into chunks of this size")
    ap.add_argument("--max-tokens", type=int, default=512)
    ap.add_argument("--verify", action="store_true", help="also run VERIFY FACT on the gemma answer's first line")
    args = ap.parse_args()

    env = load_env(ROOT / ".env")
    for key in ("VLLM_URL", "VLLM_API_KEY", "VLLM_MODEL"):
        if not env.get(key):
            log(f"missing {key} in .env — cannot call gemma")
            return 1

    questions = read_jsonl(Path(args.questions))[: args.limit]
    retrieval = {r["question_id"]: r for r in read_jsonl(Path(args.retrieval))}
    gold_rows = read_jsonl(Path(args.gold))
    gold = {
        r["question_id"]: {str(d) for d in r.get("expected_doc_ids", [])}
        for r in gold_rows
        if r.get("expected_doc_ids")
    }
    log(f"questions={len(questions)} retrieval_rows={len(retrieval)} gold_rows={len(gold)}")

    # ---- LOAD (real user path): build a fixture and bulk-ingest it ----
    idx = json.loads(UUID_INDEX.read_text(encoding="utf-8"))
    pool = build_pool(questions, retrieval, gold, args.pool_per_q)
    log(f"document pool = {len(pool)} unique docs; building fixture")
    fixture_dir = ROOT / args.fixture
    fixture_dir.mkdir(parents=True, exist_ok=True)
    dsid_by_cell: dict[int, str] = {}
    with (fixture_dir / "cells.jsonl").open("w", encoding="utf-8") as fh:
        cid = 0
        for dsid in pool:
            rel = idx.get(dsid)
            if not rel:
                continue
            text = doc_text(rel).strip()
            if not text:
                continue
            chunks = chunk_text(text, args.chunk_chars, args.doc_chars) if args.chunk_chars > 0 else [text[: args.doc_chars]]
            for ci, chunk in enumerate(chunks):
                cid += 1
                dsid_by_cell[cid] = dsid
                # Child chunk cells keep parent_id=dsid so ContextPack parent
                # expansion + dedup work; source carries the dsid for citations.
                meta = (
                    f"scope={SCOPE}\nstatus=ready\ntype=document\nsource={dsid}\n"
                    f"document_id={dsid}\nparent_id={dsid}\nchunk_role=child\nchunk_id={dsid}#{ci}\n"
                )
                fh.write(json.dumps({"cell_id": cid, "payload": f"{meta}\n{chunk}"}, ensure_ascii=True) + "\n")
    log(f"fixture cells={cid}")

    db = ROOT / args.db
    if db.exists():
        subprocess.run(["rm", "-rf", str(db)], check=False)
    db.parent.mkdir(parents=True, exist_ok=True)
    log("ingest via `cortexdb load-fixture` ...")
    rc, out, err = cli(["load-fixture", str(db), str(fixture_dir)], timeout=900)
    if rc != 0:
        log(f"load-fixture failed: {err[:400]}")
        return 1
    log(f"ingested: {out.strip()[:200]}")

    # ---- QUERY (real user path): ContextPack per question, gemma answers ----
    out_path = ROOT / args.out
    out_path.parent.mkdir(parents=True, exist_ok=True)
    hits = 0
    answered = 0
    conflicts = 0
    sample = None
    with out_path.open("w", encoding="utf-8") as fh:
        for i, q in enumerate(questions, 1):
            qid, qtext = q["question_id"], q["question"]
            task = aql_escape(qtext)
            aql = (
                f'RETRIEVE CONTEXT FOR TASK "{task}" IN BRAIN default '
                f'LIMIT {args.context_limit} CANDIDATES BUDGET {args.budget} TOKENS;'
            )
            rc, out, err = cli(["context", str(db), SCOPE, aql, "--json"], timeout=300, retries=4)
            cells = []
            if rc == 0 and out.strip():
                try:
                    pack = json.loads(out)
                    cells = pack.get("cells", [])
                except json.JSONDecodeError:
                    pass
            ctx_parts, doc_ids = [], []
            for c in cells:
                src = c.get("citation") or extract_source(c.get("payload_text", ""))
                if src:
                    doc_ids.append(src)
                ctx_parts.append(c.get("payload_text", ""))
            context = "\n\n---\n\n".join(ctx_parts)[:16000]

            try:
                answer = gemma_answer(env, qtext, context, args.max_tokens) if context else "Insufficient information."
            except Exception as exc:  # noqa: BLE001
                answer = "Insufficient information."
                log(f"gemma error q={qid}: {str(exc)[:120]}")

            if "insufficient information" not in answer.lower():
                answered += 1
            g = gold.get(qid, set())
            if g and (g & set(doc_ids)):
                hits += 1

            conflict_flag = False
            if args.verify and answer and "insufficient" not in answer.lower():
                first = answer.splitlines()[0][:200].replace('"', "'")
                rc2, out2, _ = cli(
                    ["verify", str(db), SCOPE, f'VERIFY FACT "{first}" IN BRAIN default;', "--json"],
                    timeout=120, retries=2,
                )
                if rc2 == 0 and out2.strip():
                    try:
                        v = json.loads(out2)
                        if v.get("numeric_conflicts") or v.get("verdict") in ("mixed_evidence", "contradicted"):
                            conflict_flag = True
                            conflicts += 1
                    except json.JSONDecodeError:
                        pass

            fh.write(
                json.dumps(
                    {
                        "question_id": qid,
                        "answer": answer,
                        "document_ids": doc_ids,
                        "contextpack_cells": len(cells),
                        "conflict_flagged": conflict_flag,
                    },
                    ensure_ascii=True,
                )
                + "\n"
            )
            if sample is None and cells:
                sample = {"qid": qid, "q": qtext[:120], "cells": len(cells),
                          "citations": doc_ids[:3], "answer": answer[:200]}
            log(f"[{i}/{len(questions)}] {qid} cells={len(cells)} docs={len(doc_ids)} "
                f"gold_hit={'Y' if g and (g & set(doc_ids)) else '-'} conflict={'Y' if conflict_flag else '-'}")

    pool_recall = hits / max(1, sum(1 for q in questions if gold.get(q['question_id'])))
    log("=" * 60)
    log(f"DONE  questions={len(questions)}  answered={answered}  "
        f"pool_recall(gold in ContextPack)={pool_recall*100:.1f}%  conflicts_flagged={conflicts}")
    log(f"answers: {out_path}")
    if sample:
        log(f"sample: q={sample['q']!r}")
        log(f"        ContextPack cells={sample['cells']} citations={sample['citations']}")
        log(f"        gemma: {sample['answer']!r}")
    return 0


def extract_source(payload: str) -> str:
    for line in payload.splitlines():
        if line.startswith("source="):
            return line[len("source="):].strip()
    return ""


if __name__ == "__main__":
    sys.exit(main())
