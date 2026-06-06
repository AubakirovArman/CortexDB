#!/usr/bin/env python3
"""Evaluate deterministic chunking policies against real-domain retrieval QA."""

from __future__ import annotations

import argparse
import json
import math
import re
import sys
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Any


TOKEN_RE = re.compile(r"[\w:.-]+", re.UNICODE)
REPO_ROOT = Path(__file__).resolve().parents[1]


@dataclass(frozen=True)
class ChunkPolicy:
    max_chars: int
    overlap_chars: int
    min_chars: int

    @classmethod
    def from_json(cls, value: dict[str, Any]) -> "ChunkPolicy":
        return cls(
            max_chars=int(value["max_chars"]),
            overlap_chars=int(value["overlap_chars"]),
            min_chars=int(value["min_chars"]),
        ).validate()

    def validate(self) -> "ChunkPolicy":
        if self.max_chars <= 0 or self.min_chars <= 0 or self.min_chars > self.max_chars:
            raise ValueError(f"invalid chunk policy: {self}")
        if self.overlap_chars >= self.max_chars:
            raise ValueError(f"invalid chunk overlap: {self}")
        return self

    def key(self) -> str:
        return f"{self.max_chars}:{self.overlap_chars}:{self.min_chars}"

    def to_json(self) -> dict[str, int]:
        return {
            "max_chars": self.max_chars,
            "overlap_chars": self.overlap_chars,
            "min_chars": self.min_chars,
        }


def load_json(path: Path) -> dict[str, Any]:
    value = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise ValueError(f"{path}: expected JSON object")
    return value


def load_jsonl(path: Path) -> list[dict[str, Any]]:
    rows = []
    with path.open("r", encoding="utf-8") as handle:
        for line_number, line in enumerate(handle, start=1):
            if not line.strip():
                continue
            value = json.loads(line)
            if not isinstance(value, dict):
                raise ValueError(f"{path}:{line_number}: expected JSON object")
            rows.append(value)
    return rows


def resolve(path: str | Path) -> Path:
    path = Path(path)
    if path.is_absolute():
        return path
    return REPO_ROOT / path


def tokenize(text: str) -> list[str]:
    return [match.group(0).lower() for match in TOKEN_RE.finditer(text)]


def split_text(text: str, policy: ChunkPolicy) -> list[str]:
    chunks: list[str] = []
    current = ""
    for paragraph in (part.strip() for part in text.split("\n\n")):
        if not paragraph:
            continue
        if len(paragraph) > policy.max_chars:
            flush_chunk(chunks, current, policy)
            current = ""
            split_long_text(paragraph, policy, chunks)
            continue
        if not current:
            current = paragraph
            continue
        combined = f"{current}\n\n{paragraph}"
        if len(combined) <= policy.max_chars:
            current = combined
        else:
            flush_chunk(chunks, current, policy)
            current = paragraph
    flush_chunk(chunks, current, policy)
    return chunks


def split_long_text(text: str, policy: ChunkPolicy, chunks: list[str]) -> None:
    start = 0
    while start < len(text):
        end = min(start + policy.max_chars, len(text))
        flush_chunk(chunks, text[start:end], policy)
        if end == len(text):
            break
        next_start = end - policy.overlap_chars
        start = end if next_start <= start else next_start


def flush_chunk(chunks: list[str], chunk: str, policy: ChunkPolicy) -> None:
    if len(chunk.strip()) >= policy.min_chars:
        chunks.append(chunk)


def build_chunks(documents: list[dict[str, Any]], policy: ChunkPolicy) -> list[dict[str, Any]]:
    chunks = []
    for document in documents:
        doc_id = str(document["doc_id"])
        text = str(document.get("text") or document.get("payload") or "")
        for index, chunk_text in enumerate(split_text(text, policy), start=1):
            chunks.append(
                {
                    "chunk_id": f"{doc_id}#bench-{index:04}",
                    "doc_id": doc_id,
                    "title": str(document.get("title", "")),
                    "text": chunk_text,
                }
            )
    return chunks


def idf(chunks: list[dict[str, Any]]) -> dict[str, float]:
    document_count = len(chunks)
    frequencies: Counter[str] = Counter()
    for chunk in chunks:
        frequencies.update(set(tokenize(chunk["text"])))
    return {
        term: math.log(1 + ((document_count - count + 0.5) / (count + 0.5)))
        for term, count in frequencies.items()
    }


def rank_chunks(query: str, chunks: list[dict[str, Any]], weights: dict[str, float]) -> list[dict[str, Any]]:
    query_terms = Counter(tokenize(query))
    scored = []
    for chunk in chunks:
        text = f"{chunk['title']} {chunk['text']}"
        term_counts = Counter(tokenize(text))
        score = 0.0
        for term, query_count in query_terms.items():
            if term_counts[term]:
                score += query_count * weights.get(term, 0.0) * term_counts[term]
        if score > 0.0:
            scored.append({"score": score, **chunk})
    scored.sort(key=lambda row: (-row["score"], row["doc_id"], row["chunk_id"]))
    return scored


def q16(value: float) -> int:
    return max(0, min(65535, int(round(value * 65535))))


def evaluate_policy(domain: dict[str, Any], policy: ChunkPolicy) -> dict[str, Any]:
    documents = load_jsonl(resolve(domain["source_root"]) / "documents.jsonl")
    queries = load_jsonl(resolve(domain["queries"]))
    ground_truth = {row["query_id"]: set(row["relevant_doc_ids"]) for row in load_jsonl(resolve(domain["ground_truth"]))}
    chunks = build_chunks(documents, policy)
    weights = idf(chunks)
    top_k = int(domain.get("top_k", 5))
    recall_total = 0.0
    mrr_total = 0.0
    query_reports = []
    for query in queries:
        query_id = query["query_id"]
        relevant = ground_truth.get(query_id, set())
        ranked = rank_chunks(str(query.get("query") or query.get("text") or ""), chunks, weights)
        top_docs = [row["doc_id"] for row in ranked[:top_k]]
        hits = [doc_id for doc_id in top_docs if doc_id in relevant]
        recall = 0.0 if not relevant else len(set(hits)) / len(relevant)
        reciprocal_rank = 0.0
        for rank, doc_id in enumerate(top_docs, start=1):
            if doc_id in relevant:
                reciprocal_rank = 1.0 / rank
                break
        recall_total += recall
        mrr_total += reciprocal_rank
        query_reports.append(
            {
                "query_id": query_id,
                "recall_at_k_q16": q16(recall),
                "reciprocal_rank_q16": q16(reciprocal_rank),
                "top_doc_ids": top_docs,
            }
        )
    query_count = len(queries)
    return {
        "policy": policy.to_json(),
        "chunk_count": len(chunks),
        "avg_chunk_chars": round(sum(len(chunk["text"]) for chunk in chunks) / max(len(chunks), 1), 2),
        "recall_at_k_q16": q16(recall_total / max(query_count, 1)),
        "mrr_q16": q16(mrr_total / max(query_count, 1)),
        "queries": query_reports,
    }


def select_best(rows: list[dict[str, Any]]) -> dict[str, Any]:
    return sorted(
        rows,
        key=lambda row: (
            -int(row["recall_at_k_q16"]),
            -int(row["mrr_q16"]),
            int(row["chunk_count"]),
            int(row["policy"]["max_chars"]),
        ),
    )[0]


def evaluate_domain(domain: dict[str, Any]) -> dict[str, Any]:
    candidates = [ChunkPolicy.from_json(candidate) for candidate in domain["candidates"]]
    selected = ChunkPolicy.from_json(domain["selected_policy"])
    policy_rows = [evaluate_policy(domain, policy) for policy in candidates]
    best = select_best(policy_rows)
    selected_row = next((row for row in policy_rows if ChunkPolicy.from_json(row["policy"]) == selected), None)
    failures = []
    if selected_row is None:
        failures.append("selected policy is not in candidates")
        selected_row = evaluate_policy(domain, selected)
    if selected.to_json() != best["policy"]:
        failures.append(
            f"selected policy {selected.to_json()} does not match recommended {best['policy']}"
        )
    for field, threshold in [
        ("recall_at_k_q16", int(domain["min_recall_at_k_q16"])),
        ("mrr_q16", int(domain["min_mrr_q16"])),
    ]:
        if int(selected_row[field]) < threshold:
            failures.append(f"selected {field} below threshold: {selected_row[field]} < {threshold}")
    return {
        "domain": domain["domain"],
        "top_k": int(domain.get("top_k", 5)),
        "status": "passed" if not failures else "failed",
        "failures": failures,
        "selected_policy": selected.to_json(),
        "recommended_policy": best["policy"],
        "selected_metrics": {
            "recall_at_k_q16": selected_row["recall_at_k_q16"],
            "mrr_q16": selected_row["mrr_q16"],
            "chunk_count": selected_row["chunk_count"],
            "avg_chunk_chars": selected_row["avg_chunk_chars"],
        },
        "candidate_results": policy_rows,
    }


def build_report(config: dict[str, Any]) -> dict[str, Any]:
    domains = config.get("domains")
    if not isinstance(domains, list) or not domains:
        raise ValueError("config.domains must be a non-empty list")
    domain_reports = [evaluate_domain(domain) for domain in domains]
    failures = [failure for row in domain_reports for failure in row["failures"]]
    return {
        "schema_version": "cortexdb.chunking_quality.report.v1",
        "status": "passed" if not failures else "failed",
        "domain_count": len(domain_reports),
        "failures": failures,
        "domains": domain_reports,
    }


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--settings", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args(argv)
    report = build_report(load_json(resolve(args.settings)))
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({"status": report["status"], "domain_count": report["domain_count"], "report": str(args.report)}, indent=2))
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
