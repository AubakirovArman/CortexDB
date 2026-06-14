"""CLI entrypoint for the official-clean EnterpriseRAG-Bench orchestrator."""

from __future__ import annotations

import datetime as dt
import json

from .artifacts import (
    paths,
    run_progress_artifacts,
    run_summary,
    stage_artifacts,
    stage_child_status,
)
from .args import build_parser, validate_args
from .stages import selected_stages
from .status import (
    log,
    set_run_log,
    set_run_metadata,
    set_run_status,
    set_stage_context,
    write_current_status,
    write_json,
    write_status,
)


def main() -> int:
    parser = build_parser()
    args = parser.parse_args()
    validate_args(parser, args)

    p = paths(
        args.size,
        args.answer_provider,
        args.judge_provider,
        args.db_root,
        args.run_label,
    )
    split_name = args.split_name.strip()
    set_run_metadata(split_name, args.questions_file)
    set_run_log(p["run_log"])
    started_at = dt.datetime.now(dt.UTC).isoformat(timespec="seconds")
    set_run_status(p["status"], started_at)
    log(
        "start run "
        f"size={args.size} answer_provider={args.answer_provider} "
        f"judge_provider={args.judge_provider} stage={args.stage} reuse_db={args.reuse_db} "
        f"split={split_name} questions_file={args.questions_file} "
        f"run_log={p['run_log']} status={p['status']}"
    )
    stages = selected_stages(args.stage)
    for index, (name, runner) in enumerate(stages, 1):
        set_stage_context(name, index, len(stages))
        log(f"step {index}/{len(stages)} {name}: start")
        write_status(
            p["status"],
            stage=name,
            step=index,
            total_steps=len(stages),
            state="running",
            started_at=started_at,
            artifacts=stage_artifacts(name, p),
            child_status=stage_child_status(name, p),
        )
        try:
            runner(args, p)
        except Exception as error:
            write_status(
                p["status"],
                stage=name,
                step=index,
                total_steps=len(stages),
                state="failed",
                started_at=started_at,
                error=str(error),
                artifacts=stage_artifacts(name, p),
                child_status=stage_child_status(name, p),
            )
            log(f"step {index}/{len(stages)} {name}: failed error={error}")
            raise
        write_status(
            p["status"],
            stage=name,
            step=index,
            total_steps=len(stages),
            state="done",
            started_at=started_at,
            artifacts=stage_artifacts(name, p),
            child_status=stage_child_status(name, p),
        )
        log(f"step {index}/{len(stages)} {name}: done")

    report = {
        "schema_version": "cortexdb.enterprise_rag_bench.official_clean_run.v1",
        "size": args.size,
        "answer_provider": args.answer_provider,
        "judge_provider": args.judge_provider,
        "run_label": args.run_label,
        "split_name": split_name,
        "questions_file": str(args.questions_file),
        "stage": args.stage,
        "reuse_db": args.reuse_db,
        "skip_checkpoint": args.skip_checkpoint,
        "retrieval_mode": args.retrieval_mode,
        "rerank": args.rerank,
        "embedding_rerank": args.embedding_rerank,
        "embedding_rerank_candidates": args.embedding_rerank_candidates if args.embedding_rerank else None,
        "prompt_style": args.prompt_style,
        "gemini_thinking_budget": args.gemini_thinking_budget,
        "context_mode": args.context_mode,
        "include_evidence_plan": args.include_evidence_plan,
        "evidence_plan_file": str(args.evidence_plan_file) if args.evidence_plan_file else None,
        "include_evidence_table": args.include_evidence_table,
        "evidence_table_file": str(args.evidence_table_file) if args.evidence_table_file else None,
        "self_consistency_repair": args.self_consistency_repair,
        "self_consistency_retries": args.self_consistency_retries,
        "query_vectors": str(args.query_vectors) if args.query_vectors else None,
        "document_vectors": str(args.document_vectors) if args.document_vectors else None,
        "prefilter_retrieval": str(args.prefilter_retrieval)
        if args.prefilter_retrieval
        else None,
        "max_documents": args.max_documents,
        "clean_questions": str(p["clean_questions"]),
        "clean_retrieval": str(p["clean_retrieval"]),
        "retrieval_status": str(p["retrieval_status"]),
        "answers": str(p["answers"]),
        "judge_results": str(p["judge_results"]),
        "run_log": str(p["run_log"]),
        "status": str(p["status"]),
        "prepare_status": str(p["prepare_status"]),
        "answer_status": str(p["answer_status"]),
        "judge_status": str(p["judge_status"]),
        "progress_artifacts": run_progress_artifacts(p),
        "summary": run_summary(p),
        "rule": "Inference input is question_id/question plus retrieved document_ids only; gold labels are used only by the judge stage.",
        "inference_oracle_policy": {
            "allowed_question_fields": ["question_id", "question"],
            "forbidden_question_fields": [
                "answer_facts",
                "expected_doc_ids",
                "gold_answer",
                "question_type",
                "source_types",
            ],
            "gold_usage": "judge-only",
        },
    }
    write_json(p["run_report"], report)
    set_stage_context("finished", len(stages), len(stages))
    write_current_status(
        state="done",
        subprocess_label="finished run",
        artifacts={
            "run_report": p["run_report"],
            "run_log": p["run_log"],
            "clean_retrieval": p["clean_retrieval"],
            "answers": p["answers"],
            "judge_results": p["judge_results"],
        },
    )
    summary = report["summary"]
    if isinstance(summary, dict):
        judge_summary = summary.get("judge")
        answer_summary = summary.get("answer")
        if isinstance(judge_summary, dict) or isinstance(answer_summary, dict):
            log(
                "final summary "
                f"overall={judge_summary.get('overall') if isinstance(judge_summary, dict) else None} "
                f"correctness={judge_summary.get('answer_correctness_pct') if isinstance(judge_summary, dict) else None} "
                f"completeness={judge_summary.get('answer_completeness_pct') if isinstance(judge_summary, dict) else None} "
                f"answer_tokens={answer_summary.get('total_tokens') if isinstance(answer_summary, dict) else None} "
                f"judge_tokens={judge_summary.get('total_tokens') if isinstance(judge_summary, dict) else None}"
            )
    log(f"wrote run report {p['run_report']}")
    print(json.dumps(report, sort_keys=True))
    log("finished run")
    return 0

