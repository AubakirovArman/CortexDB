#!/usr/bin/env python3
"""Generate EnterpriseRAG-Bench answers from retrieved CortexDB document IDs."""

from __future__ import annotations

import argparse
import concurrent.futures
import http.client
import json
import threading
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any

from answer_prompts import build_prompt
from context_windows import question_aware_snippet
from evidence_digest import evidence_digest, evidence_digest_score
from evidence_span_fallback import evidence_span_plus_fallback_context
from evidence_spans import evidence_span_context


DEFAULT_BASE_URL = "https://api.deepseek.com"
DEFAULT_MODEL = "deepseek-v4-flash"


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line]


def append_jsonl(path: Path, row: dict[str, Any], lock: threading.Lock) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with lock:
        with path.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n")


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(payload, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )


def extract_document_content(doc: dict[str, Any]) -> tuple[str, str]:
    title_field = doc.get("title_field_name")
    content_fields = doc.get("content_field_names")
    if not isinstance(title_field, str) or title_field not in doc:
        return ("", json.dumps(doc, ensure_ascii=False))
    title = str(doc.get(title_field, ""))
    if not isinstance(content_fields, list) or not content_fields:
        return (title, json.dumps(doc, ensure_ascii=False))
    parts: list[str] = []
    for field in content_fields:
        if not isinstance(field, str) or field not in doc:
            continue
        value = doc[field]
        if isinstance(value, list):
            value = "\n".join(str(item) for item in value)
        elif isinstance(value, dict):
            value = json.dumps(value, ensure_ascii=False)
        parts.append(f"{field}:\n{value}" if len(content_fields) > 1 else str(value))
    return (title, "\n\n".join(parts))


def load_context(
    doc_ids: list[str],
    uuid_index: dict[str, str],
    sources_dir: Path,
    max_chars_per_doc: int,
    question: str,
    context_mode: str,
) -> str:
    docs: list[tuple[float, int, str]] = []
    for rank, doc_id in enumerate(doc_ids, 1):
        rel_path = uuid_index.get(doc_id)
        if not rel_path:
            continue
        title, content = extract_document_content(read_json(sources_dir / rel_path))
        if context_mode == "question-window":
            snippet = question_aware_snippet(content, question, max_chars_per_doc)
        elif context_mode == "evidence-spans":
            snippet = evidence_span_context(content, title, question, max_chars_per_doc)
        elif context_mode == "span-plus-fallback":
            snippet = evidence_span_plus_fallback_context(content, title, question, max_chars_per_doc)
        elif context_mode == "question-window-digest":
            digest = evidence_digest(content, title, question)
            snippet_budget = max(1200, max_chars_per_doc - len(digest) - 160)
            snippet = question_aware_snippet(content, question, snippet_budget)
            if digest:
                snippet = f"{digest}\n\nQuestion-aware windows:\n{snippet}"
        elif context_mode == "question-window-digest-ranked":
            digest = evidence_digest(content, title, question)
            snippet_budget = max(1200, max_chars_per_doc - len(digest) - 160)
            snippet = question_aware_snippet(content, question, snippet_budget)
            if digest:
                snippet = f"{digest}\n\nQuestion-aware windows:\n{snippet}"
        else:
            snippet = content[:max_chars_per_doc]
        score = evidence_digest_score(content, question) if "digest-ranked" in context_mode else 0.0
        docs.append((
            score,
            rank,
            f"--- Document {rank} (ID: {doc_id}) ---\n"
            f"Title: {title}\n\n{snippet}",
        ))
    if "digest-ranked" in context_mode:
        docs.sort(key=lambda item: (-item[0], item[1]))
    return "\n\n".join(text for _, _, text in docs)


def chat(
    *,
    api_key: str,
    base_url: str,
    model: str,
    prompt: str,
    max_tokens: int,
    retries: int,
) -> tuple[str, dict[str, Any], int]:
    payload = {
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": max_tokens,
        "temperature": 0,
        "thinking": {"type": "disabled"},
    }
    data = json.dumps(payload).encode("utf-8")
    url = base_url.rstrip("/") + "/chat/completions"
    for attempt in range(retries + 1):
        request = urllib.request.Request(
            url,
            data=data,
            headers={
                "Authorization": "Bearer " + api_key,
                "Content-Type": "application/json",
            },
        )
        try:
            started = time.perf_counter()
            with urllib.request.urlopen(request, timeout=180) as response:
                body = json.loads(response.read().decode("utf-8"))
            elapsed_ms = int((time.perf_counter() - started) * 1000)
            answer = body["choices"][0]["message"].get("content", "")
            return (str(answer).strip(), body.get("usage", {}), elapsed_ms)
        except urllib.error.HTTPError as error:
            if error.code not in {429, 500, 502, 503, 504} or attempt >= retries:
                detail = error.read().decode("utf-8", errors="replace")[:500]
                raise RuntimeError(f"chat request failed: http={error.code} {detail}") from error
            time.sleep(min(30, 2**attempt))
        except (
            TimeoutError,
            urllib.error.URLError,
            http.client.IncompleteRead,
            http.client.RemoteDisconnected,
            json.JSONDecodeError,
        ) as error:
            if attempt >= retries:
                raise RuntimeError(f"chat request failed: {error}") from error
            time.sleep(min(30, 2**attempt))
    raise RuntimeError("unreachable retry state")


def run(args: argparse.Namespace) -> dict[str, Any]:
    api_key = args.api_key_file.read_text(encoding="utf-8").strip()
    rows = read_jsonl(args.retrieval_file)
    if args.limit is not None:
        rows = rows[: args.limit]
    uuid_index = read_json(args.uuid_index)
    output_jsonl = args.output_root / "answers.jsonl"
    output_report = args.output_root / "answer_generation_report.json"
    existing = {row.get("question_id"): row for row in read_jsonl(output_jsonl)}
    output_lock = threading.Lock()
    usage_lock = threading.Lock()
    usage_totals = {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}
    started = time.perf_counter()
    pending = [row for row in rows if row.get("question_id") not in existing]
    if not pending and output_report.exists():
        return read_json(output_report)

    def generate(row: dict[str, Any]) -> dict[str, Any]:
        doc_ids = [str(item) for item in row.get("document_ids", [])]
        question = str(row.get("question", ""))
        context = load_context(
            doc_ids[: args.top_k_context],
            uuid_index,
            args.sources_dir,
            args.max_chars_per_doc,
            question,
            args.context_mode,
        )
        answer, usage, elapsed_ms = chat(
            api_key=api_key,
            base_url=args.base_url,
            model=args.model,
            prompt=build_prompt(row, context, args.prompt_style),
            max_tokens=args.max_tokens,
            retries=args.retries,
        )
        with usage_lock:
            for key in usage_totals:
                usage_totals[key] += int(usage.get(key, 0) or 0)
        return {
            "question_id": row.get("question_id"),
            "answer": answer,
            "document_ids": doc_ids,
            "elapsed_ms": elapsed_ms,
            "prompt_tokens": int(usage.get("prompt_tokens", 0) or 0),
            "completion_tokens": int(usage.get("completion_tokens", 0) or 0),
            "total_tokens": int(usage.get("total_tokens", 0) or 0),
            "model": args.model,
            "context_mode": args.context_mode,
            "prompt_style": args.prompt_style,
        }

    with concurrent.futures.ThreadPoolExecutor(max_workers=args.workers) as executor:
        futures = [executor.submit(generate, row) for row in pending]
        for future in concurrent.futures.as_completed(futures):
            saved = future.result()
            existing[saved["question_id"]] = saved
            append_jsonl(output_jsonl, saved, output_lock)
            if args.progress_every and len(existing) % args.progress_every == 0:
                print(f"generated {len(existing)}/{len(rows)}")

    ordered = [existing[row.get("question_id")] for row in rows if row.get("question_id") in existing]
    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.deepseek_answers_report.v1",
        "model": args.model,
        "thinking": "disabled",
        "context_mode": args.context_mode,
        "prompt_style": args.prompt_style,
        "questions": len(ordered),
        "retrieval_file": str(args.retrieval_file),
        "answers_file": str(output_jsonl),
        "wall_elapsed_ms": int((time.perf_counter() - started) * 1000),
        **usage_totals,
    }
    write_json(output_report, report)
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--retrieval-file", type=Path, required=True)
    parser.add_argument("--uuid-index", type=Path, required=True)
    parser.add_argument("--sources-dir", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--api-key-file", type=Path, required=True)
    parser.add_argument("--base-url", default=DEFAULT_BASE_URL)
    parser.add_argument("--model", default=DEFAULT_MODEL)
    parser.add_argument("--top-k-context", type=int, default=6)
    parser.add_argument("--max-chars-per-doc", type=int, default=1600)
    parser.add_argument("--max-tokens", type=int, default=180)
    parser.add_argument(
        "--prompt-style",
        choices=[
            "baseline",
            "fact-focused-v2",
            "evidence-selection-v5",
            "type-aware-v9",
            "type-aware-v13",
            "type-aware-v15",
            "type-aware-v17",
            "evidence-audit-v11",
        ],
        default="baseline",
    )
    parser.add_argument(
        "--context-mode",
        choices=[
            "leading",
            "evidence-spans",
            "span-plus-fallback",
            "question-window",
            "question-window-digest",
            "question-window-digest-ranked",
        ],
        default="leading",
    )
    parser.add_argument("--workers", type=int, default=1)
    parser.add_argument("--retries", type=int, default=3)
    parser.add_argument("--limit", type=int)
    parser.add_argument("--progress-every", type=int, default=10)
    print(json.dumps(run(parser.parse_args()), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
