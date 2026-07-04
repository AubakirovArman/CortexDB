#!/usr/bin/env python3
"""F4.3: capture a pgvector+OPA thin-wrapper snapshot (run ONCE on a docker host).

Brings up the pinned pgvector+OPA stack, runs the AAB-mini query set through the
adapter, and freezes the raw per-query rankings + a pinned-digest manifest into a
versioned snapshot under fixtures/aab/snapshots/. The nightly gate re-scores the
committed snapshot fully offline (see score_snapshot.py) — this metered/docker step
is a one-time capture, not part of CI.
"""
from __future__ import annotations

import argparse
import json
import pathlib
import subprocess
import sys
import time

REPO = pathlib.Path(__file__).resolve().parents[2]
ADAPTER_DIR = REPO / "scripts" / "aab" / "adapters" / "pgvector_opa"
sys.path.insert(0, str(ADAPTER_DIR))
import adapter  # noqa: E402


def read_jsonl(path: pathlib.Path) -> list[dict]:
    return [json.loads(l) for l in path.read_text(encoding="utf-8").splitlines() if l.strip()]


def compose(*args: str) -> subprocess.CompletedProcess:
    return subprocess.run(["docker", "compose", *args], cwd=ADAPTER_DIR, text=True, capture_output=True)


def wait_healthy(timeout: int = 60) -> None:
    for _ in range(timeout // 2):
        r = compose("ps", "--format", "{{.Health}}", "pgvector")
        if r.stdout.strip() == "healthy":
            return
        time.sleep(2)
    raise RuntimeError("pgvector did not become healthy")


def image_digests() -> dict:
    out = {}
    for svc in ("pgvector", "opa"):
        r = compose("images", "--format", "{{.Repository}}:{{.Tag}}@{{.ID}}", svc)
        out[svc] = r.stdout.strip()
    return out


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--snapshot", default="pgvector-opa@pg16",
                    help="snapshot dir under fixtures/aab/snapshots/")
    ap.add_argument("--keep-up", action="store_true", help="leave containers running")
    args = ap.parse_args()

    snap_dir = REPO / "fixtures" / "aab" / "snapshots" / args.snapshot
    corpus = read_jsonl(snap_dir / "inputs" / "corpus.jsonl")
    queries = read_jsonl(snap_dir / "inputs" / "queries.jsonl")

    compose("up", "-d")
    try:
        wait_healthy()
        digests = image_digests()
        captured = adapter.capture(corpus, queries)
    finally:
        if not args.keep_up:
            compose("down")

    out_dir = snap_dir / "captured"
    out_dir.mkdir(parents=True, exist_ok=True)
    with (out_dir / "results.jsonl").open("w", encoding="utf-8") as h:
        for row in captured["results"]:
            h.write(json.dumps(row, sort_keys=True) + "\n")
    (out_dir / "manifest.json").write_text(
        json.dumps({
            "system": "thin-pgvector-opa",
            "adapter": "scripts/aab/adapters/pgvector_opa",
            "images": digests,
            "embedding_dim": captured["embedding_dim"],
            "corpus_cells": len(corpus),
            "queries": len(queries),
        }, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    print(f"captured {len(captured['results'])} queries -> {out_dir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
