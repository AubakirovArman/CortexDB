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
        "context_pack_tool_recommendation",
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
    parser.add_argument(
        "--report", default="target/context-pack-tool-recommendation/report.json"
    )
    args = parser.parse_args()

    errors = []
    errors.extend(
        require_text(
            ROOT / "docs" / "CONTEXT_PACK_TOOL_RECOMMENDATION.md",
            [
                "ContextPackWithTools",
                "ToolRecommendation",
                "why_selected",
                "Database::recommend_tools_for_task(view, task, tool_limit)",
                "make context-pack-tool-recommendation-check",
            ],
        )
    )
    errors.extend(
        require_text(
            ROOT / "docs" / "PRODUCTION_EPIC_EXECUTION_PLAN.md",
            [
                "### Epic 144. Tool Recommendation in ContextPack",
                "Status: done",
                "make context-pack-tool-recommendation-check",
            ],
        )
    )
    errors.extend(
        require_text(
            ROOT / "crates" / "cortex-engine" / "src" / "context" / "mod.rs",
            ["pub struct ContextPackWithTools"],
        )
    )
    errors.extend(
        require_text(
            ROOT / "crates" / "cortex-engine" / "src" / "context" / "pack.rs",
            ["context_pack_with_tool_recommendations_from_aql"],
        )
    )
    errors.extend(
        require_text(
            ROOT
            / "crates"
            / "cortex-engine"
            / "tests"
            / "context_pack_tool_recommendation.rs",
            [
                "context_pack_with_tools_includes_relevant_tool_and_explanation",
                "context_pack_tool_recommendations_respect_agent_scope_and_limit",
            ],
        )
    )

    test_result = run_tests()
    status = "passed" if not errors and test_result["passed"] else "failed"
    report = {
        "schema_version": "cortexdb.context_pack_tool_recommendation.report.v1",
        "status": status,
        "files_checked": [
            "docs/CONTEXT_PACK_TOOL_RECOMMENDATION.md",
            "docs/archive/PRODUCTION_EPIC_EXECUTION_PLAN.md",
            "crates/cortex-engine/src/context/mod.rs",
            "crates/cortex-engine/src/context/pack.rs",
            "crates/cortex-engine/tests/context_pack_tool_recommendation.rs",
        ],
        "test_result": test_result,
        "errors": errors,
    }
    report_path = ROOT / args.report
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))
    return 0 if status == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
