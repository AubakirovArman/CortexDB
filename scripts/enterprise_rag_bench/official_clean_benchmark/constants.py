"""Shared paths and constants for official-clean benchmark runs."""

from __future__ import annotations

from pathlib import Path


ROOT = Path(__file__).resolve().parents[3]
BENCH_ROOT = ROOT / "target/external-benchmarks/EnterpriseRAG-Bench"
OUT_ROOT = ROOT / "target/enterprise-rag-bench/official-clean"
MAX_LOG_LINE_CHARS = 4_000
