# CortexDB API JSON Schemas

Version: `v0.1.0-core-alpha`

The HTTP API serializes response bodies from typed Rust structs with
`serde::Serialize`.

The OpenAPI 3.1 contract lives in [`docs/openapi.yaml`](docs/openapi.yaml).
The stable error taxonomy is frozen in
[`docs/API_ERROR_TAXONOMY.md`](docs/API_ERROR_TAXONOMY.md).

All database endpoints accept optional query parameter `tenant=<realm>`.
Omitting it or sending `tenant=default` targets the root database. Other values
target per-tenant database realms and are supported by the dashboard and SDKs.

## Common Error

Returned by failed routes, auth failures, payload limit failures, and database
open errors.

```json
{
  "code": "bad_request",
  "error": "bad_request",
  "message": "missing scope"
}
```

Fields:

- `code`: typed stable error enum used by SDKs.
- `error`: stable short error code.
- `message`: human-readable safe message.

Stable Core Alpha error codes:

| HTTP status | Code | Meaning |
| --- | --- | --- |
| `400` | `bad_request` | Missing parameters or malformed non-AQL request input. |
| `400` | `invalid_tenant` | Tenant realm name fails charset, length, or path-safety validation. |
| `400` | `invalid_aql` | AQL parse/bind failure that is not a policy denial. |
| `401` | `unauthorized` | Missing or invalid bearer token. |
| `403` | `forbidden` | Non-AgentView authorization denial, including data-token access to admin/metrics routes. |
| `403` | `permission_denied` | AgentView, scope, mode, or policy denial. |
| `404` | `not_found` | Unknown route or missing resource such as an ingestion job. |
| `413` | `payload_too_large` | Request body exceeds server limit. |
| `429` | `rate_limited` | Optional server request-rate limit is exceeded. |
| `503` | `database_busy` | Database actor queue or database lock is busy. |
| `503` | `service_unavailable` | Server component is unavailable but not classified as queue/lock pressure. |
| `500` | `storage_corruption` | Storage checksum, format, or invariant failure. |
| `500` | `internal` | Unexpected internal error that is not classified above. |

## Health

`GET /v1/health`

```json
{
  "status": "ok",
  "version": "v1"
}
```

## Cell Read

`GET /v1/cell?cell_id=<u64>`

```json
{
  "cell": {
    "cell_id": 1,
    "payload": "scope=project:investments\nstatus=ready\n..."
  }
}
```

Missing cells return:

```json
{
  "cell": null
}
```

## Cell Write / Tombstone

`POST /v1/cell?cell_id=<u64>` and `DELETE /v1/cell?cell_id=<u64>`

```json
{
  "seq": 7,
  "cell_id": 1
}
```

## Auth Policy Mutation

Admin-only local policy-store lifecycle routes. They require
`CORTEXDB_AUTH_POLICY_STORE_FILE` and do not expose bearer token values in
responses. Successful mutations also sync a redacted durable policy-cell mirror
under the local `_system:auth_policy` scope; the mirror stores role, AgentView
binding, quota, capabilities, disabled state, and token fingerprint, not the raw
token.

`POST /v1/admin/auth/principal`

```json
{
  "principal_id": "agent-a",
  "token": "agent-token",
  "role": "data",
  "agent_id": 7,
  "request_quota_per_minute": 600,
  "body_quota_bytes_per_minute": 1048576,
  "queue_quota": 2,
  "capabilities": ["search", "read"]
}
```

If `capabilities` is present, the principal is further restricted to those API
action classes. Omitting the field preserves the default behavior for the
principal role. Quota fields are optional fixed-window local guards:
`request_quota_per_minute` counts requests, `body_quota_bytes_per_minute`
counts accepted request body bytes, and `queue_quota` limits concurrent actor
commands for the token/principal.

`DELETE /v1/admin/auth/principal?principal_id=agent-a`

`POST /v1/admin/auth/policy/rollback`

Response:

```json
{
  "schema_version": "cortexdb.auth_policy_mutation.v1",
  "action": "upsert_principal",
  "principal_id": "agent-a",
  "active_principals": 1,
  "disabled_principals": 0,
  "rollback_available": true
}
```

## Flush / Compact

`POST /v1/flush` and `POST /v1/compact`

```json
{
  "checkpoint_seq": 7,
  "cells_flushed": 3
}
```

## Stats

`GET /v1/stats`

```json
{
  "current_seq": 7,
  "checkpoint_seq": 7,
  "live_segments": 1,
  "retired_segments": 0,
  "memtable_cells": 3,
  "memtable_versions": 3,
  "memtable_payload_bytes": 2048,
  "estimated_memtable_bytes": 4096,
  "estimated_index_bytes": 8192,
  "estimated_context_pack_bytes": 16384,
  "estimated_total_memory_bytes": 28672,
  "wal_size_bytes": 128,
  "wal_writer_records": 3,
  "wal_writer_bytes": 512,
  "wal_writer_fsyncs": 3,
  "wal_writer_batches": 3
}
```

## Validate

`GET /v1/validate`

```json
{
  "ok": true,
  "manifest_ok": true,
  "wal_ok": true,
  "live_segments_checked": 1,
  "bitmap_indexes_checked": 1,
  "lexical_indexes_checked": 1,
  "vector_indexes_checked": 0,
  "hnsw_graphs_checked": 0,
  "cells_checked": 3,
  "wal_records_checked": 0,
  "wal_safe_truncate_offset": 0,
  "errors": []
}
```

## Metrics

`GET /v1/metrics`

```json
{
  "current_seq": 7,
  "checkpoint_seq": 7,
  "live_segments": 1,
  "retired_segments": 0,
  "memtable_cells": 3,
  "memtable_versions": 3,
  "memtable_payload_bytes": 2048,
  "estimated_memtable_bytes": 4096,
  "estimated_index_bytes": 8192,
  "estimated_context_pack_bytes": 16384,
  "estimated_total_memory_bytes": 28672,
  "wal_size_bytes": 128,
  "wal_writer_records": 3,
  "wal_writer_bytes": 512,
  "wal_writer_fsyncs": 3,
  "wal_writer_batches": 3,
  "ann_graph_nodes": 0,
  "ann_total_edges": 0,
  "ann_persisted_segments": 0,
  "ann_has_checkpoint": false,
  "ann_has_uncheckpointed_changes": false,
  "ann_search_requests": 0,
  "ann_fallbacks": 0,
  "ann_no_fallback_requests": 0,
  "ann_no_fallback_allowed": 0,
  "ann_no_fallback_blocked": 0,
  "ann_search_latency_ms": {
    "count": 0,
    "sum_ms": 0,
    "le_10_ms": 0,
    "le_50_ms": 0,
    "le_100_ms": 0,
    "le_500_ms": 0,
    "le_1000_ms": 0,
    "gt_1000_ms": 0
  },
  "actor_queue_depth": 0,
  "actor_queue_capacity": 1024,
  "request_count": 10,
  "request_rejected": 0,
  "request_duration_ms_total": 42,
  "validation_failures": 0,
  "principal_quota_requests_allowed": 10,
  "principal_quota_requests_rejected": 0,
  "principal_quota_body_bytes_allowed": 512,
  "principal_quota_body_bytes_rejected": 0,
  "principal_quota_queue_acquired": 8,
  "principal_quota_queue_rejected": 0
}
```

## AQL Retrieve

`POST /v1/aql?scope=<scope>` with an AQL body.

```json
{
  "cells": [
    {
      "cell_id": 1,
      "payload": "scope=project:investments\nstatus=ready\n..."
    }
  ]
}
```

`EXPLAIN RETRIEVE CONTEXT ...` uses the same endpoint and returns an empty
`cells` array plus an `explain` report. The report is diagnostic output for the
bound AQL plan; it does not return payload rows.

```json
{
  "cells": [],
  "explain": {
    "task": "budget",
    "brain_id": 1,
    "selected_mode": "balanced",
    "bitmap_plan": "BitmapProgram(max_stack_depth=3)\n0000: PushAgentAllowed",
    "bitmap_ops": ["PushAgentAllowed", "PushLive", "And"],
    "filters": [
      {"kind": "policy", "expression": "agent_allowed"},
      {"kind": "liveness", "expression": "live"},
      {"kind": "where", "expression": "status = \"ready\""}
    ],
    "candidate_counts": {
      "universe": 2,
      "agent_allowed": 2,
      "live": 2,
      "after_bitmap": 1,
      "after_quality": 1,
      "returned_limit": 1
    },
    "candidate_limit": 10,
    "budget_tokens": 2048,
    "citations_required": false
  }
}
```

## Search

`POST /v1/search?scope=<scope>&q=<term>` or
`POST /v1/search?scope=<scope>&mode=vector&algorithm=exact&vector=1,2,3`

Use `mode=auto` to let CortexDB choose keyword, vector, or hybrid routing from
the available query text and vector:

```text
POST /v1/search?scope=<scope>&mode=auto&q=solar+budget&vector=1,2,3
```

```json
{
  "search_mode": "keyword",
  "routing": {
    "requested_mode": "auto",
    "selected_strategy": "keyword",
    "reason": "auto_text_only_or_default",
    "text_available": true,
    "vector_available": false
  },
  "ann_report": null,
  "results": [
    {
      "cell_id": 1,
      "score": 65535,
      "lexical_score": 65535,
      "vector_score": 0,
      "payload": "scope=project:investments\nstatus=ready\n..."
    }
  ]
}
```

`routing.selected_strategy` is the strategy the engine actually used:
`keyword`, `vector_exact`, `vector_ann`, or `hybrid`. `routing.reason` explains
whether the route was explicit or chosen by `mode=auto`.

For `search_mode: "vector_ann"`, `ann_report` is populated:

```json
{
  "path": "exact_fallback",
  "fallback_reason": "no_persisted_segments",
  "fallback_performed": true,
  "requested_limit": 20,
  "allowed_candidates": 1,
  "graph_nodes": 0,
  "returned_candidates": 1,
  "recall_q16": null,
  "min_recall_q16": null,
  "hnsw_max_neighbors": 0,
  "hnsw_ef_search": 0,
  "hnsw_ef_construction": 0,
  "hnsw_layer_count": 1,
  "upper_graph_edges": 0,
  "require_slo": true,
  "production_safe": false,
  "slo_violations": ["no_persisted_segments"]
}
```

If callers explicitly pass `no_fallback_rollout=true`, or opt into the
persisted operator profile with `no_fallback_profile=active`, responses also
include a `no_fallback_decision` object. This is an operator-facing guardrail:
it reports whether the current ANN result is allowed for fallback-free serving,
but it does not remove exact fallback globally.

```json
{
  "allowed": false,
  "reasons": ["fallback_enabled", "slo_not_required"]
}
```

## Search Explain

`POST /v1/search/explain?scope=<scope>&q=<query>&mode=keyword`

For hybrid inspection, pass both query text and vector:

`POST /v1/search/explain?scope=<scope>&mode=hybrid&q=<query>&vector=1,2,3`

```json
{
  "query_terms": ["budget"],
  "search_mode": "hybrid",
  "results": [
    {
      "cell_id": 1,
      "rank": 1,
      "score": 32786,
      "lexical_score": 42,
      "vector_score": 100,
      "lexical_contribution_q16": 19383,
      "vector_contribution_q16": 46152,
      "fusion_rank_score": 32786,
      "matched_terms": ["budget"],
      "term_contributions": [
        {
          "term": "budget",
          "term_frequency": 2,
          "score": 42
        }
      ],
      "contribution_summary": "hybrid rrf_score=32786 lexical_score=42 vector_score=100",
      "payload_preview": "scope=project:investments\nstatus=ready\n..."
    }
  ]
}
```

`term_contributions` explains lexical matches. `vector_score` is the raw vector
similarity score. `fusion_rank_score` is non-zero when a result has both lexical
and vector evidence in hybrid mode.

## HNSW No-fallback Profile

The admin profile endpoint persists the local operator rollout policy used when
ANN calls pass `no_fallback_profile=active`.

```json
{
  "configured": true,
  "rollout_enabled": true,
  "min_recall_q16": 65535,
  "require_upper_layers": true
}
```

An unconfigured or cleared profile returns the same shape with nullable profile
fields:

```json
{
  "configured": false,
  "rollout_enabled": null,
  "min_recall_q16": null,
  "require_upper_layers": null
}
```

`fallback_reason` may also be `low_recall` when the HNSW graph returns enough
candidates but fails the exact top-k recall guard. It may be `hnsw_disabled`
when production-safe defaults intentionally use exact persisted vector search
instead of the experimental HNSW graph. In that case `production_safe=true`.
With `require_slo=true`, callers should treat `production_safe=false` as an
ANN/HNSW guardrail breach even when exact fallback returned correct results.
The HNSW profile fields (`hnsw_max_neighbors`, `hnsw_ef_search`,
`hnsw_ef_construction`, `hnsw_layer_count`) identify the persisted graph shape
used for recall and latency comparisons.

## ANN Evaluation

`POST /v1/search/ann-evaluate?scope=<scope>&vector=1,2,3&limit=20`

```json
{
  "available": true,
  "reason": null,
  "ann_report": {
    "path": "hnsw_graph",
    "fallback_reason": null,
    "fallback_performed": false,
    "requested_limit": 20,
    "allowed_candidates": 2,
    "graph_nodes": 2,
    "returned_candidates": 2,
    "recall_q16": 65535,
    "min_recall_q16": 65535,
    "require_slo": true,
    "production_safe": true,
    "slo_violations": []
  },
  "exact_top_k": [2, 1],
  "ann_top_k": [2, 1],
  "no_fallback_decision": {
    "allowed": true,
    "reasons": []
  },
  "overlap_count": 2,
  "recall_q16": 65535
}
```

When the database has no checkpointed vector snapshot, or when newer WAL tail
changes exist, `available` is `false` and `reason` is
`requires_persisted_checkpoint_without_wal_tail`.

## Context Pack

`POST /v1/context?scope=<scope>` with an AQL body returns typed JSON by default.
Use `format=prompt` for a stable agent prompt export or `format=markdown` for a
stable Markdown export.

```json
{
  "schema_version": "context_pack.v1",
  "token_budget_tokens": 1000,
  "estimated_tokens": 42,
  "truncated": false,
  "citations_required": false,
  "cells": [
    {
      "cell_id": 1,
      "estimated_tokens": 42,
      "citation": "doc-a",
      "payload_text": "scope=project:investments\nstatus=ready\n...",
      "explain": {
        "score": 65535,
        "matched_terms": ["budget"],
        "why_selected": "matched query terms",
        "score_components": [
          {
            "name": "base_bm25",
            "value": 65535,
            "contribution": 65535,
            "reason": "keyword overlap between query terms and cell body"
          },
          {
            "name": "source_trust_bonus",
            "value": 0,
            "contribution": 0,
            "reason": "default provenance trust because source_trust_q16 is absent"
          },
          {
            "name": "redundancy_penalty",
            "value": 0,
            "contribution": 0,
            "reason": "weighted Jaccard overlap with already packed cells"
          }
        ],
        "base_bm25": 65535,
        "source_trust_q16": 32768,
        "source_trust_category": "unknown",
        "source_trust_bonus": 0,
        "redundancy_penalty": 0
      },
      "source_ref": {
        "source_id": "doc-a",
        "source_url": "https://example.test/doc-a",
        "document_id": null,
        "page": null,
        "cell_range": null,
        "json_path": null,
        "confidence_q16": 65535
      }
    }
  ],
  "anomalies": []
}
```

Prompt export:

```text
CortexDB ContextPack v1

Use only the context cells below.
Preserve citations when answering.
If the supplied context is insufficient or conflicting, say so.

Budget: token_budget_tokens=1000 estimated_tokens=42 truncated=false citations_required=false

Context cells:

[1] cell_id=1
estimated_tokens=42
citation=doc-a
text:
scope=project:investments
status=ready
...
```

Markdown export:

```markdown
# CortexDB ContextPack

- schema_version: `context_pack.v1`
- token_budget_tokens: `1000`
- estimated_tokens: `42`
- truncated: `false`
- citations_required: `false`

## Cells

### Cell 1

- cell_id: `1`
- estimated_tokens: `42`
- citation: `doc-a`
```

## LLM Inference Test-double Endpoint

`POST /v1/inference` is an opt-in deterministic test-double contract. It is
disabled by default and must be enabled with `CORTEXDB_LLM_TEST_DOUBLE=true`.
It consumes explicit ContextPack input, never calls an external provider, never
retrieves context internally, and must not receive provider API keys.

`/v1/llm` and `/v1/chat` remain intentionally absent.

```json
{
  "schema_version": "cortexdb.llm_inference.smoke_response.v1",
  "provider": "test_double",
  "model": "deterministic-echo-v1",
  "output": "Test-double answer from explicit ContextPack only: ...",
  "used_context_cell_ids": [101],
  "citations": ["doc://investment-risk#p1"],
  "audit": {
    "context_pack_only": true,
    "prompt_body_logged": false,
    "secrets_logged": false
  }
}
```

## Remember

`POST /v1/remember?scope=<scope>` with a `REMEMBER` AQL body.

```json
{
  "seq": 8,
  "cell_id": 42,
  "ttl_seconds": 60
}
```

## Verify

`POST /v1/verify?scope=<scope>` with a `VERIFY FACT` AQL body.

Supported fact:

```json
{
  "fact": "The budget is 1.2B KZT",
  "status": "supported",
  "verdict": "supported",
  "evidence": [
    {
      "cell_id": 1,
      "matched_terms": 5,
      "source_trust_q16": 32768,
      "source_trust_category": "unknown",
      "citation": "doc-a",
      "payload_text": "scope=project:investments\nstatus=ready\n..."
    }
  ],
  "contradicting_evidence": [],
  "guards": [],
  "supporting": [
    {
      "cell_id": 1,
      "matched_terms": 5,
      "source_trust_q16": 32768,
      "source_trust_category": "unknown",
      "citation": "doc-a",
      "payload_text": "scope=project:investments\nstatus=ready\n..."
    }
  ],
  "contradicting": [],
  "numeric_conflicts": []
}
```

Mixed evidence with numeric conflict:

```json
{
  "fact": "The budget is 1.2B KZT",
  "status": "mixed",
  "verdict": "mixed_evidence",
  "evidence": [
    {
      "cell_id": 1,
      "matched_terms": 5,
      "source_trust_q16": 32768,
      "source_trust_category": "unknown",
      "citation": "doc-a",
      "payload_text": "scope=project:investments\nstatus=ready\n..."
    }
  ],
  "contradicting_evidence": [
    {
      "cell_id": 2,
      "matched_terms": 2,
      "source_trust_q16": 32768,
      "source_trust_category": "unknown",
      "citation": "doc-b",
      "payload_text": "scope=project:investments\nstatus=ready\n..."
    }
  ],
  "guards": [
    {
      "cell_id": 2,
      "code": "numeric_mismatch",
      "message": "payload numeric claim differs from fact numeric claim"
    }
  ],
  "supporting": [
    {
      "cell_id": 1,
      "matched_terms": 5,
      "source_trust_q16": 32768,
      "source_trust_category": "unknown",
      "citation": "doc-a",
      "payload_text": "scope=project:investments\nstatus=ready\n..."
    }
  ],
  "contradicting": [
    {
      "cell_id": 2,
      "matched_terms": 2,
      "source_trust_q16": 32768,
      "source_trust_category": "unknown",
      "citation": "doc-b",
      "payload_text": "scope=project:investments\nstatus=ready\n..."
    }
  ],
  "numeric_conflicts": [
    {
      "metric": "budget",
      "left": "1.2B KZT",
      "right": "1400000000 KZT"
    }
  ]
}
```

## Ingestion

`POST /v1/ingest/text`, `POST /v1/ingest/json`, and
`POST /v1/ingest/csv`. Each route accepts optional `scope` and `source` query
parameters.

```json
{
  "rows_ingested": 0,
  "chunks_ingested": 0,
  "facts_ingested": 0,
  "first_cell_id": null,
  "job_id": 1,
  "validation_report": {
    "cells_seen": 0,
    "warnings": [],
    "skipped_items": [
      {"reason": "no_cells_emitted", "input_ref": "ingest_text"}
    ],
    "source_refs": []
  }
}
```

Empty inputs return zero counts, `first_cell_id: null`, and a
`validation_report.skipped_items` entry. Non-empty ingestion reports include
per-cell SourceRef summaries so clients can verify whether chunks/facts are
citable.

`GET /v1/ingest/jobs/<job_id>` returns the persisted ingestion job progress
object for jobs created by engine-side job workflows. `POST
/v1/ingest/jobs/<job_id>/retry` moves a failed job back to `queued`, `POST
/v1/ingest/jobs/<job_id>/cancel` cancels queued/running jobs, and `DELETE
/v1/ingest/jobs/<job_id>` deletes a persisted job record. Job records are
durable local metadata; on database restart, stale `running` jobs are atomically
requeued as `queued` and retain their completed item counters and last cell id.
