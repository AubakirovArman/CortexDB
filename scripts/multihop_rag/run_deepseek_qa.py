#!/usr/bin/env python3
"""Generate MultiHop-RAG QA answers with DeepSeek flash."""

from __future__ import annotations

import argparse
import concurrent.futures
import json
import re
import threading
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any


DEFAULT_BASE_URL = "https://api.deepseek.com"
DEFAULT_MODEL = "deepseek-v4-flash"
WORD_RE = re.compile(r"[A-Za-z0-9]+")
SENTENCE_RE = re.compile(r"(?<=[.!?])\s+")


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line]


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True, ensure_ascii=True) + "\n", encoding="utf-8")


def append_jsonl(path: Path, row: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n")


def query_key(row: dict[str, Any]) -> str:
    return str(row.get("query", ""))


def tokenize(value: str) -> set[str]:
    stop = {
        "the", "and", "for", "with", "from", "that", "this", "what", "which", "who",
        "about", "reported", "article", "according", "information", "another", "both",
    }
    return {word.lower() for word in WORD_RE.findall(value) if len(word) > 2 and word.lower() not in stop}


def payload_parts(payload: str) -> tuple[dict[str, str], str]:
    header, _, body = payload.partition("\n\n")
    metadata: dict[str, str] = {}
    for line in header.splitlines():
        key, sep, value = line.partition("=")
        if sep:
            metadata[key.strip()] = value.strip()
    return metadata, body.strip()


def best_snippet(query: str, payload: str, max_chars: int) -> str:
    metadata, body = payload_parts(payload)
    query_terms = tokenize(query)
    sentences = [sentence.strip() for sentence in SENTENCE_RE.split(body) if sentence.strip()]
    scored = []
    for index, sentence in enumerate(sentences):
        words = tokenize(sentence)
        score = len(query_terms & words)
        if score:
            scored.append((score, -index, sentence))
    selected = [sentence for _, _, sentence in sorted(scored, reverse=True)[:4]]
    if not selected:
        selected = sentences[:4]
    snippet = " ".join(selected)
    prefix = " | ".join(
        value
        for value in [metadata.get("title", ""), metadata.get("source", ""), metadata.get("published_at", "")]
        if value
    )
    text = f"{prefix}\n{snippet}" if prefix else snippet
    return text[:max_chars]


def build_prompt(row: dict[str, Any], top_k: int, max_chars_per_doc: int, prompt_style: str) -> str:
    contexts = []
    for item in row.get("retrieval_list", [])[:top_k]:
        text = str(item.get("text", ""))
        snippet = best_snippet(str(row.get("query", "")), text, max_chars_per_doc)
        if snippet:
            contexts.append(f"[{len(contexts) + 1}]\n{snippet}")
    question_type = str(row.get("question_type", ""))
    if prompt_style == "multihop-v2":
        type_instruction = {
            "comparison_query": (
                "This is a comparison question. If the context supports both sides, "
                "answer with Yes or No."
            ),
            "temporal_query": (
                "This is a temporal question. Compare the dates or event order in "
                "the context and answer with Yes or No."
            ),
            "null_query": (
                "This is a null-query check. Answer Insufficient Information unless "
                "the context directly supports the requested entity or fact."
            ),
            "inference_query": (
                "This is an inference question. Combine the relevant context snippets "
                "and answer with the shortest supported entity, date, number, or phrase."
            ),
        }.get(question_type, "Use only the provided context.")
        return "\n\n".join(
            [
                "Answer the question using only the provided context.",
                type_instruction,
                "Use exactly one short answer.",
                "For yes/no questions, answer exactly Yes or No.",
                "If the context is insufficient, answer exactly: Insufficient Information",
                "Do not explain your reasoning.",
                "",
                f"Question type: {question_type}",
                f"Question: {row.get('query', '')}",
                "",
                "Context:",
                "\n\n".join(contexts),
                "",
                "Answer:",
            ]
        )
    return "\n\n".join(
        [
            "Answer the question using only the provided context.",
            "The answer should be a short entity, name, date, number, or phrase.",
            "If the context is insufficient, answer exactly: Insufficient Information",
            "Do not explain your reasoning.",
            "",
            f"Question: {row.get('query', '')}",
            "",
            "Context:",
            "\n\n".join(contexts),
            "",
            "Answer:",
        ]
    )


def chat(
    *,
    api_key: str,
    base_url: str,
    model: str,
    prompt: str,
    max_tokens: int,
    retries: int,
) -> tuple[str, dict[str, Any]]:
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
        req = urllib.request.Request(
            url,
            data=data,
            headers={"Authorization": "Bearer " + api_key, "Content-Type": "application/json"},
        )
        try:
            with urllib.request.urlopen(req, timeout=120) as response:
                body = json.loads(response.read().decode("utf-8"))
            choice = body["choices"][0]
            message = choice["message"]
            return str(message.get("content", "")).strip(), body.get("usage", {})
        except urllib.error.HTTPError as error:
            if error.code not in {429, 500, 502, 503, 504} or attempt >= retries:
                detail = error.read().decode("utf-8", errors="replace")[:500]
                raise RuntimeError(f"chat request failed: http={error.code} detail={detail}") from error
            time.sleep(min(30, 2 ** attempt))
    raise RuntimeError("unreachable retry state")


def run(args: argparse.Namespace) -> dict[str, Any]:
    api_key = args.api_key_file.read_text(encoding="utf-8").strip()
    rows = read_json(args.retrieval_file)
    if args.max_queries is not None:
        rows = rows[: args.max_queries]
    output_root = args.output_root
    jsonl_path = output_root / "deepseek_qa.jsonl"
    json_path = output_root / "deepseek_qa.json"
    report_path = output_root / "deepseek_qa_report.json"
    existing = {query_key(row): row for row in read_jsonl(jsonl_path)}
    usage_lock = threading.Lock()
    output_lock = threading.Lock()
    usage_totals = {"prompt_tokens": 0, "completion_tokens": 0, "completed": len(existing)}

    def generate_one(index: int, row: dict[str, Any]) -> tuple[str, dict[str, Any], dict[str, Any]]:
        prompt = build_prompt(row, args.top_k_context, args.max_chars_per_doc, args.prompt_style)
        answer, usage = chat(
            api_key=api_key,
            base_url=args.base_url,
            model=args.model,
            prompt=prompt,
            max_tokens=args.max_tokens,
            retries=args.retries,
        )
        return query_key(row), {
            "query": row.get("query", ""),
            "prompt": prompt,
            "model_answer": answer,
            "gold_answer": row.get("answer", ""),
            "question_type": row.get("question_type", ""),
        }, usage

    pending = [(index, row) for index, row in enumerate(rows, 1) if query_key(row) not in existing]
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.workers) as executor:
        futures = [executor.submit(generate_one, index, row) for index, row in pending]
        for future in concurrent.futures.as_completed(futures):
            key, saved, usage = future.result()
            with output_lock:
                if key not in existing:
                    existing[key] = saved
                    append_jsonl(jsonl_path, saved)
            with usage_lock:
                usage_totals["prompt_tokens"] += int(usage.get("prompt_tokens", 0) or 0)
                usage_totals["completion_tokens"] += int(usage.get("completion_tokens", 0) or 0)
                usage_totals["completed"] += 1
                if args.progress_every and usage_totals["completed"] % args.progress_every == 0:
                    print(f"generated {usage_totals['completed']}/{len(rows)}")
    ordered = [existing[query_key(row)] for row in rows if query_key(row) in existing]
    write_json(json_path, ordered)
    report = {
        "schema_version": "cortexdb.multihop_rag.deepseek_qa_report.v1",
        "model": args.model,
        "thinking": "disabled",
        "questions": len(ordered),
        "retrieval_file": str(args.retrieval_file),
        "qa_json": str(json_path),
        "workers": args.workers,
        "prompt_style": args.prompt_style,
        "prompt_tokens_new": usage_totals["prompt_tokens"],
        "completion_tokens_new": usage_totals["completion_tokens"],
    }
    write_json(report_path, report)
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--retrieval-file", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--api-key-file", type=Path, required=True)
    parser.add_argument("--base-url", default=DEFAULT_BASE_URL)
    parser.add_argument("--model", default=DEFAULT_MODEL)
    parser.add_argument("--top-k-context", type=int, default=6)
    parser.add_argument("--max-chars-per-doc", type=int, default=1200)
    parser.add_argument("--max-tokens", type=int, default=80)
    parser.add_argument("--max-queries", type=int)
    parser.add_argument("--progress-every", type=int, default=25)
    parser.add_argument("--retries", type=int, default=3)
    parser.add_argument("--workers", type=int, default=1)
    parser.add_argument("--prompt-style", choices=["legacy", "multihop-v2"], default="multihop-v2")
    print(json.dumps(run(parser.parse_args()), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
