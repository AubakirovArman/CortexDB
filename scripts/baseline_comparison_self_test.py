"""Self-test for the C20 baseline comparison gate."""

from __future__ import annotations

import argparse
import json
import tempfile
from pathlib import Path

from baseline_comparison_common import Q16_ONE, write_json
from baseline_comparison_report import build_report


def run_self_test() -> None:
    with tempfile.TemporaryDirectory() as temp_dir:
        root = Path(temp_dir)
        corpus = root / "corpus.jsonl"
        queries = root / "queries.jsonl"
        truth = root / "truth.jsonl"
        datasets = root / "datasets.json"
        features = root / "features.json"
        retrieval = root / "retrieval.json"
        context = root / "context.json"
        corpus.write_text(
            "\n".join([
                json.dumps({"chunk_id": "c1", "doc_id": "d1", "title": "Alpha", "text": "red alpha project"}),
                json.dumps({"chunk_id": "c2", "doc_id": "d2", "title": "Beta", "text": "blue beta project"}),
            ]) + "\n",
            encoding="utf-8",
        )
        queries.write_text(
            json.dumps({"query_id": "q1", "query": "red alpha"}) + "\n",
            encoding="utf-8",
        )
        truth.write_text(
            json.dumps({"query_id": "q1", "relevant_chunk_ids": ["c1"]}) + "\n",
            encoding="utf-8",
        )
        write_json(datasets, {
            "datasets": [{
                "domain": "demo",
                "corpus": str(corpus),
                "queries": str(queries),
                "ground_truth": str(truth),
            }],
        })
        write_json(features, {
            "schema_version": "cortexdb.baseline_comparison.features.v1",
            "rows": [{"feature": "f", "naive_stack": "n", "cortexdb": "c", "evidence": "e"}],
        })
        write_json(retrieval, {
            "domains": [{
                "domain": "demo",
                "latest_mean_recall_q16": Q16_ONE,
                "latest_p95_latency_nanos": 1,
            }],
        })
        write_json(context, {
            "status": "passed",
            "case_count": 1,
            "external_dataset_count": 1,
            "failure_category_count": 1,
            "evidence_coverage_q16": Q16_ONE,
            "citation_coverage_q16": Q16_ONE,
            "token_reduction_q16": 1,
        })
        args = argparse.Namespace(
            repo_root=root,
            datasets=datasets,
            features=features,
            cortexdb_retrieval_report=retrieval,
            context_pack_report=context,
            top_k=1,
            repeat_runs=1,
            min_domains=1,
        )
        report = build_report(args)
        if report["status"] != "passed":
            raise AssertionError(report["failures"])
        row = report["comparison"][0]
        if row["sqlite_fts5_mean_hit_recall_q16"] != Q16_ONE:
            raise AssertionError("SQLite FTS5 self-test did not retrieve the relevant chunk")
