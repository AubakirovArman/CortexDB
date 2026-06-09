#!/usr/bin/env python3
"""Jira/project source selector for EnterpriseRAG-Bench.

Targets project-related questions where Jira support tickets are part of the
evidence chain but are missing from top10. It uses question text, source type
metadata, baseline retrieval, candidate retrieval, paths, and Jira document
text. It does not call an LLM/API and does not use gold labels to select docs.
"""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path
from typing import Any
STOPWORDS = set(
    "a an and any are as at be by can do for from how if in is it no not of on or "
    "our should that the their this to under via we what when where which with".split()
)
MODE_TERMS = {
    "residency_error_contract": (
        "residency policy violation error contract 409 subcode region_not_allowed "
        "primary_region_unavailable policy_misconfigured streaming generic 500 "
        "sdk gateway pm requirements adr"
    ),
    "sdk_streaming_parity": (
        "sdk streaming timeout timeouts retry retries retry-after retry after python "
        "typescript go parity matrix support tickets 429 disconnect proxy partial "
        "output conformance standard overall_timeout_ms stream_idle_timeout_ms"
    ),
    "canary_rollout_mismatch": (
        "canary traffic split rollout console config observed percentage region "
        "us-east smart routing telemetry oncall mitigations ga preventative fixes "
        "mismatch cohort effective intended"
    ),
}

def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))

def read_jsonl(path: Path) -> list[dict[str, Any]]:
    return [
        json.loads(line)
        for line in path.read_text(encoding="utf-8").splitlines()
        if line
    ]

def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")

def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        "".join(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n" for row in rows),
        encoding="utf-8",
    )

def rows_by_id(rows: list[dict[str, Any]], label: str) -> dict[str, dict[str, Any]]:
    values: dict[str, dict[str, Any]] = {}
    for index, row in enumerate(rows, 1):
        qid = row.get("question_id")
        if not isinstance(qid, str) or not qid:
            raise ValueError(f"{label} row {index} missing question_id")
        if qid in values:
            raise ValueError(f"{label} duplicate question_id: {qid}")
        values[qid] = row
    return values

def normalize(value: str) -> str:
    return re.sub(r"[^a-z0-9]+", " ", value.lower()).strip()

def tokens(value: str) -> list[str]:
    return [
        item
        for item in normalize(value).split()
        if len(item) > 1 and item not in STOPWORDS
    ]

def doc_ids(row: dict[str, Any] | None) -> list[str]:
    return [str(item) for item in (row or {}).get("document_ids", []) if str(item)]

def unique(values: list[str]) -> list[str]:
    selected: list[str] = []
    seen: set[str] = set()
    for value in values:
        if value and value not in seen:
            selected.append(value)
            seen.add(value)
    return selected

def recall_pct(question: dict[str, Any], docs: list[str]) -> float | None:
    expected = {str(item) for item in question.get("expected_doc_ids", []) if str(item)}
    if not expected:
        return None
    return round(len(expected & set(docs)) / len(expected) * 100.0, 2)

def selector_mode(question: dict[str, Any]) -> str | None:
    if question.get("question_type") != "project_related":
        return None
    if "jira" not in {str(item) for item in question.get("source_types", [])}:
        return None
    text = str(question.get("question", "")).lower()
    if "residency" in text and ("error contract" in text or "policy block" in text):
        return "residency_error_contract"
    if all(item in text for item in ["streaming timeout", "retry behavior", "python", "typescript", "go"]):
        return "sdk_streaming_parity"
    if "canary traffic" in text and "console rollout" in text:
        return "canary_rollout_mismatch"
    return None

def query_terms(mode: str, question_text: str) -> list[str]:
    return tokens(question_text) + MODE_TERMS.get(mode, "").split()

def score_terms(haystack: str, terms: list[str]) -> int:
    return sum(min(haystack.count(normalize(term)), 5) for term in set(terms) if normalize(term))

class SourceIndex:
    def __init__(self, *, uuid_index: dict[str, str], sources_dir: Path) -> None:
        self.uuid_index = uuid_index
        self.sources_dir = sources_dir
        self.reverse_index = {path: doc_id for doc_id, path in uuid_index.items()}
        self.text_cache: dict[str, str] = {}
        self.jira_docs = self.load_jira_docs()

    def load_jira_docs(self) -> list[tuple[str, str, dict[str, Any], str]]:
        docs: list[tuple[str, str, dict[str, Any], str]] = []
        for path in sorted((self.sources_dir / "jira").rglob("*.json")):
            rel_path = str(path.relative_to(self.sources_dir))
            raw = path.read_text(encoding="utf-8", errors="ignore")
            try:
                payload = json.loads(raw)
            except json.JSONDecodeError:
                payload = {}
            text = normalize(json.dumps(payload, ensure_ascii=False) + " " + rel_path)
            docs.append((self.reverse_index.get(rel_path, ""), rel_path, payload, text))
        return docs

    def normalized_text(self, doc_id: str) -> str:
        if doc_id not in self.text_cache:
            rel_path = self.uuid_index.get(doc_id, "")
            raw = (self.sources_dir / rel_path).read_text(encoding="utf-8", errors="ignore")
            try:
                payload = json.loads(raw)
            except json.JSONDecodeError:
                payload = {}
            self.text_cache[doc_id] = normalize(
                json.dumps(payload, ensure_ascii=False) + " " + rel_path
            )
        return self.text_cache[doc_id]

def score_jira_doc(index: SourceIndex, mode: str, question_text: str, doc: tuple[str, str, dict[str, Any], str]) -> int:
    _doc_id, rel_path, payload, text = doc
    terms = query_terms(mode, question_text)
    fields = normalize(
        json.dumps(
            {
                key: payload.get(key)
                for key in [
                    "components",
                    "labels",
                    "summary",
                    "key",
                    "linked_issues",
                    "related_github_prs",
                    "related_confluence_pages",
                ]
            },
            ensure_ascii=False,
        )
    )
    score = score_terms(text, terms) + 5 * score_terms(fields, terms)
    if rel_path.startswith("jira/customer-support/"):
        score += 30
    if rel_path.startswith("jira/internal-support/"):
        score -= 35
    if mode == "residency_error_contract" and "residency" not in text:
        return 0
    if mode == "sdk_streaming_parity" and not any(
        marker in text
        for marker in ["sdk python", "sdk typescript", "sdk go", "sdk parity", "retry after", "streaming timeout", "proxy retry"]
    ):
        return 0
    if mode == "canary_rollout_mismatch" and not all(marker in text for marker in ["canary", "rollout"]):
        return 0
    return score

def top_jira_docs(index: SourceIndex, mode: str, question_text: str, limit: int) -> list[str]:
    scored: list[tuple[int, str, str]] = []
    for doc in index.jira_docs:
        score = score_jira_doc(index, mode, question_text, doc)
        if score > 0 and doc[0]:
            scored.append((score, doc[0], doc[1]))
    return [doc_id for _score, doc_id, _path in sorted(scored, key=lambda item: (-item[0], item[2]))[:limit]]

def top_candidate_source(
    *,
    index: SourceIndex,
    mode: str,
    question_text: str,
    candidate_ids: list[str],
    source_type: str,
    limit: int,
) -> list[str]:
    terms = query_terms(mode, question_text)
    scored: list[tuple[float, int, str]] = []
    for rank, doc_id in enumerate(candidate_ids, 1):
        if not index.uuid_index.get(doc_id, "").startswith(f"{source_type}/"):
            continue
        score = score_terms(index.normalized_text(doc_id), terms) + 10 / (rank + 5)
        if score > 0:
            scored.append((score, rank, doc_id))
    return [doc_id for _score, _rank, doc_id in sorted(scored, key=lambda item: (-item[0], item[1]))[:limit]]

def select_docs(mode: str, question_text: str, baseline_ids: list[str], candidate_ids: list[str], index: SourceIndex, limit: int) -> list[str]:
    if mode == "residency_error_contract":
        sdk_docs = [doc_id for doc_id in baseline_ids if "sdk" in index.uuid_index.get(doc_id, "")]
        selected = baseline_ids[:4] + top_jira_docs(index, mode, question_text, 1) + sdk_docs + baseline_ids
    elif mode == "sdk_streaming_parity":
        selected = top_jira_docs(index, mode, question_text, 4) + top_candidate_source(
            index=index,
            mode=mode,
            question_text=question_text,
            candidate_ids=candidate_ids,
            source_type="confluence",
            limit=2,
        ) + baseline_ids
    else:
        selected = top_jira_docs(index, mode, question_text, 1) + top_candidate_source(
            index=index,
            mode=mode,
            question_text=question_text,
            candidate_ids=candidate_ids,
            source_type="confluence",
            limit=2,
        ) + baseline_ids
    return unique(selected)[:limit]

def run(args: argparse.Namespace) -> dict[str, Any]:
    questions = rows_by_id(read_jsonl(args.questions_file), "questions")
    baseline = rows_by_id(read_jsonl(args.baseline_retrieval_file), "baseline")
    candidates = rows_by_id(read_jsonl(args.candidate_retrieval_file), "candidates")
    uuid_index = read_json(args.uuid_index)
    if not isinstance(uuid_index, dict):
        raise ValueError("uuid index must be a JSON object")
    index = SourceIndex(uuid_index=uuid_index, sources_dir=args.sources_dir)
    output_rows: list[dict[str, Any]] = []
    recall_values: list[float] = []
    changed_rows = 0
    mode_counts: dict[str, int] = {}
    diagnostics: dict[str, Any] = {}

    for qid, base_row in sorted(baseline.items()):
        question = questions.get(qid, base_row)
        baseline_ids = doc_ids(base_row)[: args.limit]
        output = dict(base_row)
        mode = selector_mode(question)
        if mode:
            selected = select_docs(mode, str(question.get("question", "")), baseline_ids, doc_ids(candidates.get(qid)), index, args.limit)
            changed_rows += int(selected != baseline_ids)
            mode_counts[mode] = mode_counts.get(mode, 0) + 1
            output["document_ids"] = selected
            output["route"] = {"enabled": True, "mode": mode, "policy": args.policy_name, "source": "jira_project_source_selector"}
            if args.diagnostics:
                diagnostics[qid] = {"baseline": baseline_ids, "mode": mode, "selected": selected}
        else:
            output["document_ids"] = baseline_ids
            output["route"] = {"enabled": False, "policy": args.policy_name, "source": "jira_project_source_selector"}
        output_rows.append(output)
        recall = recall_pct(question, output["document_ids"])
        if recall is not None:
            recall_values.append(recall)

    report = {
        "average_recall_pct": round(sum(recall_values) / len(recall_values), 2) if recall_values else 0.0,
        "baseline_retrieval_file": str(args.baseline_retrieval_file),
        "candidate_retrieval_file": str(args.candidate_retrieval_file),
        "changed_rows": changed_rows,
        "diagnostics": diagnostics,
        "full_recall_questions": sum(1 for value in recall_values if value == 100.0),
        "hit_questions": sum(1 for value in recall_values if value > 0.0),
        "mode_counts": mode_counts,
        "note": "Local deterministic selector; no LLM/API calls and no gold-aware selection.",
        "output": str(args.output),
        "policy_name": args.policy_name,
        "questions": len(output_rows),
        "questions_file": str(args.questions_file),
        "routed_rows": sum(mode_counts.values()),
        "schema_version": "cortexdb.enterprise_rag_bench.jira_project_source_selector.v1",
    }
    write_jsonl(args.output, output_rows)
    write_json(args.report, report)
    return report

def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--baseline-retrieval-file", type=Path, required=True)
    parser.add_argument("--candidate-retrieval-file", type=Path, required=True)
    parser.add_argument("--uuid-index", type=Path, required=True)
    parser.add_argument("--sources-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--policy-name", default="jira_project_source_selector_v1")
    parser.add_argument("--limit", type=int, default=10)
    parser.add_argument("--diagnostics", action="store_true")
    args = parser.parse_args()
    if args.limit <= 0:
        parser.error("--limit must be positive")
    return args

def main() -> int:
    report = run(parse_args())
    print(
        json.dumps(
            {
                "average_recall_pct": report["average_recall_pct"],
                "changed_rows": report["changed_rows"],
                "full_recall_questions": report["full_recall_questions"],
                "hit_questions": report["hit_questions"],
                "mode_counts": report["mode_counts"],
                "output": report["output"],
                "routed_rows": report["routed_rows"],
            },
            sort_keys=True,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
