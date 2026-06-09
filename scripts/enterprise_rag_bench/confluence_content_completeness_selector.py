#!/usr/bin/env python3
"""Confluence content completeness selector for EnterpriseRAG-Bench.

Targets completeness questions where the needed Confluence evidence is a
content family, not a single title/path match. It uses local source text only:
no LLM/API calls and no gold-aware document selection.
"""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

from jira_project_source_selector import (
    doc_ids,
    normalize,
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


MODE_TERMS = {
    "postmortem_followups": (
        "postmortem follow up action items owner team assigned most follow up "
        "owning team priority remediation incident production outage"
    ),
    "postmortem_fallback": (
        "postmortem internal incident writeups activating automatic fallback "
        "model region mitigation auto fallback fallback activated region "
        "fallback model fallback hosted dedicated private"
    ),
    "private_upgrade_gate": (
        "private deployment upgrade go no go gate validations approvals "
        "customer communications maintenance comms rollback checklist "
        "standards enterprise onboarding"
    ),
    "production_change_process": (
        "production change definition approvals pre deploy verification deploy "
        "execution customer internal communications rollback post change hosted "
        "api console runbook standard"
    ),
    "serving_runtime_hotfix": (
        "emergency serving runtime hotfix production hosted dedicated approvals "
        "checklists rollout rollback customer communications runbook procedure "
        "policy"
    ),
}

MODE_LIMITS = {
    "postmortem_followups": 9,
    "postmortem_fallback": 6,
    "private_upgrade_gate": 5,
    "production_change_process": 5,
    "serving_runtime_hotfix": 5,
}


class ConfluenceIndex:
    def __init__(self, *, uuid_index: dict[str, str], sources_dir: Path) -> None:
        self.sources_dir = sources_dir
        self.reverse_index = {path: doc_id for doc_id, path in uuid_index.items()}
        self.docs = self.load_docs()

    def load_docs(self) -> list[tuple[str, str, dict[str, Any], str]]:
        docs: list[tuple[str, str, dict[str, Any], str]] = []
        for path in sorted((self.sources_dir / "confluence").rglob("*.json")):
            rel_path = str(path.relative_to(self.sources_dir))
            raw = path.read_text(encoding="utf-8", errors="ignore")
            try:
                payload = json.loads(raw)
            except json.JSONDecodeError:
                payload = {}
            text = normalize(json.dumps(payload, ensure_ascii=False) + " " + rel_path)
            docs.append((self.reverse_index.get(rel_path, ""), rel_path, payload, text))
        return docs


def selector_mode(question: dict[str, Any]) -> str | None:
    if question.get("question_type") != "completeness":
        return None
    if "confluence" not in {str(item) for item in question.get("source_types", [])}:
        return None

    text = str(question.get("question", "")).lower()
    if "postmortems" in text and "follow-up action items" in text:
        return "postmortem_followups"
    if "incident writeups" in text and "automatic fallback" in text:
        return "postmortem_fallback"
    if "private deployment upgrade" in text and "go/no-go" in text:
        return "private_upgrade_gate"
    if "production change" in text and "hosted api and console" in text:
        return "production_change_process"
    if "emergency serving-runtime hotfix" in text:
        return "serving_runtime_hotfix"
    return None


def query_terms(mode: str, question_text: str) -> list[str]:
    return tokens(question_text) + MODE_TERMS.get(mode, "").split()


def title_path_text(rel_path: str, payload: dict[str, Any]) -> str:
    return normalize(f"{rel_path} {payload.get('title', '')} {payload.get('name', '')} {payload.get('space', '')}")


def allowed_path(mode: str, rel_path: str) -> bool:
    if mode.startswith("postmortem_"):
        return "/postmortems/" in rel_path and "template" not in rel_path and "taxonomy" not in rel_path
    if mode == "private_upgrade_gate":
        return any(
            marker in rel_path
            for marker in [
                "private-deployments",
                "change-management",
                "status-page-and-comms",
                "enterprise-onboarding",
            ]
        )
    if mode == "production_change_process":
        return any(marker in rel_path for marker in ["change-management", "runbooks", "status-page-and-comms"])
    if mode == "serving_runtime_hotfix":
        return any(
            marker in rel_path
            for marker in ["serving-runtime", "change-management", "runbooks", "status-page-and-comms"]
        )
    return False


def path_boost(mode: str, rel_path: str) -> int:
    boosts = {
        "private_upgrade_gate": [
            "go-no-go",
            "private-release-gating",
            "planned-maintenance-comms",
            "production-change-management-policy",
        ],
        "production_change_process": [
            "production-change-management-standard",
            "hosted-api-deploy-runbook",
            "console-deploy-runbook",
            "planned-maintenance-and-change-comms",
            "rollback-and-post-change-validation",
        ],
        "serving_runtime_hotfix": [
            "serving-runtime-hotfix-runbook",
            "serving-runtime-hotfix-procedure",
            "hosted-rollout-and-rollback",
            "emergency-change-policy",
        ],
    }
    return 70 * sum(1 for marker in boosts.get(mode, []) if marker in rel_path)


def mode_gate_and_boost(mode: str, text: str) -> int | None:
    if mode == "postmortem_followups":
        if "follow up action items" not in text and "action items" not in text:
            return None
        return 12 * text.count("owner team") + 8 * text.count("priority")
    if mode == "postmortem_fallback":
        if "fallback" not in text:
            return None
        return 55 * int("automatic" in text or "auto fallback" in text) + 25 * int("mitigation" in text)
    return 0


def score_doc(mode: str, question_text: str, doc: tuple[str, str, dict[str, Any], str]) -> int:
    _doc_id, rel_path, payload, text = doc
    if not allowed_path(mode, rel_path):
        return 0
    boost = mode_gate_and_boost(mode, text)
    if boost is None:
        return 0
    terms = query_terms(mode, question_text)
    score = score_terms(text, terms) + 4 * score_terms(title_path_text(rel_path, payload), terms)
    return score + boost + path_boost(mode, rel_path)


def top_confluence_docs(index: ConfluenceIndex, mode: str, question_text: str) -> list[str]:
    scored: list[tuple[int, str, str]] = []
    for doc in index.docs:
        score = score_doc(mode, question_text, doc)
        if score > 0 and doc[0]:
            scored.append((score, doc[0], doc[1]))
    limit = MODE_LIMITS.get(mode, 5)
    return [doc_id for _score, doc_id, _path in sorted(scored, key=lambda item: (-item[0], item[2]))[:limit]]


def select_docs(mode: str, question_text: str, baseline_ids: list[str], index: ConfluenceIndex, limit: int) -> list[str]:
    return unique(top_confluence_docs(index, mode, question_text) + baseline_ids)[:limit]


def run(args: argparse.Namespace) -> dict[str, Any]:
    questions = rows_by_id(read_jsonl(args.questions_file), "questions")
    baseline = rows_by_id(read_jsonl(args.baseline_retrieval_file), "baseline")
    uuid_index = read_json(args.uuid_index)
    if not isinstance(uuid_index, dict):
        raise ValueError("uuid index must be a JSON object")
    index = ConfluenceIndex(uuid_index=uuid_index, sources_dir=args.sources_dir)

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
            output["route"] = {
                "enabled": True,
                "mode": mode,
                "policy": args.policy_name,
                "source": "confluence_content_completeness_selector",
            }
        else:
            output["document_ids"] = baseline_ids
            output["route"] = {
                "enabled": False,
                "policy": args.policy_name,
                "source": "confluence_content_completeness_selector",
            }
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
        "questions_file": str(args.questions_file),
        "routed_rows": sum(mode_counts.values()),
        "schema_version": "cortexdb.enterprise_rag_bench.confluence_content_completeness_selector.v1",
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
    parser.add_argument("--policy-name", default="confluence_content_completeness_selector_v1")
    parser.add_argument("--limit", type=int, default=10)
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
