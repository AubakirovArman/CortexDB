#!/usr/bin/env python3
"""Run EnterpriseRAG-Bench official answer metrics with a local judge env file."""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
from pathlib import Path


def load_env_file(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    if not path.exists():
        return values
    for raw in path.read_text(encoding="utf-8").splitlines():
        line = raw.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        values[key.strip()] = value.strip().strip("'\"")
    return values


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--official-repo", type=Path, required=True)
    parser.add_argument("--python", type=Path, required=True)
    parser.add_argument("--answers-file", type=Path, required=True)
    parser.add_argument("--questions-file", type=Path, required=True)
    parser.add_argument("--results-file", type=Path, required=True)
    parser.add_argument("--updated-questions-file", type=Path, required=True)
    parser.add_argument("--uuid-index-cache-file", default="generated_data/uuid_index.json")
    parser.add_argument("--judge-env-file", type=Path, required=True)
    parser.add_argument("--provider", default="openai")
    parser.add_argument("--model", default="gpt-4o-mini")
    parser.add_argument("--api-key-var", default="OPENAI_API_KEY")
    parser.add_argument("--limit", type=int)
    parser.add_argument("--question-id")
    parser.add_argument("--timeout-seconds", type=int)
    parser.add_argument("--no-correction", action="store_true")
    parser.add_argument("--skip-citation-stripping", action="store_true")
    args = parser.parse_args()
    if args.limit is not None and args.limit <= 0:
        parser.error("--limit must be positive")
    if args.timeout_seconds is not None and args.timeout_seconds <= 0:
        parser.error("--timeout-seconds must be positive")
    return args


def main() -> int:
    args = parse_args()
    values = load_env_file(args.judge_env_file)
    api_key = values.get(args.api_key_var) or os.environ.get(args.api_key_var)
    if not api_key:
        raise RuntimeError(f"judge key not found in {args.api_key_var}")

    env = os.environ.copy()
    env["LLM_PROVIDER"] = args.provider
    env["LLM_API_KEY"] = api_key
    env["LLM_MODEL_NAME"] = args.model

    command = [
        str(args.python),
        "-m",
        "src.scripts.answer_evaluation.metrics_based_eval",
        "--answers-file",
        str(args.answers_file.resolve()),
        "--questions-file",
        str(args.questions_file.resolve()),
        "--results-file",
        str(args.results_file.resolve()),
        "--updated-questions-file",
        str(args.updated_questions_file.resolve()),
        "--uuid-index-cache-file",
        args.uuid_index_cache_file,
    ]
    if args.no_correction:
        command.append("--no-correction")
    if args.skip_citation_stripping:
        command.append("--skip-citation-stripping")
    if args.limit is not None:
        command.extend(["--limit", str(args.limit)])
    if args.question_id:
        command.extend(["--question-id", args.question_id])

    try:
        completed = subprocess.run(
            command,
            cwd=args.official_repo,
            env=env,
            check=False,
            timeout=args.timeout_seconds,
        )
    except subprocess.TimeoutExpired:
        print(
            f"official EnterpriseRAG judge timed out after {args.timeout_seconds}s",
            file=sys.stderr,
        )
        return 124
    return completed.returncode


if __name__ == "__main__":
    raise SystemExit(main())
