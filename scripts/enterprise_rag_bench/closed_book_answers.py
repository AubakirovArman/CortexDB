#!/usr/bin/env python3
"""Closed-book ablation answerer for EnterpriseRAG-Bench (EPIC-01 control).

The same answer model answers each question from ITS OWN knowledge, with NO
retrieved documents. Comparing this against the retrieval-backed run isolates
the database's contribution: if closed-book collapses while CortexDB-backed
scores high, the retrieved context — not the model's parametric memory — is what
produces correct answers.

Output matches the official-clean answers schema so the existing judge can score
it. Oracle-clean: reads only question text.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import threading
import time
import urllib.error
import urllib.request
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from rerank_with_embeddings import load_env_file, read_jsonl  # noqa: E402

PROMPT = """You answer enterprise knowledge-base questions about a specific company.
No documents are available to you — answer ONLY from your own prior knowledge.
If you do not actually know the specific company-internal answer, reply exactly:
Insufficient information.
Do not invent specifics.

Question:
{q}

Answer:"""

# HyDE: write a plausible passage as if it were the document that answers the
# question. Used as a dense-retrieval query (embedding of a hypothetical answer
# is closer to the real source doc than the bare question). Never refuse.
HYDE_PROMPT = """Write a short, specific passage (2-4 sentences) that would plausibly
appear in an internal company document and that directly answers the question
below. Invent concrete-sounding details (names, dates, metrics, paths) in the
style of a real enterprise doc. Do NOT say you lack information — always write a
passage.

Question:
{q}

Passage:"""


def normalize_base(url: str) -> str:
    base = url.strip().rstrip("/")
    for suffix in ("/chat/completions", "/embeddings"):
        if base.endswith(suffix):
            base = base[: -len(suffix)]
    return base


def chat(prompt: str, *, url: str, key: str, model: str, timeout: float) -> str:
    payload = {
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "temperature": 0,
        "max_tokens": 420,
    }
    req = urllib.request.Request(
        normalize_base(url) + "/chat/completions",
        data=json.dumps(payload).encode("utf-8"),
        headers={"Authorization": "Bearer " + key, "Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=timeout) as resp:
        body = json.loads(resp.read().decode("utf-8"))
    return str(body["choices"][0]["message"].get("content", "")).strip()


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--questions", type=Path, required=True, help="Clean questions JSONL.")
    ap.add_argument("--output", type=Path, required=True)
    ap.add_argument("--env-file", type=Path, default=Path(".env"))
    ap.add_argument("--workers", type=int, default=4)
    ap.add_argument("--timeout-seconds", type=float, default=120.0)
    ap.add_argument("--progress-every", type=int, default=10)
    ap.add_argument("--mode", choices=["closed-book", "hyde"], default="closed-book")
    args = ap.parse_args()
    prompt_template = HYDE_PROMPT if args.mode == "hyde" else PROMPT

    load_env_file(args.env_file)
    url = os.environ.get("VLLM_URL", "").strip().rstrip("/")
    key = os.environ.get("VLLM_API_KEY", "").strip()
    model = os.environ.get("VLLM_MODEL", "google/gemma-4-31B-it").strip()
    if not url:
        print("ERROR: VLLM_URL missing in env"); return 1

    questions = read_jsonl(args.questions)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    lock = threading.Lock()
    done = 0
    started = time.time()

    def work(q: dict) -> dict:
        qid = str(q["question_id"])
        try:
            ans = chat(prompt_template.format(q=q.get("question", "")), url=url, key=key, model=model, timeout=args.timeout_seconds)
        except Exception as error:  # noqa: BLE001
            ans = "Insufficient information."
            print(f"  warn {qid}: {error}", flush=True)
        return {"answer": ans, "document_ids": [], "question": str(q.get("question", "")), "question_id": qid,
                "model": model, "context_mode": args.mode, "prompt_style": f"{args.mode}-v1"}

    rows: list[dict] = []
    with ThreadPoolExecutor(max_workers=args.workers) as ex:
        futs = {ex.submit(work, q): q for q in questions}
        for fut in as_completed(futs):
            rows.append(fut.result())
            with lock:
                done += 1
                if args.progress_every and done % args.progress_every == 0:
                    print(f"closed-book {done}/{len(questions)} elapsed={time.time()-started:.0f}s", flush=True)

    rows.sort(key=lambda r: r["question_id"])
    with args.output.open("w", encoding="utf-8") as h:
        for r in rows:
            h.write(json.dumps(r, ensure_ascii=True) + "\n")
    print(f"wrote {len(rows)} closed-book answers -> {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
