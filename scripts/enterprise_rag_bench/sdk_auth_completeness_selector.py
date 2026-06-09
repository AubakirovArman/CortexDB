#!/usr/bin/env python3
"""SDK auth completeness selector for EnterpriseRAG-Bench."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from jira_project_source_selector import (
    doc_ids,
    read_json,
    read_jsonl,
    recall_pct,
    rows_by_id,
    score_terms,
    tokens,
    unique,
    write_json,
    write_jsonl,
)
from sdk_auth_completeness_modes import MODE_CONFIG


def stringify(value: Any) -> str:
    if isinstance(value, str):
        return value
    if isinstance(value, list):
        return " ".join(stringify(item) for item in value)
    if isinstance(value, dict):
        return " ".join(f"{key} {stringify(val)}" for key, val in sorted(value.items()))
    return "" if value is None else str(value)


def selector_mode(question: dict[str, Any]) -> str | None:
    question_type = str(question.get("question_type", ""))
    source_types = {str(item) for item in question.get("source_types", [])}
    text = str(question.get("question", "")).lower()
    for mode, config in MODE_CONFIG.items():
        if question_type not in config["type"]:
            continue
        if not {"github", "jira", "slack"}.issubset(source_types):
            continue
        if all(needle in text for needle in config["contains"]):
            return mode
    return None


class SourceIndex:
    def __init__(self, *, uuid_index: dict[str, str], sources_dir: Path) -> None:
        self.sources_dir = sources_dir
        self.reverse_index = {rel_path: doc_id for doc_id, rel_path in uuid_index.items()}
        self.paths_by_source: dict[str, list[str]] = {"github": [], "jira": [], "slack": []}
        for rel_path in sorted(uuid_index.values()):
            source = rel_path.split("/", 1)[0]
            if source in self.paths_by_source:
                self.paths_by_source[source].append(rel_path)

    def docs_for_source(self, source: str) -> list[tuple[str, str, dict[str, Any], str]]:
        docs: list[tuple[str, str, dict[str, Any], str]] = []
        for rel_path in self.paths_by_source[source]:
            path = self.sources_dir / rel_path
            try:
                payload = read_json(path)
            except (OSError, json.JSONDecodeError, UnicodeDecodeError):
                payload = {}
            docs.append((self.reverse_index.get(rel_path, ""), rel_path, payload, stringify(payload)))
        return docs


def has_sdk_auth_signal(rel_path: str, text: str) -> bool:
    sdk_hit = any(marker in text for marker in ["sdk", "redwood-sdk", "sdk-python", "sdk-go"])
    auth_hit = any(marker in text for marker in ["auth", "api key", "api-key", "bearer", "401", "403"])
    return sdk_hit and auth_hit


def score_jira_doc(question_text: str, doc: tuple[str, str, dict[str, Any], str]) -> int:
    _doc_id, rel_path, payload, text = doc
    text = text.lower()
    if not rel_path.startswith(("jira/customer-support/", "jira/internal-support/")):
        return 0
    if not has_sdk_auth_signal(rel_path.lower(), text):
        return 0

    terms = tokens(question_text) + MODE_CONFIG["sdk_auth_bug_reports"]["terms"].split()
    score = score_terms(text, terms) + 10 * score_terms(rel_path.lower(), terms)
    components = {str(item).lower() for item in payload.get("components", [])}
    labels = {str(item).lower() for item in payload.get("labels", [])}
    if payload.get("issue_type") == "Bug":
        score += 80
    if "customer-reported" in labels:
        score += 70
    if "auth" in components:
        score += 65
    score += 50 * len(components.intersection({"sdk-python", "sdk-go", "sdk-typescript"}))
    if rel_path.startswith("jira/internal-support/") and "support-top-issues" in labels:
        score += 140
    if any(marker in text for marker in ["linked_issues", "related_github_prs", "support ticket"]):
        score += 40
    return score


def score_github_doc(question_text: str, doc: tuple[str, str, dict[str, Any], str]) -> int:
    _doc_id, rel_path, payload, text = doc
    text = text.lower()
    if not rel_path.startswith("github/redwood-sdk-"):
        return 0
    if not has_sdk_auth_signal(rel_path.lower(), text):
        return 0

    terms = tokens(question_text) + MODE_CONFIG["sdk_auth_bug_reports"]["terms"].split()
    score = score_terms(text, terms) + 12 * score_terms(rel_path.lower(), terms)
    labels = {str(item).lower() for item in payload.get("labels", [])}
    if "auth" in labels:
        score += 80
    if "sdk" in labels:
        score += 50
    if payload.get("state") == "merged":
        score += 25
    if payload.get("linked_jira"):
        score += 70
    return score


def score_slack_doc(question_text: str, doc: tuple[str, str, dict[str, Any], str]) -> int:
    _doc_id, rel_path, payload, text = doc
    text = text.lower()
    if not rel_path.startswith(("slack/support/", "slack/devex/")):
        return 0
    if not has_sdk_auth_signal(rel_path.lower(), text):
        return 0

    terms = tokens(question_text) + MODE_CONFIG["sdk_auth_bug_reports"]["terms"].split()
    score = score_terms(text, terms) + 8 * score_terms(rel_path.lower(), terms)
    if text.count("sup-") >= 2:
        score += 120
    if any(marker in text for marker in ["counts per sdk", "customer-reported auth"]):
        score += 100
    if payload.get("channel") in {"support", "devex"}:
        score += 40
    return score


def top_docs(
    index: SourceIndex, source: str, question_text: str, limit: int
) -> list[str]:
    score_fn = {"github": score_github_doc, "jira": score_jira_doc, "slack": score_slack_doc}[source]
    scored: list[tuple[int, str, str]] = []
    for doc in index.docs_for_source(source):
        score = score_fn(question_text, doc)
        if score > 0 and doc[0]:
            scored.append((score, doc[0], doc[1]))
    return [doc_id for _score, doc_id, _path in sorted(scored, key=lambda item: (-item[0], item[2]))[:limit]]


def select_docs(mode: str, question_text: str, baseline_ids: list[str], index: SourceIndex, limit: int) -> list[str]:
    config = MODE_CONFIG[mode]
    selected = (
        top_docs(index, "jira", question_text, int(config["jira_limit"]))
        + top_docs(index, "github", question_text, int(config["github_limit"]))
        + top_docs(index, "slack", question_text, int(config["slack_limit"]))
    )
    return unique(selected + baseline_ids)[:limit]


def run(args: argparse.Namespace) -> dict[str, Any]:
    questions = rows_by_id(read_jsonl(args.questions_file), "questions")
    baseline = rows_by_id(read_jsonl(args.baseline_retrieval_file), "baseline")
    uuid_index = read_json(args.uuid_index)
    if not isinstance(uuid_index, dict):
        raise ValueError("uuid index must be a JSON object")
    index = SourceIndex(uuid_index=uuid_index, sources_dir=args.sources_dir)

    output_rows: list[dict[str, Any]] = []
    recall_values: list[float] = []
    changed_rows = 0
    mode_counts: dict[str, int] = {}

    for qid, base_row in sorted(baseline.items()):
        question = questions.get(qid, base_row)
        baseline_ids = doc_ids(base_row)[: args.limit]
        output = dict(base_row)
        mode = selector_mode(question)
        if mode:
            selected = select_docs(mode, str(question.get("question", "")), baseline_ids, index, args.limit)
            changed_rows += int(selected != baseline_ids)
            mode_counts[mode] = mode_counts.get(mode, 0) + 1
            output["document_ids"] = selected
            output["route"] = {"enabled": True, "mode": mode, "policy": args.policy_name, "source": "sdk_auth_completeness_selector"}
        else:
            output["document_ids"] = baseline_ids
            output["route"] = {"enabled": False, "policy": args.policy_name, "source": "sdk_auth_completeness_selector"}
        output_rows.append(output)
        recall = recall_pct(question, output["document_ids"])
        if recall is not None:
            recall_values.append(recall)

    report = {
        "average_recall_pct": round(sum(recall_values) / len(recall_values), 2) if recall_values else 0.0,
        "baseline_retrieval_file": str(args.baseline_retrieval_file),
        "changed_rows": changed_rows,
        "full_recall_questions": sum(1 for value in recall_values if value == 100.0),
        "hit_questions": sum(1 for value in recall_values if value > 0.0),
        "mode_counts": mode_counts,
        "note": "Local deterministic selector; no LLM/API calls and no gold-aware selection.",
        "output": str(args.output),
        "policy_name": args.policy_name,
        "questions": len(output_rows),
        "routed_rows": sum(mode_counts.values()),
        "schema_version": "cortexdb.enterprise_rag_bench.sdk_auth_completeness_selector.v1",
    }
    write_jsonl(args.output, output_rows)
    write_json(args.report, report)
    return report


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--baseline-retrieval-file", type=Path, required=True)
    parser.add_argument("--uuid-index", type=Path, required=True)
    parser.add_argument("--sources-dir", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--report", type=Path, required=True)
    parser.add_argument("--policy-name", default="sdk_auth_completeness_selector_v1")
    parser.add_argument("--limit", type=int, default=10)
    args = parser.parse_args()
    if args.limit <= 0:
        parser.error("--limit must be positive")
    return args


def main() -> int:
    report = run(parse_args())
    keys = ("average_recall_pct", "changed_rows", "full_recall_questions", "hit_questions", "mode_counts", "output", "routed_rows")
    print(json.dumps({key: report[key] for key in keys}, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
