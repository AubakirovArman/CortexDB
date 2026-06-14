from __future__ import annotations

import argparse
import threading
from pathlib import Path
from typing import Any, Callable

from answer_artifacts import evidence_plan_for_row, evidence_table_for_row
from answer_chat import chat
from answer_context import load_context
from answer_guard import guard_unsupported_claims
from answer_prompts import build_prompt
from answer_repair import build_self_consistency_repair_prompt, should_self_consistency_repair

from . import log_state
from .budget import resolve_answer_budget


class AnswerWorker:
    def __init__(
        self,
        *,
        args: argparse.Namespace,
        api_key: str,
        uuid_index: dict[str, Any],
        evidence_plans: dict[str, Any],
        evidence_tables: dict[str, Any],
        rows_len: int,
        usage_totals: dict[str, int],
        usage_lock: threading.Lock,
        completed_count: Callable[[], int],
    ) -> None:
        self.args = args
        self.api_key = api_key
        self.uuid_index = uuid_index
        self.evidence_plans = evidence_plans
        self.evidence_tables = evidence_tables
        self.rows_len = rows_len
        self.usage_totals = usage_totals
        self.usage_lock = usage_lock
        self.completed_count = completed_count

    def generate(self, row: dict[str, Any]) -> dict[str, Any]:
        qid = str(row.get("question_id") or "")
        doc_ids = [str(item) for item in row.get("document_ids", [])]
        question = str(row.get("question", ""))
        budget = resolve_answer_budget(
            question=question,
            args=self.args,
        )
        context_mode = str(budget["context_mode"])
        top_k_context = int(budget["top_k_context"])
        max_chars_per_doc = int(budget["max_chars_per_doc"])
        max_tokens = int(budget["max_tokens"])
        answer_intent = str(budget["answer_intent"])
        log_state.log(
            "question start answer "
            f"question_id={qid} doc_count={len(doc_ids)} context_mode={context_mode} "
            f"top_k_context={top_k_context} max_tokens={max_tokens} answer_intent={answer_intent}"
        )
        log_state.LOGGER.status(
            stage="answer_generation",
            state="running",
            active_step="answer_question",
            active_question_id=qid,
            active_doc_count=len(doc_ids),
            active_context_mode=context_mode,
            active_answer_intent=answer_intent,
            active_top_k_context=top_k_context,
            model=self.args.model,
            completed_questions=self.completed_count(),
            total_questions=self.rows_len,
            pending_questions=max(0, self.rows_len - self.completed_count()),
            prompt_tokens=self.usage_totals["prompt_tokens"],
            completion_tokens=self.usage_totals["completion_tokens"],
            total_tokens=self.usage_totals["total_tokens"],
        )
        try:
            context = load_context(
                doc_ids[:top_k_context],
                self.uuid_index,
                self.args.sources_dir,
                max_chars_per_doc,
                question,
                context_mode,
            )
            evidence_plan = evidence_plan_for_row(
                row,
                self.evidence_plans,
                self.args.include_evidence_plan,
            )
            evidence_table = evidence_table_for_row(
                row=row,
                tables=self.evidence_tables,
                include=self.args.include_evidence_table,
                doc_ids=doc_ids[:top_k_context],
                uuid_index=self.uuid_index,
                sources_dir=self.args.sources_dir,
                max_facts_per_doc=self.args.max_evidence_facts_per_doc,
                max_table_rows=self.args.max_evidence_table_rows,
            )
            answer, usage, elapsed_ms = chat(
                api_key=self.api_key,
                base_url=self.args.base_url,
                model=self.args.model,
                prompt=build_prompt(
                    row,
                    context,
                    self.args.prompt_style,
                    evidence_plan,
                    evidence_table,
                ),
                max_tokens=max_tokens,
                retries=self.args.retries,
                omit_thinking_field=self.args.omit_thinking_field,
                gemini_native=self.args.gemini_native,
                gemini_thinking_budget=self.args.gemini_thinking_budget,
                openai_reasoning=getattr(self.args, "openai_reasoning", False),
            )
            answer_guard_result = {"mode": self.args.unsupported_claim_guard}
            if self.args.unsupported_claim_guard != "off":
                answer, answer_guard_result = guard_unsupported_claims(
                    answer,
                    context,
                    mode=self.args.unsupported_claim_guard,
                )
            self_consistency_result: dict[str, Any] = {
                "enabled": self.args.self_consistency_repair,
                "attempted": False,
            }
            if self.args.self_consistency_repair:
                _report_answer, consistency_report = guard_unsupported_claims(
                    answer,
                    context,
                    mode="report",
                )
                self_consistency_result["pre_repair_guard"] = consistency_report
                if should_self_consistency_repair(consistency_report):
                    repair_prompt = build_self_consistency_repair_prompt(
                        question=question,
                        context=context,
                        draft_answer=answer,
                        guard_report=consistency_report,
                    )
                    repaired_answer, repair_usage, repair_elapsed_ms = chat(
                        api_key=self.api_key,
                        base_url=self.args.base_url,
                        model=self.args.model,
                        prompt=repair_prompt,
                        max_tokens=max_tokens,
                        retries=self.args.self_consistency_retries,
                        omit_thinking_field=self.args.omit_thinking_field,
                        gemini_native=self.args.gemini_native,
                        gemini_thinking_budget=self.args.gemini_thinking_budget,
                        openai_reasoning=getattr(self.args, "openai_reasoning", False),
                    )
                    answer, repair_guard = guard_unsupported_claims(
                        repaired_answer,
                        context,
                        mode="repair",
                    )
                    for key in self.usage_totals:
                        usage[key] = int(usage.get(key, 0) or 0) + int(
                            repair_usage.get(key, 0) or 0
                        )
                    self_consistency_result.update(
                        {
                            "attempted": True,
                            "elapsed_ms": repair_elapsed_ms,
                            "guard": repair_guard,
                            "prompt_tokens": int(repair_usage.get("prompt_tokens", 0) or 0),
                            "completion_tokens": int(
                                repair_usage.get("completion_tokens", 0) or 0
                            ),
                            "total_tokens": int(repair_usage.get("total_tokens", 0) or 0),
                        }
                    )
        except Exception as error:
            log_state.log(f"question failed answer question_id={qid} error={error}")
            log_state.LOGGER.status(
                stage="answer_generation",
                state="failed",
                active_step="answer_question",
                active_question_id=qid,
                failed_question_id=qid,
                error=str(error),
                completed_questions=self.completed_count(),
                total_questions=self.rows_len,
                pending_questions=max(0, self.rows_len - self.completed_count()),
            )
            raise
        with self.usage_lock:
            for key in self.usage_totals:
                self.usage_totals[key] += int(usage.get(key, 0) or 0)
        log_state.log(
            "question done answer "
            f"question_id={qid} elapsed_ms={elapsed_ms} "
            f"prompt_tokens={int(usage.get('prompt_tokens', 0) or 0)} "
            f"completion_tokens={int(usage.get('completion_tokens', 0) or 0)} "
            f"total_tokens={int(usage.get('total_tokens', 0) or 0)}"
        )
        return {
            "question_id": qid,
            "answer": answer,
            "document_ids": doc_ids,
            "elapsed_ms": elapsed_ms,
            "prompt_tokens": int(usage.get("prompt_tokens", 0) or 0),
            "completion_tokens": int(usage.get("completion_tokens", 0) or 0),
            "total_tokens": int(usage.get("total_tokens", 0) or 0),
            "model": self.args.model,
            "context_mode": context_mode,
            "answer_intent": answer_intent,
            "answer_intent_score": budget["answer_intent_score"],
            "budget_profile": budget["budget_profile"],
            "active_top_k_context": top_k_context,
            "selected_result_limit": top_k_context,
            "active_max_chars_per_doc": max_chars_per_doc,
            "active_max_tokens": max_tokens,
            "retrieved_doc_count": len(doc_ids),
            "used_doc_count": min(len(doc_ids), top_k_context),
            "adaptive_budget_applied": budget["adaptive_budget_applied"],
            "high_level_override_applied": budget["high_level_override_applied"],
            "answer_guard": answer_guard_result,
            "self_consistency_repair": self_consistency_result,
            "prompt_style": self.args.prompt_style,
            "evidence_plan": "included" if evidence_plan else "none",
            "evidence_table": "included" if evidence_table else "none",
        }
