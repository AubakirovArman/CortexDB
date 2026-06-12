#!/usr/bin/env python3
import argparse
import json
import subprocess
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def require_text(path: Path, markers: list[str]) -> list[str]:
    text = path.read_text(encoding="utf-8")
    missing = [marker for marker in markers if marker not in text]
    return [f"{path.relative_to(ROOT)} missing marker: {marker}" for marker in missing]


def run_tests() -> dict:
    command = [
        "cargo",
        "test",
        "-p",
        "cortex-engine",
        "--test",
        "tool_registry_tests",
    ]
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
    parser.add_argument("--report", default="target/tool-registry/report.json")
    args = parser.parse_args()

    checks = []
    checks.extend(
        require_text(
            ROOT / "docs" / "TOOL_REGISTRY.md",
            [
                "Epic 143 Tool Registry v1 Contract",
                "ToolDescriptor",
                "KnowledgeCellType::Tool",
                "permissions=read,execute,approval_required",
                "Add tool cells",
                "Add permissions",
                "Add input/output schema",
                "Add tool retrieval by task",
                "Database::list_tools(view)",
                "Tool Retrieval By Task",
                "Database::recommend_tools_for_task(view, task, limit)",
                "make tool-registry-check",
            ],
        )
    )
    checks.extend(
        require_text(
            ROOT / "docs" / "PRODUCTION_EPIC_EXECUTION_PLAN.md",
            ["### Epic 143. Tool Registry v1", "Status: done", "make tool-registry-check"],
        )
    )
    checks.extend(
        require_text(
            ROOT / "crates" / "cortex-engine" / "src" / "tool_registry.rs",
            [
                "pub struct ToolDescriptor",
                "pub enum ToolPermission",
                "pub struct ToolRecommendation",
                "pub fn register_tool",
                "pub fn recommend_tools_for_task",
            ],
        )
    )
    checks.extend(
        require_text(
            ROOT / "crates" / "cortex-engine" / "tests" / "tool_registry_tests.rs",
            ["tool_retrieval_by_task_returns_relevant_tool_cell"],
        )
    )

    test_result = run_tests()
    status = "passed" if not checks and test_result["passed"] else "failed"
    report = {
        "schema_version": "cortexdb.tool_registry.report.v2",
        "status": status,
        "files_checked": [
            "docs/TOOL_REGISTRY.md",
            "docs/archive/PRODUCTION_EPIC_EXECUTION_PLAN.md",
            "crates/cortex-engine/src/tool_registry.rs",
            "crates/cortex-engine/tests/tool_registry_tests.rs",
        ],
        "tool_registry_docs_ok": not checks,
        "test_result": test_result,
        "errors": checks,
    }
    report_path = ROOT / args.report
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    if status != "passed":
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
