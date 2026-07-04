#!/usr/bin/env python3
"""F4.3: pgvector+OPA thin-wrapper adapter — runs the AAB-mini query set against a
real pgvector + OPA stack and captures the raw per-query rankings.

Flow (assumes `docker compose up` is running in this directory):
  1. embed the corpus + queries via the CORTEXDB_EMBEDDING_* endpoint (bge-m3);
  2. ingest cells into pgvector (scope + float embedding);
  3. per query: vector-search the full corpus (similarity only — pgvector has no
     notion of scope), then post-filter each candidate through OPA
     (`aab/access/allow` with the agent's readable_scopes), and keep the top-budget
     of the ALLOWED cells — exactly how a thin wrapper composes retrieval + authz;
  4. emit the raw ranking (all candidates + the allowed top-budget) per query.

The captured raw output is what capture_snapshot.py freezes; score_snapshot.py
computes the axis scores from it OFFLINE (no docker, no network).
"""
from __future__ import annotations

import json
import os
import pathlib
import subprocess
import time
import urllib.error
import urllib.request

HERE = pathlib.Path(__file__).resolve().parent
OPA_URL = os.environ.get("AAB_OPA_URL", "http://localhost:58181")


def embed(text: str) -> list[float]:
    url = os.environ["CORTEXDB_EMBEDDING_URL"]
    key = os.environ["CORTEXDB_EMBEDDING_API_KEY"]
    model = os.environ["CORTEXDB_EMBEDDING_MODEL"]
    body = json.dumps({"model": model, "input": text}).encode("utf-8")
    req = urllib.request.Request(
        url, data=body, headers={"Authorization": "Bearer " + key, "Content-Type": "application/json"}
    )
    for attempt in range(4):
        try:
            with urllib.request.urlopen(req, timeout=60) as r:
                return json.loads(r.read().decode("utf-8"))["data"][0]["embedding"]
        except urllib.error.HTTPError as e:
            if e.code in {429, 500, 502, 503, 504} and attempt < 3:
                time.sleep(min(20, 2 ** attempt))
                continue
            raise
    raise RuntimeError("embedding failed")


def psql(sql: str) -> str:
    """Run SQL inside the pgvector container (no host port / psycopg dependency)."""
    proc = subprocess.run(
        ["docker", "compose", "exec", "-T", "pgvector", "psql", "-U", "postgres", "-d", "aab", "-tA", "-v", "ON_ERROR_STOP=1"],
        cwd=HERE, input=sql, text=True, capture_output=True,
    )
    if proc.returncode != 0:
        raise RuntimeError(f"psql failed: {proc.stderr}\n---sql---\n{sql[:500]}")
    return proc.stdout


def opa_allow(cell_scope: str, readable_scopes: list[str]) -> bool:
    body = json.dumps({"input": {"cell_scope": cell_scope, "readable_scopes": readable_scopes}}).encode()
    req = urllib.request.Request(
        OPA_URL + "/v1/data/aab/access/allow", data=body, headers={"Content-Type": "application/json"}
    )
    with urllib.request.urlopen(req, timeout=15) as r:
        return bool(json.loads(r.read().decode()).get("result", False))


def vec_literal(vector: list[float]) -> str:
    return "[" + ",".join(f"{x:.6f}" for x in vector) + "]"


def ingest(corpus: list[dict]) -> int:
    dim = None
    rows = []
    for cell in corpus:
        v = embed(cell["text"])
        dim = dim or len(v)
        text_esc = cell["text"].replace("'", "''")
        rows.append(f"('{cell['cell_id']}','{cell['scope']}','{text_esc}','{vec_literal(v)}')")
    sql = (
        "CREATE EXTENSION IF NOT EXISTS vector;\n"
        "DROP TABLE IF EXISTS cells;\n"
        f"CREATE TABLE cells(cell_id text primary key, scope text, body text, embedding vector({dim}));\n"
        "INSERT INTO cells(cell_id, scope, body, embedding) VALUES\n" + ",\n".join(rows) + ";\n"
    )
    psql(sql)
    return dim


def search_with_authz(query: dict, top_n: int = 50) -> dict:
    qv = embed(query["text"])
    # pgvector ranks the WHOLE corpus by cosine distance — no scope awareness.
    out = psql(
        f"SELECT cell_id, scope FROM cells ORDER BY embedding <=> '{vec_literal(qv)}' ASC LIMIT {top_n};"
    )
    ranked = [line.split("|") for line in out.splitlines() if line.strip()]
    ranked = [{"cell_id": c, "scope": s} for c, s in ranked]
    # Thin-wrapper authz: OPA post-filters each candidate; keep the allowed top-budget.
    allowed = [c for c in ranked if opa_allow(c["scope"], query["readable_scopes"])]
    return {
        "query_id": query["query_id"],
        "axis": query["axis"],
        "readable_scopes": query["readable_scopes"],
        "budget": query["budget"],
        "ranked_all": [c["cell_id"] for c in ranked],
        "ranked_all_scopes": [c["scope"] for c in ranked],
        "allowed_top_budget": [c["cell_id"] for c in allowed[: query["budget"]]],
        "gold": {k: v for k, v in query.items() if k.startswith("gold")},
    }


def capture(corpus: list[dict], queries: list[dict]) -> dict:
    dim = ingest(corpus)
    results = [search_with_authz(q) for q in queries]
    return {"embedding_dim": dim, "results": results}
