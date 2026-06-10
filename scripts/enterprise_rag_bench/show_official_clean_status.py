#!/usr/bin/env python3
"""Show live EnterpriseRAG official-clean progress from status JSON files."""

from __future__ import annotations

import argparse
import json
import time
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
DEFAULT_RUN_DIR = ROOT / "target/enterprise-rag-bench/official-clean/500"


def read_json(path: Path) -> dict[str, Any]:
    payload = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise ValueError(f"{path}: expected JSON object")
    return payload


def compact_path(value: Any) -> str:
    text = str(value)
    try:
        path = Path(text)
        return str(path.relative_to(ROOT))
    except (ValueError, RuntimeError):
        return text


def present(value: Any) -> bool:
    return value is not None and value != "" and value != []


def field(payload: dict[str, Any], *names: str) -> Any:
    for name in names:
        value = payload.get(name)
        if present(value):
            return value
    return None


def progress_label(payload: dict[str, Any]) -> str:
    completed = field(payload, "completed", "completed_questions", "completed_missing")
    total = field(payload, "total", "total_questions", "missing_texts")
    pct = payload.get("progress_pct")
    if completed is not None and total is not None:
        suffix = f" ({pct}%)" if pct is not None else ""
        return f"{completed}/{total}{suffix}"
    step = payload.get("step")
    total_steps = payload.get("total_steps")
    if step is not None and total_steps is not None:
        return f"step {step}/{total_steps}"
    return "unknown"


def token_label(payload: dict[str, Any]) -> str:
    prompt = payload.get("prompt_tokens")
    completion = payload.get("completion_tokens")
    total = payload.get("total_tokens")
    if prompt is None and completion is None and total is None:
        return ""
    return f"tokens prompt={prompt or 0} completion={completion or 0} total={total or 0}"


def render_status(payload: dict[str, Any], *, title: str, indent: int = 0) -> list[str]:
    pad = " " * indent
    lines = [f"{pad}{title}"]
    lines.append(
        f"{pad}- state: {payload.get('state', 'unknown')}"
        f" | stage: {payload.get('stage', 'unknown')}"
        f" | progress: {progress_label(payload)}"
    )
    operation = field(payload, "subprocess_label", "active_step")
    operation_bits = []
    if operation:
        operation_bits.append(f"operation={operation}")
    for name in ("pid", "provider", "model"):
        value = payload.get(name)
        if present(value):
            operation_bits.append(f"{name}={value}")
    if operation_bits:
        lines.append(f"{pad}- current: " + " | ".join(operation_bits))
    if payload.get("split_name") or payload.get("questions_file"):
        lines.append(
            f"{pad}- split: {payload.get('split_name') or 'unknown'}"
            f" | questions: {compact_path(payload.get('questions_file'))}"
        )
    timing = []
    for name in ("updated_at", "elapsed", "elapsed_seconds", "eta_seconds"):
        value = payload.get(name)
        if present(value):
            timing.append(f"{name}={value}")
    if timing:
        lines.append(f"{pad}- timing: " + " | ".join(timing))
    active_question = field(payload, "active_question_id", "last_question_id")
    if active_question:
        lines.append(f"{pad}- question: {active_question}")
    detail_bits = []
    for name in (
        "queued_questions",
        "workers",
        "pending_questions",
        "active_doc_count",
        "active_top_k_context",
        "active_context_mode",
        "active_document_recall_pct",
        "active_invalid_extra_docs",
        "overall",
        "correctness",
        "completeness",
    ):
        value = payload.get(name)
        if present(value):
            detail_bits.append(f"{name}={value}")
    if detail_bits:
        lines.append(f"{pad}- details: " + " | ".join(detail_bits))
    tokens = token_label(payload)
    if tokens:
        lines.append(f"{pad}- {tokens}")
    log_file = payload.get("log_file")
    if present(log_file):
        lines.append(f"{pad}- log: {compact_path(log_file)}")
    last_output = payload.get("last_output_line")
    if present(last_output):
        lines.append(f"{pad}- last: {last_output}")
    error = payload.get("error")
    if present(error):
        lines.append(f"{pad}- error: {error}")
    artifacts = payload.get("artifacts")
    if isinstance(artifacts, dict) and artifacts:
        lines.append(f"{pad}- artifacts:")
        for key, value in sorted(artifacts.items()):
            lines.append(f"{pad}  - {key}: {compact_path(value)}")
    child = payload.get("child_status")
    if isinstance(child, dict):
        lines.extend(render_status(child, title="child status", indent=indent + 2))
    return lines


def default_status_path(run_dir: Path) -> Path:
    return run_dir / "official_clean_status.json"


def tail_lines(path: Path, count: int) -> list[str]:
    if count <= 0 or not path.exists():
        return []
    lines = path.read_text(encoding="utf-8", errors="replace").splitlines()
    return lines[-count:]


def collect_log_files(payload: dict[str, Any]) -> list[Path]:
    values: list[Path] = []
    for key in ("run_log", "log_file"):
        value = payload.get(key)
        if present(value):
            values.append(Path(str(value)))
    child = payload.get("child_status")
    if isinstance(child, dict):
        values.extend(collect_log_files(child))
    deduped: list[Path] = []
    seen: set[str] = set()
    for value in values:
        key = str(value)
        if key not in seen:
            seen.add(key)
            deduped.append(value)
    return deduped


def output_once(path: Path, *, as_json: bool, tail_count: int) -> None:
    payload = read_json(path)
    if as_json:
        print(json.dumps(payload, indent=2, sort_keys=True))
        return
    print("\n".join(render_status(payload, title=f"status: {compact_path(path)}")))
    if tail_count > 0:
        for log_file in collect_log_files(payload):
            tail = tail_lines(log_file, tail_count)
            if not tail:
                continue
            print("")
            print(f"log tail: {compact_path(log_file)}")
            for line in tail:
                print(f"  {line}")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run-dir", type=Path, default=DEFAULT_RUN_DIR)
    parser.add_argument("--status-file", type=Path)
    parser.add_argument("--watch", action="store_true")
    parser.add_argument("--interval-seconds", type=float, default=5.0)
    parser.add_argument(
        "--tail-lines",
        type=int,
        default=0,
        help="Print the last N lines from run/child log files after each status render.",
    )
    parser.add_argument("--json", action="store_true", help="Print the raw status JSON.")
    args = parser.parse_args()

    status_file = args.status_file or default_status_path(args.run_dir)
    if args.interval_seconds <= 0:
        parser.error("--interval-seconds must be positive")
    if args.tail_lines < 0:
        parser.error("--tail-lines must be non-negative")
    if not status_file.exists():
        raise FileNotFoundError(f"status file not found: {status_file}")

    if not args.watch:
        output_once(status_file, as_json=args.json, tail_count=args.tail_lines)
        return 0

    while True:
        output_once(status_file, as_json=args.json, tail_count=args.tail_lines)
        print("", flush=True)
        time.sleep(args.interval_seconds)


if __name__ == "__main__":
    raise SystemExit(main())
