#!/usr/bin/env python3
import argparse
import json
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def require_markers(path: Path, markers: list[str]) -> list[str]:
    text = path.read_text(encoding="utf-8")
    return [
        f"{path.relative_to(ROOT)} missing marker: {marker}"
        for marker in markers
        if marker not in text
    ]


def reject_markers(path: Path, markers: list[str]) -> list[str]:
    text = path.read_text(encoding="utf-8")
    return [
        f"{path.relative_to(ROOT)} contains forbidden marker: {marker}"
        for marker in markers
        if marker in text
    ]


def run_test_command(command: list[str]) -> dict:
    completed = subprocess.run(
        command,
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    return {
        "command": " ".join(command),
        "returncode": completed.returncode,
        "passed": completed.returncode == 0,
        "output_tail": completed.stdout[-4000:],
    }


def run_graph_tests() -> list[dict]:
    return [
        run_test_command(["cargo", "test", "-p", "cortex-engine", "--test", test_name])
        for test_name in [
            "graph_tests",
            "graph_index_incremental_tests",
            "graph_retrieval_tests",
            "verification_graph_tests",
        ]
    ]


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", default="target/knowledge-graph/report.json")
    args = parser.parse_args()

    files_checked = [
        "docs/KNOWLEDGE_GRAPH.md",
        "docs/archive/PRODUCTION_EPIC_EXECUTION_PLAN.md",
        "crates/cortex-engine/src/graph.rs",
        "crates/cortex-engine/src/graph/types.rs",
        "crates/cortex-engine/src/graph/index.rs",
        "crates/cortex-engine/src/graph/index_helpers.rs",
        "crates/cortex-engine/src/graph/database.rs",
        "crates/cortex-engine/src/graph/store.rs",
        "crates/cortex-engine/src/graph_retrieval.rs",
        "crates/cortex-engine/src/bin/graph_index_performance_check.rs",
        "crates/cortex-engine/src/verification/graph.rs",
        "crates/cortex-engine/tests/graph_tests.rs",
        "crates/cortex-engine/tests/graph_index_incremental_tests.rs",
        "crates/cortex-engine/tests/graph_retrieval_tests.rs",
        "crates/cortex-engine/tests/verification_graph_tests.rs",
    ]
    errors = []
    errors.extend(
        require_markers(
            ROOT / "docs" / "KNOWLEDGE_GRAPH.md",
            [
                "Knowledge Graph/Provenance Index Contract",
                "EPIC-C15",
                "GraphEdgeKind",
                "type=entity",
                "type=relation",
                "source_supports_fact",
                "fact_contradicts_fact",
                "source references",
                "GraphIndexStore",
                "incremental",
                "source_support_edges_by_fact",
                "Graph Retrieval",
                "GraphRetrievalHit",
                "GraphRetrievalReport",
                "visit_budget",
                "proximity_score_q16",
                "explaining_edges",
                "VERIFY source-support",
                "graph_source_supports_fact_edges",
                "graph_fact_contradicts_fact_edges",
                "make knowledge-graph-check",
                "make graph-index-performance-check",
            ],
        )
    )
    errors.extend(
        require_markers(
            ROOT / "docs" / "archive" / "PRODUCTION_EPIC_EXECUTION_PLAN.md",
            [
                "### Epic 145. Knowledge Graph Layer v1",
                "Status: done",
                "make knowledge-graph-check",
                "### Epic 146. Graph Retrieval",
                "GraphRetrievalHit",
            ],
        )
    )
    errors.extend(
        require_markers(
            ROOT / "crates" / "cortex-engine" / "src" / "graph" / "types.rs",
            [
                "pub enum GraphEdgeKind",
                "pub struct KnowledgeGraphIndex",
                "GraphNodeId",
                "edge_ids_by_entity",
                "edges_by_id",
                "source_support_edges_by_fact",
            ],
        )
    )
    errors.extend(
        require_markers(
            ROOT / "crates" / "cortex-engine" / "src" / "graph" / "database.rs",
            [
                "pub fn knowledge_graph_index",
                "pub fn graph_source_supports_fact_edges",
                "pub fn graph_fact_contradicts_fact_edges",
            ],
        )
    )
    errors.extend(
        require_markers(
            ROOT / "crates" / "cortex-engine" / "src" / "graph" / "index.rs",
            [
                "source_supports_fact_edges_for_cells",
                "index_record",
                "add_record",
                "remove_cell",
            ],
        )
    )
    errors.extend(
        require_markers(
            ROOT / "crates" / "cortex-engine" / "src" / "graph" / "store.rs",
            [
                "fn insert_record",
                "fn insert_record_unchecked",
                "fn remove_record",
                "self.index.remove_cell(cell_id)",
            ],
        )
    )
    errors.extend(
        reject_markers(
            ROOT / "crates" / "cortex-engine" / "src" / "graph" / "store.rs",
            ["fn rebuild"],
        )
    )
    errors.extend(
        reject_markers(
            ROOT / "crates" / "cortex-engine" / "src" / "graph" / "database.rs",
            ["visible_iter", "payload_for_version"],
        )
    )
    errors.extend(
        require_markers(
            ROOT / "crates" / "cortex-engine" / "src" / "graph_retrieval.rs",
            [
                "pub struct GraphRetrievalHit",
                "pub struct GraphRetrievalReport",
                "pub fn graph_retrieve_related",
                "pub fn graph_retrieve_related_with_budget",
                "pub fn retrieve_related_cells",
                "retrieve_related_cells_with_budget",
                "budget_exceeded",
                "proximity_score_q16",
                "explaining_edges",
            ],
        )
    )
    errors.extend(
        require_markers(
            ROOT
            / "crates"
            / "cortex-engine"
            / "src"
            / "bin"
            / "graph_index_performance_check.rs",
            [
                "cortexdb.graph_index_performance.v1",
                "100_000",
                "max_p95_ms",
                "budget_exceeded_samples",
            ],
        )
    )
    errors.extend(
        require_markers(
            ROOT / "crates" / "cortex-engine" / "src" / "verification" / "graph.rs",
            [
                "source_supports_fact_edges_for_cells(&target_cells)",
                "merge_source_supports_from_edges",
            ],
        )
    )
    errors.extend(
        reject_markers(
            ROOT
            / "crates"
            / "cortex-engine"
            / "src"
            / "verification"
            / "operator"
            / "candidates.rs",
            ["verification_source_support_versions"],
        )
    )
    errors.extend(
        require_markers(
            ROOT / "crates" / "cortex-engine" / "tests" / "graph_tests.rs",
            [
                "knowledge_graph_indexes_source_supports_fact_edges",
                "knowledge_graph_indexes_fact_contradicts_fact_edges",
            ],
        )
    )
    errors.extend(
        require_markers(
            ROOT
            / "crates"
            / "cortex-engine"
            / "tests"
            / "graph_index_incremental_tests.rs",
            [
                "knowledge_graph_incremental_store_matches_rebuild_after_mutations",
                "lazy_knowledge_graph_queries_do_not_materialize_payloads",
            ],
        )
    )
    errors.extend(
        require_markers(
            ROOT / "crates" / "cortex-engine" / "tests" / "verification_graph_tests.rs",
            ["VERIFY should read the fact and matching source-support relation only"],
        )
    )
    errors.extend(
        require_markers(
            ROOT / "crates" / "cortex-engine" / "tests" / "graph_retrieval_tests.rs",
            [
                "graph_retrieve_related_walks_multiple_hops",
                "graph_retrieve_related_scores_by_proximity",
                "graph_retrieve_related_explains_edges_for_hits",
                "graph_retrieve_related_reports_visit_budget_exceeded",
                "graph_retrieve_related_zero_budget_returns_seed_only",
            ],
        )
    )

    test_results = run_graph_tests()
    tests_passed = all(result["passed"] for result in test_results)
    report = {
        "schema_version": "cortexdb.knowledge_graph.report.v2",
        "status": "passed" if not errors and tests_passed else "failed",
        "files_checked": files_checked,
        "knowledge_graph_docs_ok": not errors,
        "test_results": test_results,
        "errors": errors,
    }
    report_path = ROOT / args.report
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if not errors and tests_passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
