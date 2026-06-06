#!/usr/bin/env python3
"""Report whether a real embedding ANN baseline run is ready to execute."""

from __future__ import annotations

import argparse
import contextlib
import io
import json
import os
import tempfile
import unittest
from pathlib import Path
from typing import Any

from embedding_provider import DEFAULT_KEY_ENV, DEFAULT_MODEL_ENV, DEFAULT_URL_ENV, provider_profile
from preflight_real_embedding_benchmark import (
    METRICS,
    embedding_config,
    optional_path,
    validate_embedding_provider,
    validate_queries,
    validate_sources,
)


def blocker(code: str, message: str) -> dict[str, str]:
    return {"code": code, "message": message}


def collect_readiness(args: argparse.Namespace) -> dict[str, Any]:
    blockers: list[dict[str, str]] = []
    source_files = 0
    source_rows = 0
    query_rows = 0

    if not args.source_root:
        blockers.append(blocker("missing_source_root", "set ANN_REAL_EMBEDDING_SOURCE_ROOT"))
    else:
        try:
            source_files, source_rows = validate_sources(args.source_root, args.max_documents)
        except ValueError as error:
            blockers.append(blocker("invalid_source_root", str(error)))

    if args.queries is None:
        blockers.append(blocker("missing_queries", "set ANN_REAL_EMBEDDING_QUERIES"))
    else:
        try:
            query_rows = validate_queries(args.queries)
        except ValueError as error:
            blockers.append(blocker("invalid_queries", str(error)))

    if args.metric not in METRICS:
        blockers.append(blocker("invalid_metric", f"unsupported metric: {args.metric}"))
    if args.scale <= 0 or args.scale > 32767:
        blockers.append(blocker("invalid_scale", "--scale must be in 1..32767"))
    if args.limit <= 0:
        blockers.append(blocker("invalid_limit", "--limit must be greater than zero"))
    if args.max_documents is not None and args.max_documents <= 0:
        blockers.append(blocker("invalid_max_documents", "--max-documents must be greater than zero"))

    config = embedding_config(args)
    try:
        validate_embedding_provider(args)
    except ValueError as error:
        blockers.append(blocker("invalid_embedding_provider", str(error)))

    missing_env = [name for name in args.require_env if not os.environ.get(name)]
    if missing_env:
        blockers.append(
            blocker(
                "missing_env",
                "missing required environment variables: " + ", ".join(sorted(missing_env)),
            )
        )

    if args.require_source_archive and args.source_archive_manifest is None:
        blockers.append(
            blocker(
                "missing_source_archive_manifest",
                "set ANN_REAL_EMBEDDING_SOURCE_ARCHIVE_MANIFEST for publishable baselines",
            )
        )
    elif args.source_archive_manifest is not None and not args.source_archive_manifest.is_file():
        blockers.append(
            blocker(
                "invalid_source_archive_manifest",
                f"{args.source_archive_manifest}: file not found",
            )
        )

    ready = not blockers
    report = {
        "schema_version": 1,
        "ready": ready,
        "status": "ready" if ready else "blocked",
        "blockers": blockers,
        "source_roots": [str(path) for path in args.source_root],
        "source_files": source_files,
        "source_rows": source_rows,
        "queries": str(args.queries) if args.queries is not None else "",
        "query_rows": query_rows,
        "metric": args.metric,
        "normalization": args.normalization,
        "scale": args.scale,
        "limit": args.limit,
        "required_env": sorted(args.require_env),
        "provider": args.provider,
        "embedding_command": args.embedding_command if args.provider == "command" else "",
        "embedding_model": provider_profile(config)["model"],
        "embedding_endpoint_origin": provider_profile(config)["endpoint_origin"],
        "embedding_provider": provider_profile(config),
        "source_archive_manifest": str(args.source_archive_manifest or ""),
    }
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return report


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path, action="append", default=[])
    parser.add_argument("--queries", type=Path)
    parser.add_argument(
        "--provider",
        choices=["command", "openai-compatible", "local", "file", "hash-smoke"],
        default="command",
    )
    parser.add_argument("--embedding-command", default="python3 scripts/ann/embed_text_command.py --require-model")
    parser.add_argument("--url")
    parser.add_argument("--url-env", default=DEFAULT_URL_ENV)
    parser.add_argument("--model")
    parser.add_argument("--model-env", default=DEFAULT_MODEL_ENV)
    parser.add_argument("--api-key-env", default=DEFAULT_KEY_ENV)
    parser.add_argument("--embedding-file", type=optional_path)
    parser.add_argument("--embedding-cache", type=optional_path)
    parser.add_argument("--timeout-seconds", type=float, default=30.0)
    parser.add_argument("--dimension", type=int)
    parser.add_argument("--hash-dimension", type=int, default=64)
    parser.add_argument("--require-model", action="store_true")
    parser.add_argument("--metric", default="cosine")
    parser.add_argument("--normalization", choices=["unit", "max_abs", "none"], default="unit")
    parser.add_argument("--scale", type=int, default=32767)
    parser.add_argument("--limit", type=int, default=10)
    parser.add_argument("--max-documents", type=int)
    parser.add_argument("--require-env", action="append", default=[])
    parser.add_argument("--source-archive-manifest", type=Path)
    parser.add_argument("--require-source-archive", action="store_true")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--fail-if-blocked", action="store_true")
    parser.add_argument("--self-test", action="store_true")
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    if "--self-test" in argv:
        suite = unittest.defaultTestLoader.loadTestsFromTestCase(SelfTests)
        return 0 if unittest.TextTestRunner().run(suite).wasSuccessful() else 1
    args = parse_args(argv)
    report = collect_readiness(args)
    print(json.dumps(report, sort_keys=True, separators=(",", ":")))
    if args.fail_if_blocked and not report["ready"]:
        return 2
    return 0


class SelfTests(unittest.TestCase):
    def test_reports_blockers_without_inputs(self) -> None:
        report = collect_readiness(parse_args([]))
        codes = {item["code"] for item in report["blockers"]}
        self.assertFalse(report["ready"])
        self.assertIn("missing_source_root", codes)
        self.assertIn("missing_queries", codes)

    def test_ready_with_valid_inputs_and_env(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            root = Path(raw_dir)
            source = root / "source"
            source.mkdir()
            (source / "cells.jsonl").write_text('{"payload":"alpha"}\n', encoding="utf-8")
            queries = root / "queries.jsonl"
            queries.write_text('{"name":"q","text":"alpha","limit":1}\n', encoding="utf-8")
            archive = root / "archive.json"
            archive.write_text('{"sha256":"abc"}\n', encoding="utf-8")
            args = parse_args([
                "--source-root",
                str(source),
                "--queries",
                str(queries),
                "--provider",
                "command",
                "--embedding-command",
                "python3 scripts/ann/embed_text_command.py --require-model",
                "--source-archive-manifest",
                str(archive),
                "--require-source-archive",
            ])
            report = collect_readiness(args)
            self.assertTrue(report["ready"])
            self.assertEqual(report["source_rows"], 1)
            self.assertEqual(report["query_rows"], 1)

    def test_ready_with_file_provider(self) -> None:
        with tempfile.TemporaryDirectory() as raw_dir:
            root = Path(raw_dir)
            source = root / "source"
            source.mkdir()
            (source / "cells.jsonl").write_text('{"payload":"alpha"}\n', encoding="utf-8")
            queries = root / "queries.jsonl"
            queries.write_text('{"name":"q","text":"alpha","limit":1}\n', encoding="utf-8")
            embeddings = root / "embeddings.jsonl"
            embeddings.write_text(
                json.dumps({"text": "alpha", "embedding": [1.0, 0.0]}) + "\n",
                encoding="utf-8",
            )
            args = parse_args([
                "--source-root",
                str(source),
                "--queries",
                str(queries),
                "--provider",
                "file",
                "--embedding-file",
                str(embeddings),
            ])
            report = collect_readiness(args)
            self.assertTrue(report["ready"])
            self.assertEqual(report["embedding_provider"]["provider"], "file")

    def test_fail_if_blocked_returns_nonzero(self) -> None:
        with contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(main(["--fail-if-blocked"]), 2)


if __name__ == "__main__":
    raise SystemExit(main(__import__("sys").argv[1:]))
