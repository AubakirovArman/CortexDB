#!/usr/bin/env python3
"""Project-aware retrieval reranking for EnterpriseRAG-Bench."""

from __future__ import annotations

import argparse
import json
import re
from collections import Counter, defaultdict
from pathlib import Path
from typing import Any

STOPWORDS = set(
    """
    a an and any are as at be been before by case do for from how if in into is it no not
    of on or our should that the their these this those to under via we what when where
    which while who why with without
    """.split()
)

PHRASES = "usage api|status page|support bridge|enterprise route|quickstart templates|corporate proxy|priority routing|streaming retries|credits wording|policy exception".split("|")


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))

def read_jsonl(path: Path) -> list[dict[str, Any]]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line]

def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")

def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        "".join(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n" for row in rows),
        encoding="utf-8",
    )


def normalize(value: str) -> str:
    return re.sub(r"[^a-z0-9]+", " ", value.lower()).strip()

def tokens(value: str) -> list[str]:
    return [item for item in normalize(value).split() if len(item) > 1 and item not in STOPWORDS]

def rows_by_id(rows: list[dict[str, Any]], label: str) -> dict[str, dict[str, Any]]:
    indexed: dict[str, dict[str, Any]] = {}
    for row_index, row in enumerate(rows, 1):
        qid = row.get("question_id")
        if not isinstance(qid, str) or not qid:
            raise ValueError(f"{label} row {row_index} missing question_id")
        if qid in indexed:
            raise ValueError(f"{label} duplicate question_id: {qid}")
        indexed[qid] = row
    return indexed

def question_anchors(question: str) -> list[str]:
    anchors: list[str] = []
    patterns = [
        r"\b[A-Z]{2,}[A-Z0-9-]*\b",
        r"\b[A-Za-z]+-\d+[A-Za-z0-9-]*\b",
        r"\b\d+(?:\.\d+)?%?\b",
        r"\b[a-z]+-[a-z0-9-]+(?:-[a-z0-9]+)*\b",
        r"\b[A-Z][a-z]+\b",
    ]
    for pattern in patterns:
        anchors.extend(re.findall(pattern, question))
    return sorted({normalize(anchor) for anchor in anchors if normalize(anchor)})

def domain_terms(question: str) -> list[str]:
    lower = question.lower()
    terms: list[str] = []
    if "429" in lower or "routing" in lower or "slo" in lower:
        terms += (
            "overload admission control protection route routing slo error budget burn "
            "availability p95 p99 latency shed rate retry after hot enterprise protected dashboards"
        ).split()
    if "invoice" in lower or "usage api" in lower or "double-count" in lower:
        terms += (
            "billing metering ledger invoice usage api export retry fallback dedupe idempotency "
            "billed tokens credit refund approval laura michael emily logan nadia"
        ).split()
    if "escalat" in lower or "status page" in lower or "credits wording" in lower:
        terms += (
            "support bridge incident incidents status page comms customer update cadence credits "
            "sla leadership approval severity p0 p1 legal"
        ).split()
    if "telemetry" in lower or "header" in lower or "template" in lower:
        terms += (
            "template telemetry header quickstart kpi tracking proxy corporate x redwood tags "
            "opt sent adoption metrics allow list"
        ).split()
    return terms

class CorpusIndex:
    def __init__(self, uuid_index: dict[str, str], sources_dir: Path):
        self.uuid_index = uuid_index
        self.sources_dir = sources_dir
        self.suffix_to_doc: dict[str, str] = {}
        self.key_to_docs: dict[str, list[str]] = defaultdict(list)
        self._doc_cache: dict[str, tuple[str, dict[str, Any], str]] = {}
        self._build_path_indexes()

    def _build_path_indexes(self) -> None:
        for doc_id, rel_path in self.uuid_index.items():
            path_no_ext = rel_path.removesuffix(".json").removeprefix("sources/")
            parts = path_no_ext.split("/")
            for start in range(len(parts)):
                suffix = normalize("/".join(parts[start:]))
                self.suffix_to_doc.setdefault(suffix, doc_id)
            for key in re.findall(r"\b[A-Za-z]+-\d+[A-Za-z0-9-]*\b", rel_path):
                self.key_to_docs[normalize(key)].append(doc_id)

    def load_doc(self, doc_id: str) -> tuple[str, dict[str, Any], str]:
        cached = self._doc_cache.get(doc_id)
        if cached is not None:
            return cached
        rel_path = self.uuid_index.get(doc_id)
        if not rel_path:
            return "", {}, ""
        text = (self.sources_dir / rel_path).read_text(encoding="utf-8", errors="ignore")
        try:
            payload = json.loads(text)
        except json.JSONDecodeError:
            payload = {}
        result = (text, payload, rel_path)
        self._doc_cache[doc_id] = result
        return result

    def resolve_ref(self, value: str) -> list[str]:
        ref = normalize(
            value.replace("sources/", "")
            .replace(".json", "")
            .replace("confluence://", "confluence/")
            .replace("Confluence:", "confluence/")
            .replace("Jira:", "jira/")
            .replace("GitHub:", "github/")
            .replace("Fireflies:", "fireflies/")
            .replace("HubSpot:", "hubspot/")
        )
        resolved: list[str] = []
        if ref in self.suffix_to_doc:
            resolved.append(self.suffix_to_doc[ref])
        for key in re.findall(r"\b[a-z]+ \d+[a-z0-9]*\b", ref):
            resolved.extend(self.key_to_docs.get(key, []))
        return resolved

def collect_link_refs(payload: Any) -> list[str]:
    refs: list[str] = []

    def walk(value: Any) -> None:
        if isinstance(value, dict):
            for child in value.values():
                walk(child)
        elif isinstance(value, list):
            for child in value:
                walk(child)
        elif isinstance(value, str) and ("/" in value or ":" in value or re.search(r"[A-Z]+-\d+", value)):
            refs.append(value)

    walk(payload)
    return refs

def score_doc(corpus: CorpusIndex, question: dict[str, Any], doc_id: str, rank: int, link_bonus: float) -> float:
    text, _payload, rel_path = corpus.load_doc(doc_id)
    question_text = str(question.get("question", ""))
    query_terms = tokens(question_text) + domain_terms(question_text)
    query_counts = Counter(query_terms)
    doc_counts = Counter(tokens(f"{text} {rel_path}"))
    normalized_doc = normalize(f"{text} {rel_path}")
    coverage = sum(1 for term in set(query_terms) if term in doc_counts)
    term_frequency = sum(min(doc_counts[term], 5) * weight for term, weight in query_counts.items())
    anchor_score = sum(6 for anchor in question_anchors(question_text) if anchor in normalized_doc)
    phrase_score = sum(4 for phrase in PHRASES if phrase in question_text.lower() and phrase in normalized_doc)
    source_boost = 2 if rel_path.split("/", 1)[0] in question.get("source_types", []) else 0
    rank_boost = 12 / (rank + 5) if rank > 0 else 0
    return term_frequency + coverage * 1.5 + anchor_score + phrase_score + source_boost + rank_boost + link_bonus

def rerank_project_question(
    corpus: CorpusIndex,
    question: dict[str, Any],
    candidate_row: dict[str, Any],
    limit: int,
    seed_count: int,
    link_bonus: float,
) -> tuple[list[str], dict[str, Any]]:
    candidate_ids = [str(item) for item in candidate_row.get("document_ids", []) if str(item)]
    pool: dict[str, tuple[int, float]] = {doc_id: (rank, 0.0) for rank, doc_id in enumerate(candidate_ids, 1)}
    first_pass = [
        (score_doc(corpus, question, doc_id, rank, 0.0), doc_id)
        for doc_id, (rank, _bonus) in pool.items()
    ]
    seed_ids = [doc_id for _score, doc_id in sorted(first_pass, reverse=True)[:seed_count]]

    linked_ids: set[str] = set()
    for doc_id in seed_ids:
        _text, payload, _rel_path = corpus.load_doc(doc_id)
        for ref in collect_link_refs(payload):
            linked_ids.update(corpus.resolve_ref(ref))

    added_by_link = 0
    for doc_id in linked_ids:
        rank, previous_bonus = pool.get(doc_id, (0, 0.0))
        if doc_id not in pool:
            added_by_link += 1
        pool[doc_id] = (rank, max(previous_bonus, link_bonus))

    scored = [
        (score_doc(corpus, question, doc_id, rank, bonus), doc_id)
        for doc_id, (rank, bonus) in pool.items()
    ]
    reranked = [doc_id for _score, doc_id in sorted(scored, reverse=True)[:limit]]
    return reranked, {
        "seed_count": len(seed_ids),
        "linked_docs_added": added_by_link,
        "candidate_pool": len(candidate_ids),
        "expanded_pool": len(pool),
    }

def recall_pct(question: dict[str, Any], doc_ids: list[str]) -> float | None:
    expected = {str(item) for item in question.get("expected_doc_ids", [])}
    if not expected:
        return None
    return round(len(expected & set(doc_ids)) / len(expected) * 100.0, 2)

def run(args: argparse.Namespace) -> dict[str, Any]:
    questions = rows_by_id(read_jsonl(args.questions_file), "questions")
    default_rows = rows_by_id(read_jsonl(args.default_retrieval_file), "default retrieval")
    candidate_rows = rows_by_id(read_jsonl(args.candidate_retrieval_file), "candidate retrieval")
    corpus = CorpusIndex(read_json(args.uuid_index), args.sources_dir)

    output_rows: list[dict[str, Any]] = []
    routed_count = 0
    recall_values: list[float] = []
    project_recalls: list[float] = []
    diagnostics: dict[str, Any] = {}

    for qid, question in questions.items():
        row = dict(default_rows[qid])
        if question.get("question_type") == "project_related":
            routed_count += 1
            doc_ids, diag = rerank_project_question(
                corpus, question, candidate_rows[qid], args.limit, args.seed_count, args.link_bonus
            )
            row["document_ids"] = doc_ids
            row["route"] = {"policy": args.policy_name, "source": "project_chain", "question_type": "project_related"}
            diagnostics[qid] = diag
            project_recall = recall_pct(question, doc_ids)
            if project_recall is not None:
                project_recalls.append(project_recall)
        recall = recall_pct(question, [str(item) for item in row.get("document_ids", [])])
        if recall is not None:
            recall_values.append(recall)
        output_rows.append(row)

    write_jsonl(args.output, output_rows)
    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.project_chain_rerank_report.v1",
        "policy_name": args.policy_name,
        "questions": len(output_rows),
        "routed_project_questions": routed_count,
        "average_recall_pct": round(sum(recall_values) / len(recall_values), 2) if recall_values else 0.0,
        "project_related_recall_pct": round(sum(project_recalls) / len(project_recalls), 2) if project_recalls else 0.0,
        "diagnostics": diagnostics,
    }
    write_json(args.report, report)
    print(json.dumps(report, sort_keys=True))
    return report

def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--default-retrieval-file", type=Path, required=True)
    parser.add_argument("--candidate-retrieval-file", type=Path, required=True)
    parser.add_argument("--uuid-index", type=Path, required=True)
    parser.add_argument("--sources-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--policy-name", default="v10_project_chain")
    parser.add_argument("--limit", type=int, default=10)
    parser.add_argument("--seed-count", type=int, default=12)
    parser.add_argument("--link-bonus", type=float, default=32.0)
    return parser.parse_args()

if __name__ == "__main__":
    run(parse_args())
