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


def run_graph_tests() -> dict:
    command = ["cargo", "test", "-p", "cortex-engine", "--test", "graph_tests"]
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


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--report", default="target/knowledge-graph/report.json")
    args = parser.parse_args()

    files_checked = [
        "docs/KNOWLEDGE_GRAPH.md",
        "docs/PRODUCTION_EPIC_EXECUTION_PLAN.md",
        "crates/cortex-engine/src/graph.rs",
        "crates/cortex-engine/tests/graph_tests.rs",
    ]
    errors = []
    errors.extend(
        require_markers(
            ROOT / "docs" / "KNOWLEDGE_GRAPH.md",
            [
                "Epic 145 Knowledge Graph Layer v1 Contract",
                "GraphEdgeKind",
                "type=entity",
                "type=relation",
                "source_supports_fact",
                "fact_contradicts_fact",
                "source references",
                "graph_source_supports_fact_edges",
                "graph_fact_contradicts_fact_edges",
                "make knowledge-graph-check",
            ],
        )
    )
    errors.extend(
        require_markers(
            ROOT / "docs" / "PRODUCTION_EPIC_EXECUTION_PLAN.md",
            [
                "### Epic 145. Knowledge Graph Layer v1",
                "Status: done",
                "make knowledge-graph-check",
            ],
        )
    )
    errors.extend(
        require_markers(
            ROOT / "crates" / "cortex-engine" / "src" / "graph.rs",
            [
                "pub enum GraphEdgeKind",
                "pub struct KnowledgeGraphIndex",
                "pub fn knowledge_graph_index",
                "pub fn cells_for_source",
                "pub fn graph_source_supports_fact_edges",
                "pub fn graph_fact_contradicts_fact_edges",
            ],
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

    test_result = run_graph_tests()
    report = {
        "schema_version": "cortexdb.knowledge_graph.report.v2",
        "status": "passed" if not errors and test_result["passed"] else "failed",
        "files_checked": files_checked,
        "knowledge_graph_docs_ok": not errors,
        "test_result": test_result,
        "errors": errors,
    }
    report_path = ROOT / args.report
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if not errors and test_result["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
