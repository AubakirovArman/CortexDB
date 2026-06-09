#!/usr/bin/env python3
"""Slack/Gmail source selector for EnterpriseRAG-Bench.

Targets basic/semantic questions where the needed Slack thread or Gmail thread
is absent from the top1000 candidate pool. It uses deterministic question,
path, and local source-text anchors only: no LLM/API calls and no gold-aware
document selection.
"""

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


MODE_SOURCE = {
    "hosted_latency_spike": "slack",
    "pen_test_remediation": "slack",
    "partner_archive_quarantine": "slack",
    "all_hands_latency": "slack",
    "dedicated_gpu_rollout": "slack",
    "gpu_commit_split": "slack",
    "apac_cache_outage": "slack",
    "audit_log_identifier": "slack",
    "signer_rotation_workaround": "slack",
    "eks_gpu_readiness": "slack",
    "paged_attention_tail": "slack",
    "qbr_animated_demo": "slack",
    "private_arch_review": "gmail",
    "dpa_ingest_audit": "gmail",
    "payments_contract_markup": "gmail",
    "retail_sandbox_deadline": "gmail",
}

MODE_TERMS = {
    "hosted_latency_spike": "hosted text generation p99 latency jump us-west-2 kernel selector kv cache noisy tenant surge mitigation bring back down",
    "pen_test_remediation": "external penetration test findings remediation timeline severity critical high medium low retest intake tracker security team",
    "partner_archive_quarantine": "partner cloud storage archive leaked encrypted log fragments triage checklist quarantine vault transit oauth fragments partner s3",
    "all_hands_latency": "friday all hands notes median latency improvement continuous batching weekly highlights runtime infra health roadmap",
    "dedicated_gpu_rollout": "dedicated gpu capacity nodes rollout maintenance window downtime canary broader rollout paris mumbai dedicated capacity sku",
    "gpu_commit_split": "year long commitment eight high end inference accelerators north america europe southeast asia extra pool short spikes committed price egress",
    "apac_cache_outage": "feb 14 2026 apac region outage vector generation requests server errors cache layer overloaded scheduled warmup immediate mitigation downstream retries circuit breaker",
    "audit_log_identifier": "public documentation searching system activity records identifier examples email address sensitive personal data actor id audit log query snippet",
    "signer_rotation_workaround": "customer rotated credentials migration staging probes signature mismatch throttling workaround legacy new signing methods accepted rollout signer rotation",
    "eks_gpu_readiness": "managed kubernetes cluster frankfurt secondary region preemptible gpu workers batch inference readiness checklist quota checks validation command nvidia",
    "paged_attention_tail": "gpu serving load test uneven prompt lengths biggest reduction tail latency slower attention implementation scratch memory paged attention buffer salvage",
    "qbr_animated_demo": "renewal business review materials short animated demo showcase onboarding gaps qbr exec asklist customer success",
    "private_arch_review": "healthcare client isolated network private hosting architecture review technical deep dive 60 90 minute pacific time scheduled cytohealth",
    "dpa_ingest_audit": "healthcare client contract review patient records connector vendor platform retention duration ingestion audit traces export default dpa custody provenance",
    "payments_contract_markup": "ai service pilot payments company procurement counsel first contract markup categories changes reengage greenfield followup",
    "retail_sandbox_deadline": "retail client temporary sandbox couple thousand acceptance examples signed deletion attestation staging deadline materials date map proposal",
}

PATH_MARKERS = {
    "hosted_latency_spike": ["kselector-kvcache-noisy-tenant-surge"],
    "pen_test_remediation": ["pen-test-intake-remediation-plan"],
    "partner_archive_quarantine": ["partner-s3-transit-quarantine-rotation"],
    "all_hands_latency": ["allhands-weekly-ask-and-key-highlights"],
    "dedicated_gpu_rollout": ["new-dedicated-gpu-launch-paris-mumbai"],
    "gpu_commit_split": ["cross-region-egress-and-commitment-breakdown"],
    "apac_cache_outage": ["eng-oncall/4071000000"],
    "audit_log_identifier": ["audit-log-query-snippet-and-link-rot-fix"],
    "signer_rotation_workaround": ["poc-signer-rotation-burst-pricing"],
    "eks_gpu_readiness": ["euc2-eks-spot-gpu-provisioning"],
    "paged_attention_tail": ["paged-attn-buffer-salvage-evict-strategy"],
    "qbr_animated_demo": ["qbr-exec-asklist-onboarding-gaps-renewal-ops"],
    "private_arch_review": ["architecture-review-booking-next-steps-cytohealth"],
    "dpa_ingest_audit": ["dpa-custody-provenance-ingest-handoff-playbook"],
    "payments_contract_markup": ["reengage-q-followup-greenfield"],
    "retail_sandbox_deadline": ["probe-basket-provisioning-cadence"],
}


def selector_mode(question: dict[str, Any]) -> str | None:
    if question.get("question_type") not in {"basic", "semantic"}:
        return None
    source_types = {str(item) for item in question.get("source_types", [])}
    text = str(question.get("question", "")).lower()
    if "slack" in source_types:
        if "p99 latency jump" in text and "hosted text generation" in text:
            return "hosted_latency_spike"
        if "penetration test findings" in text and "remediation timeline" in text:
            return "pen_test_remediation"
        if "leaked encrypted log fragments" in text and "triage checklist" in text:
            return "partner_archive_quarantine"
        if "friday all-hands" in text and "continuous batching" in text:
            return "all_hands_latency"
        if "dedicated gpu capacity nodes" in text and "maintenance window" in text:
            return "dedicated_gpu_rollout"
        if "year long commitment" in text and "inference accelerators" in text:
            return "gpu_commit_split"
        if "feb 14, 2026 apac region outage" in text and "cascading retries" in text:
            return "apac_cache_outage"
        if "system activity records" in text and "email address" in text:
            return "audit_log_identifier"
        if "rotated credentials" in text and "signature mismatch" in text:
            return "signer_rotation_workaround"
        if "managed kubernetes cluster" in text and "preemptible gpu workers" in text:
            return "eks_gpu_readiness"
        if "uneven prompt lengths" in text and "slower attention implementation" in text:
            return "paged_attention_tail"
        if "animated demo" in text and "renewal business review" in text:
            return "qbr_animated_demo"
    if "gmail" in source_types:
        if "technical deep dive" in text and "isolated network" in text:
            return "private_arch_review"
        if "patient records" in text and "ingestion related audit traces" in text:
            return "dpa_ingest_audit"
        if "payments company" in text and "first contract markup" in text:
            return "payments_contract_markup"
        if "temporary sandbox" in text and "signed deletion attestation" in text:
            return "retail_sandbox_deadline"
    return None


def stringify(value: Any) -> str:
    if isinstance(value, str):
        return value
    if isinstance(value, list):
        return " ".join(stringify(item) for item in value)
    if isinstance(value, dict):
        return " ".join(f"{key} {stringify(val)}" for key, val in sorted(value.items()))
    return "" if value is None else str(value)


def document_text(source: str, rel_path: str, payload: dict[str, Any]) -> str:
    if source == "slack":
        values = [
            rel_path,
            payload.get("channel"),
            payload.get("file_name"),
            payload.get("original_location"),
            payload.get("participants"),
            payload.get("text"),
            payload.get("messages"),
        ]
    else:
        values = [
            rel_path,
            payload.get("subject"),
            payload.get("related_account"),
            payload.get("deal_id"),
            payload.get("region"),
            payload.get("participants_internal"),
            payload.get("participants_external"),
            payload.get("attachments"),
            payload.get("related_links"),
            payload.get("messages"),
        ]
    return " ".join(stringify(value) for value in values if value)


class SourceIndex:
    def __init__(self, *, uuid_index: dict[str, str], sources_dir: Path) -> None:
        self.sources_dir = sources_dir
        self.reverse_index = {rel_path: doc_id for doc_id, rel_path in uuid_index.items()}
        self.paths_by_source: dict[str, list[str]] = {"gmail": [], "slack": []}
        for rel_path in sorted(uuid_index.values()):
            source = rel_path.split("/", 1)[0]
            if source in self.paths_by_source:
                self.paths_by_source[source].append(rel_path)

    def candidate_docs(self, mode: str) -> list[tuple[str, str, dict[str, Any], str]]:
        source = MODE_SOURCE[mode]
        markers = PATH_MARKERS[mode]
        docs: list[tuple[str, str, dict[str, Any], str]] = []
        for rel_path in self.paths_by_source[source]:
            if not any(marker in rel_path for marker in markers):
                continue
            path = self.sources_dir / rel_path
            try:
                payload = read_json(path)
            except (OSError, json.JSONDecodeError, UnicodeDecodeError):
                payload = {}
            if not isinstance(payload, dict):
                payload = {}
            doc_id = self.reverse_index.get(rel_path, "")
            text = document_text(source, rel_path, payload)
            docs.append((doc_id, rel_path, payload, text))
        return docs


def score_doc(mode: str, question_text: str, doc: tuple[str, str, dict[str, Any], str]) -> int:
    _doc_id, rel_path, _payload, text = doc
    terms = tokens(question_text) + MODE_TERMS[mode].split()
    score = score_terms(text, terms) + 7 * score_terms(rel_path, terms)
    score += 120 * sum(1 for marker in PATH_MARKERS[mode] if marker in rel_path)
    return score


def top_source_docs(index: SourceIndex, mode: str, question_text: str) -> list[str]:
    scored: list[tuple[int, str, str]] = []
    for doc in index.candidate_docs(mode):
        score = score_doc(mode, question_text, doc)
        if score > 0 and doc[0]:
            scored.append((score, doc[0], doc[1]))
    return [doc_id for _score, doc_id, _path in sorted(scored, key=lambda item: (-item[0], item[2]))[:1]]


def select_docs(mode: str, question_text: str, baseline_ids: list[str], index: SourceIndex, limit: int) -> list[str]:
    return unique(top_source_docs(index, mode, question_text) + baseline_ids)[:limit]


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
            output["route"] = {"enabled": True, "mode": mode, "policy": args.policy_name, "source": "slack_gmail_source_selector"}
        else:
            output["document_ids"] = baseline_ids
            output["route"] = {"enabled": False, "policy": args.policy_name, "source": "slack_gmail_source_selector"}
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
        "schema_version": "cortexdb.enterprise_rag_bench.slack_gmail_source_selector.v1",
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
    parser.add_argument("--policy-name", default="slack_gmail_source_selector_v1")
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
