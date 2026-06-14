#!/usr/bin/env python3
"""Company-scope fallback route for high-level zero-document questions.

When the official-clean retrieval pipeline returns no documents for a question
that looks like a company/strategy overview query, this script runs a dense-only
top-K retrieval over the whole corpus and injects those candidates back into the
retrieval row. It is oracle-free: it uses only the question text and document
vectors, never question_type / source_types / expected_doc_ids / gold labels.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).resolve().parent))

from oracle_free_abstain import is_high_level_question


def read_jsonl(path: Path) -> list[dict[str, Any]]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        "".join(json.dumps(row, ensure_ascii=True, sort_keys=True) + "\n" for row in rows),
        encoding="utf-8",
    )


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def run_dense_fallback(
    questions_file: Path,
    retrieval_file: Path,
    corpus_vectors: Path,
    env_file: Path,
    output: Path,
    dense_top_k: int,
    top_k: int,
    query_cache: Path | None,
    report: Path,
) -> None:
    """Call dense_candidates.py with empty lexical retrieval -> dense-only top-k."""
    cmd = [
        "python3",
        "scripts/enterprise_rag_bench/dense_candidates.py",
        "--questions",
        str(questions_file),
        "--corpus-vectors",
        str(corpus_vectors),
        "--lexical-retrieval",
        str(retrieval_file),
        "--output",
        str(output),
        "--env-file",
        str(env_file),
        "--dense-top-k",
        str(dense_top_k),
        "--top-k",
        str(top_k),
        "--report",
        str(report),
    ]
    if query_cache:
        cmd.extend(["--query-cache", str(query_cache)])
    subprocess.run(cmd, check=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--retrieval-file", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--corpus-vectors", type=Path, required=True)
    parser.add_argument("--env-file", type=Path, default=Path(".env"))
    parser.add_argument("--dense-top-k", type=int, default=100)
    parser.add_argument("--top-k", type=int, default=8)
    parser.add_argument("--query-cache", type=Path)
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()

    questions = {str(row["question_id"]): row for row in read_jsonl(args.questions_file)}
    retrieval_rows = read_jsonl(args.retrieval_file)

    fallback_qids: list[str] = []
    for row in retrieval_rows:
        qid = str(row["question_id"])
        docs = [str(d) for d in row.get("document_ids", []) if str(d)]
        if docs:
            continue
        qtext = str(questions.get(qid, {}).get("question", ""))
        if qtext and is_high_level_question(qtext):
            fallback_qids.append(qid)

    if not fallback_qids:
        write_jsonl(args.output, retrieval_rows)
        report_payload = {
            "schema_version": "cortexdb.enterprise_rag_bench.company_scope_route.v1",
            "questions": len(retrieval_rows),
            "fallback_questions": 0,
            "note": "No high-level zero-document questions; retrieval copied unchanged.",
        }
        if args.report:
            write_json(args.report, report_payload)
        print(json.dumps(report_payload, sort_keys=True))
        return 0

    with tempfile.TemporaryDirectory(prefix="company-scope-") as tmpdir:
        tmp = Path(tmpdir)
        fallback_questions = [questions[qid] for qid in fallback_qids]
        fallback_retrieval = [
            {
                "answer": "",
                "document_ids": [],
                "question": questions[qid].get("question", ""),
                "question_id": qid,
            }
            for qid in fallback_qids
        ]
        fq_file = tmp / "fallback_questions.jsonl"
        fr_file = tmp / "fallback_retrieval.jsonl"
        fc_file = tmp / "fallback_dense.jsonl"
        freport_file = tmp / "fallback_dense_report.json"
        write_jsonl(fq_file, fallback_questions)
        write_jsonl(fr_file, fallback_retrieval)

        query_cache = args.query_cache or (tmp / "query_vectors.jsonl")
        run_dense_fallback(
            questions_file=fq_file,
            retrieval_file=fr_file,
            corpus_vectors=args.corpus_vectors,
            env_file=args.env_file,
            output=fc_file,
            dense_top_k=args.dense_top_k,
            top_k=args.top_k,
            query_cache=query_cache,
            report=freport_file,
        )

        fallback_rows = {str(row["question_id"]): row for row in read_jsonl(fc_file)}
        output_rows: list[dict[str, Any]] = []
        changed = 0
        for row in retrieval_rows:
            qid = str(row["question_id"])
            if qid in fallback_rows:
                new_docs = fallback_rows[qid].get("document_ids", [])
                if new_docs:
                    row = dict(row)
                    row["document_ids"] = new_docs
                    row["route"] = {
                        "policy": "company_scope_dense_fallback",
                        "source": "dense-only",
                        "reason": "high_level_zero_doc",
                    }
                    changed += 1
            output_rows.append(row)

    write_jsonl(args.output, output_rows)
    report_payload = {
        "schema_version": "cortexdb.enterprise_rag_bench.company_scope_route.v1",
        "questions": len(retrieval_rows),
        "fallback_questions": len(fallback_qids),
        "changed_rows": changed,
        "fallback_question_ids": fallback_qids,
    }
    if args.report:
        write_json(args.report, report_payload)
    print(json.dumps(report_payload, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
