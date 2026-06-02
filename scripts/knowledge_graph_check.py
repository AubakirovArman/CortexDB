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

    errors = []
    errors.extend(
        require_markers(
            ROOT / "docs" / "KNOWLEDGE_GRAPH.md",
            [
                "type=entity",
                "type=relation",
                "source references",
                "make knowledge-graph-check",
            ],
        )
    )
    errors.extend(
        require_markers(
            ROOT / "docs" / "NEXT_60_EPICS.md",
            ["| 58 | Knowledge Graph Layer | closed |"],
        )
    )
    errors.extend(
        require_markers(
            ROOT / "crates" / "cortex-engine" / "src" / "graph.rs",
            [
                "pub struct KnowledgeGraphIndex",
                "pub fn knowledge_graph_index",
                "pub fn cells_for_source",
            ],
        )
    )

    test_result = run_graph_tests()
    report = {
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
