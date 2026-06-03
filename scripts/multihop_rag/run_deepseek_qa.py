#!/usr/bin/env python3
"""Generate MultiHop-RAG QA answers with DeepSeek flash."""

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

from qa_prompting import (
    build_comparison_decomposition_retry_prompt,
    build_comparison_retry_prompt,
    build_prompt,
    build_temporal_abstention_retry_prompt,
    build_temporal_decomposition_retry_prompt,
)


DEFAULT_BASE_URL = "https://api.deepseek.com"
DEFAULT_MODEL = "deepseek-v4-flash"


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


def is_insufficient_answer(answer: str) -> bool:
    return " ".join(answer.lower().split()) in {"insufficient information", "insufficient info"}


def is_no_answer(answer: str) -> bool:
    return " ".join(answer.lower().split()).strip(".") == "no"


def is_yes_answer(answer: str) -> bool:
    return " ".join(answer.lower().split()).strip(".") == "yes"


def merge_usage(left: dict[str, Any], right: dict[str, Any]) -> dict[str, Any]:
    merged = dict(left)
    for key in [
        "prompt_tokens",
        "completion_tokens",
        "total_tokens",
        "prompt_cache_hit_tokens",
        "prompt_cache_miss_tokens",
    ]:
        merged[key] = int(left.get(key, 0) or 0) + int(right.get(key, 0) or 0)
    return merged


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
        req = urllib.request.Request(
            url,
            data=data,
            headers={"Authorization": "Bearer " + api_key, "Content-Type": "application/json"},
        )
        try:
            started = time.perf_counter()
            with urllib.request.urlopen(req, timeout=120) as response:
                body = json.loads(response.read().decode("utf-8"))
            elapsed_ms = int((time.perf_counter() - started) * 1000)
            choice = body["choices"][0]
            message = choice["message"]
            return str(message.get("content", "")).strip(), body.get("usage", {}), elapsed_ms
        except urllib.error.HTTPError as error:
            if error.code not in {429, 500, 502, 503, 504} or attempt >= retries:
                detail = error.read().decode("utf-8", errors="replace")[:500]
                raise RuntimeError(f"chat request failed: http={error.code} detail={detail}") from error
            time.sleep(min(30, 2 ** attempt))
        except (
            TimeoutError,
            urllib.error.URLError,
            http.client.IncompleteRead,
            http.client.RemoteDisconnected,
            json.JSONDecodeError,
        ) as error:
            if attempt >= retries:
                raise RuntimeError(f"chat request failed after retryable transport error: {error}") from error
            time.sleep(min(30, 2 ** attempt))
    raise RuntimeError("unreachable retry state")


def run(args: argparse.Namespace) -> dict[str, Any]:
    api_key = args.api_key_file.read_text(encoding="utf-8").strip()
    rows = read_json(args.retrieval_file)
    if args.question_type:
        rows = [row for row in rows if row.get("question_type") == args.question_type]
    if args.max_queries is not None:
        rows = rows[: args.max_queries]
    output_root = args.output_root
    jsonl_path = output_root / "deepseek_qa.jsonl"
    json_path = output_root / "deepseek_qa.json"
    report_path = output_root / "deepseek_qa_report.json"
    existing = {query_key(row): row for row in read_jsonl(jsonl_path)}
    usage_lock = threading.Lock()
    output_lock = threading.Lock()
    started = time.perf_counter()
    usage_totals = {
        "prompt_tokens": 0,
        "completion_tokens": 0,
        "total_tokens": 0,
        "prompt_cache_hit_tokens": 0,
        "prompt_cache_miss_tokens": 0,
        "elapsed_ms": 0,
        "completed": len(existing),
        "temporal_abstention_retries": 0,
        "temporal_decomposition_retries": 0,
        "comparison_retries": 0,
    }

    def generate_one(index: int, row: dict[str, Any]) -> tuple[str, dict[str, Any], dict[str, Any], int, int, int, int]:
        prompt = build_prompt(row, args.top_k_context, args.max_chars_per_doc, args.prompt_style)
        answer, usage, elapsed_ms = chat(
            api_key=api_key,
            base_url=args.base_url,
            model=args.model,
            prompt=prompt,
            max_tokens=args.max_tokens,
            retries=args.retries,
        )
        temporal_retry_count = 0
        temporal_decomposition_retry_count = 0
        comparison_retry_count = 0
        saved_extra = {}
        if (
            args.temporal_decomposition_retry
            and row.get("question_type") == "temporal_query"
            and (is_yes_answer(answer) or is_no_answer(answer) or is_insufficient_answer(answer))
        ):
            retry_prompt = build_temporal_decomposition_retry_prompt(
                row,
                args.top_k_context,
                args.max_chars_per_doc,
            )
            retry_answer, retry_usage, retry_elapsed_ms = chat(
                api_key=api_key,
                base_url=args.base_url,
                model=args.model,
                prompt=retry_prompt,
                max_tokens=args.max_tokens,
                retries=args.retries,
            )
            saved_extra = {
                "initial_model_answer": answer,
                "temporal_decomposition_retry_used": True,
                "temporal_decomposition_retry_prompt": retry_prompt,
            }
            answer = retry_answer
            usage = merge_usage(usage, retry_usage)
            elapsed_ms += retry_elapsed_ms
            temporal_decomposition_retry_count = 1
        elif (
            args.temporal_abstention_retry
            and row.get("question_type") == "temporal_query"
            and is_insufficient_answer(answer)
        ):
            retry_prompt = build_temporal_abstention_retry_prompt(
                row,
                args.top_k_context,
                args.max_chars_per_doc,
            )
            retry_answer, retry_usage, retry_elapsed_ms = chat(
                api_key=api_key,
                base_url=args.base_url,
                model=args.model,
                prompt=retry_prompt,
                max_tokens=args.max_tokens,
                retries=args.retries,
            )
            saved_extra = {
                "initial_model_answer": answer,
                "abstention_retry_used": True,
                "abstention_retry_prompt": retry_prompt,
            }
            answer = retry_answer
            usage = merge_usage(usage, retry_usage)
            elapsed_ms += retry_elapsed_ms
            temporal_retry_count = 1
        elif (
            args.comparison_retry
            and row.get("question_type") == "comparison_query"
            and (is_insufficient_answer(answer) or is_no_answer(answer))
        ):
            if args.comparison_retry_style == "decompose":
                retry_prompt = build_comparison_decomposition_retry_prompt(
                    row,
                    args.top_k_context,
                    args.max_chars_per_doc,
                )
            else:
                retry_prompt = build_comparison_retry_prompt(
                    row,
                    args.top_k_context,
                    args.max_chars_per_doc,
                )
            retry_answer, retry_usage, retry_elapsed_ms = chat(
                api_key=api_key,
                base_url=args.base_url,
                model=args.model,
                prompt=retry_prompt,
                max_tokens=args.max_tokens,
                retries=args.retries,
            )
            saved_extra = {
                "initial_model_answer": answer,
                "comparison_retry_used": True,
                "comparison_retry_prompt": retry_prompt,
            }
            answer = retry_answer
            usage = merge_usage(usage, retry_usage)
            elapsed_ms += retry_elapsed_ms
            comparison_retry_count = 1
        return query_key(row), {
            "query": row.get("query", ""),
            "prompt": prompt,
            "model_answer": answer,
            "gold_answer": row.get("answer", ""),
            "question_type": row.get("question_type", ""),
            "usage": usage,
            "elapsed_ms": elapsed_ms,
            **saved_extra,
        }, usage, elapsed_ms, temporal_retry_count, temporal_decomposition_retry_count, comparison_retry_count

    pending = [(index, row) for index, row in enumerate(rows, 1) if query_key(row) not in existing]
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.workers) as executor:
        futures = [executor.submit(generate_one, index, row) for index, row in pending]
        for future in concurrent.futures.as_completed(futures):
            (
                key,
                saved,
                usage,
                elapsed_ms,
                temporal_retry_count,
                temporal_decomposition_retry_count,
                comparison_retry_count,
            ) = future.result()
            with output_lock:
                if key not in existing:
                    existing[key] = saved
                    append_jsonl(jsonl_path, saved)
            with usage_lock:
                usage_totals["prompt_tokens"] += int(usage.get("prompt_tokens", 0) or 0)
                usage_totals["completion_tokens"] += int(usage.get("completion_tokens", 0) or 0)
                usage_totals["total_tokens"] += int(usage.get("total_tokens", 0) or 0)
                usage_totals["prompt_cache_hit_tokens"] += int(usage.get("prompt_cache_hit_tokens", 0) or 0)
                usage_totals["prompt_cache_miss_tokens"] += int(usage.get("prompt_cache_miss_tokens", 0) or 0)
                usage_totals["elapsed_ms"] += elapsed_ms
                usage_totals["temporal_abstention_retries"] += temporal_retry_count
                usage_totals["temporal_decomposition_retries"] += temporal_decomposition_retry_count
                usage_totals["comparison_retries"] += comparison_retry_count
                usage_totals["completed"] += 1
                if args.progress_every and usage_totals["completed"] % args.progress_every == 0:
                    print(f"generated {usage_totals['completed']}/{len(rows)}")
    ordered = [existing[query_key(row)] for row in rows if query_key(row) in existing]
    write_json(json_path, ordered)
    wall_elapsed_ms = int((time.perf_counter() - started) * 1000)
    cache_tokens = usage_totals["prompt_cache_hit_tokens"] + usage_totals["prompt_cache_miss_tokens"]
    cache_hit_rate = (
        usage_totals["prompt_cache_hit_tokens"] / cache_tokens
        if cache_tokens
        else None
    )
    completion_seconds = usage_totals["elapsed_ms"] / 1000
    completion_tokens_per_second = (
        usage_totals["completion_tokens"] / completion_seconds
        if completion_seconds and usage_totals["completion_tokens"]
        else None
    )
    report = {
        "schema_version": "cortexdb.multihop_rag.deepseek_qa_report.v1",
        "model": args.model,
        "thinking": "disabled",
        "questions": len(ordered),
        "retrieval_file": str(args.retrieval_file),
        "qa_json": str(json_path),
        "workers": args.workers,
        "prompt_style": args.prompt_style,
        "temporal_abstention_retry": args.temporal_abstention_retry,
        "temporal_decomposition_retry": args.temporal_decomposition_retry,
        "comparison_retry": args.comparison_retry,
        "comparison_retry_style": args.comparison_retry_style,
        "prompt_tokens_new": usage_totals["prompt_tokens"],
        "completion_tokens_new": usage_totals["completion_tokens"],
        "total_tokens_new": usage_totals["total_tokens"],
        "prompt_cache_hit_tokens_new": usage_totals["prompt_cache_hit_tokens"],
        "prompt_cache_miss_tokens_new": usage_totals["prompt_cache_miss_tokens"],
        "prompt_cache_hit_rate_new": cache_hit_rate,
        "api_elapsed_ms_sum_new": usage_totals["elapsed_ms"],
        "wall_elapsed_ms": wall_elapsed_ms,
        "completion_tokens_per_second_new": completion_tokens_per_second,
        "temporal_abstention_retries_new": usage_totals["temporal_abstention_retries"],
        "temporal_decomposition_retries_new": usage_totals["temporal_decomposition_retries"],
        "comparison_retries_new": usage_totals["comparison_retries"],
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
    parser.add_argument(
        "--prompt-style",
        choices=["legacy", "multihop-v2", "multihop-v3"],
        default="multihop-v2",
    )
    parser.add_argument("--question-type")
    parser.add_argument("--temporal-abstention-retry", action="store_true")
    parser.add_argument("--temporal-decomposition-retry", action="store_true")
    parser.add_argument("--comparison-retry", action="store_true")
    parser.add_argument(
        "--comparison-retry-style",
        choices=["standard", "decompose"],
        default="standard",
    )
    print(json.dumps(run(parser.parse_args()), sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
