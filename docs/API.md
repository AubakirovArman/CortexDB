# CortexDB HTTP API Contract Specification (v0.1.0-core-alpha)

CortexDB exposes a lightweight Core Alpha HTTP JSON API for interacting with
Agent Memory, Retrieving Context, and Verifying Facts. This contract is covered
by schema checks, but it is not a production SLA.

---

## 1. Global Server Information

* **Default Base URL:** `http://127.0.0.1:8181`
* **Content-Type:** `application/json`
* **Max Payload Boundary:** 2MB (Requests exceeding 2MB will return `413 Payload Too Large`)
* **OpenAPI contract:** [`openapi.yaml`](openapi.yaml)
* **Stable error taxonomy:** [`API_ERROR_TAXONOMY.md`](API_ERROR_TAXONOMY.md)
* **Tenant routing:** database endpoints accept optional `tenant=<realm>`. Omit
  it or use `tenant=default` for the root database; any other value routes to a
  per-tenant realm under the server data directory.
* **Backpressure:** each tenant uses a bounded `DatabaseActor` queue. Set
  `CORTEXDB_ACTOR_QUEUE_CAPACITY` to override the default `1024`; full queues
  return `503 database_busy`.
* **Rate limiting:** disabled by default. Set
  `CORTEXDB_RATE_LIMIT_PER_MINUTE` to enable a coarse process-wide fixed-window
  request limit; exceeded windows return `429 rate_limited`.
* **CORS:** disabled by default. Set `CORTEXDB_CORS_ALLOW_ORIGIN` to one exact
  trusted browser origin when cross-origin browser calls are required.
* **Request IDs:** responses include `x-request-id`. If the request provides a
  safe `x-request-id` header, the server echoes it; otherwise it generates a
  `cortexdb-<n>` id for log and audit correlation.

---

## 2. Common API Endpoints

### 2.1. GET `/v1/health`
Checks the server health status.

* **Response (200 OK):**
  ```json
  {
    "status": "ok",
    "version": "v1"
  }
  ```

---

### 2.2. GET `/v1/stats`
Retrieves detailed storage, segment, and WAL engine metrics.

* **Response (200 OK):**
  ```json
  {
    "current_seq": 15,
    "checkpoint_seq": 10,
    "live_segments": 2,
    "retired_segments": 0,
    "memtable_cells": 5,
    "memtable_versions": 5,
    "wal_size_bytes": 1024,
    "wal_writer_records": 15,
    "wal_writer_bytes": 1280,
    "wal_writer_fsyncs": 4,
    "wal_writer_batches": 2
  }
  ```

---

### 2.3. GET `/v1/validate`
Validates the structural and checksum integrity of all storage segments, bitmap indexes, lexical indices, and WAL records.

* **Response (200 OK):**
  ```json
  {
    "ok": true,
    "manifest_ok": true,
    "wal_ok": true,
    "live_segments_checked": 2,
    "bitmap_indexes_checked": 2,
    "lexical_indexes_checked": 2,
    "vector_indexes_checked": 2,
    "hnsw_graphs_checked": 1,
    "cells_checked": 5,
    "wal_records_checked": 0,
    "wal_safe_truncate_offset": 16,
    "errors": []
  }
  ```

---

### 2.4. GET `/v1/cell?cell_id=<cell_id>`
Retrieves a single raw knowledge cell by its ID.

* **Response (200 OK):**
  ```json
  {
    "cell": {
      "cell_id": 1,
      "payload": "scope=project:investments\nstatus=ready\n\nSolar Plant approved budget is 1.2B KZT."
    }
  }
  ```

* **Missing Cell Response (200 OK):**
  ```json
  {
    "cell": null
  }
  ```

---

### 2.5. POST `/v1/cell?cell_id=<cell_id>`
Writes or overwrites a single knowledge cell payload.

* **Request Body:** Raw text/bytes payload.
* **Response (200 OK):**
  ```json
  {
    "seq": 16,
    "cell_id": 1
  }
  ```

---

### 2.6. Admin auth policy mutation

These routes require an `admin` token and a configured
`CORTEXDB_AUTH_POLICY_STORE_FILE`. They mutate only the local JSON policy-store
file and create a rollback snapshot before publishing the new policy.

* `POST /v1/admin/auth/principal`
* `DELETE /v1/admin/auth/principal?principal_id=<principal_id>`
* `POST /v1/admin/auth/policy/rollback`

* **Request Body for upsert:**
  ```json
  {
    "principal_id": "agent-a",
    "token": "agent-token",
    "role": "data",
    "agent_id": 7,
    "request_quota_per_minute": 600
  }
  ```

* **Response (200 OK):**
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

---

### 2.7. POST `/v1/context?scope=<scope>`
Executes an AQL query and compiles a budgeted, deduplicated, and scored `ContextPack`.

* **Request Body:** Raw AQL query string.
* **Formats:** JSON is the default. Use `format=prompt` for a stable agent
  prompt export or `format=markdown` for a stable Markdown export.
* **Response (200 OK):**
  ```json
  {
    "schema_version": "context_pack.v1",
    "token_budget_tokens": 1000,
    "estimated_tokens": 126,
    "truncated": false,
    "citations_required": false,
    "cells": [
      {
        "cell_id": 1,
        "estimated_tokens": 63,
        "citation": "report_q1.pdf#page=3",
        "payload_text": "scope=project:investments\nstatus=ready\n...",
        "explain": {
          "score": 42768,
          "matched_terms": ["budget"],
          "why_selected": "contains relevant matched terms with standard provenance trust",
          "base_bm25": 10000,
          "source_trust_bonus": 32768,
          "redundancy_penalty": 0
        },
        "source_ref": {
          "source_id": "report_q1.pdf#page=3",
          "document_id": null,
          "page": null,
          "cell_range": null,
          "json_path": null,
          "confidence_q16": 32768
        }
      }
    ],
    "anomalies": []
  }
  ```

---

### 2.8. POST `/v1/verify?scope=<scope>[&format=json|markdown|audit]`
Verifies a specific factual claim against the available database knowledge using AQL.

* **Request Body:** Raw AQL query string.
* **Response (200 OK):** JSON by default, or stable Markdown/audit text when
  `format=markdown` or `format=audit` is supplied.
  ```json
  {
    "fact": "Solar Plant budget is 1.2B KZT",
    "status": "mixed",
    "verdict": "mixed_evidence",
    "evidence": [
      {
        "cell_id": 1,
        "matched_terms": 7,
        "source_trust_q16": 32768,
        "source_trust_category": "unknown",
        "citation": "report_q1.pdf#page=3",
        "payload_text": "scope=project:investments\nstatus=ready\n..."
      }
    ],
    "contradicting_evidence": [
      {
        "cell_id": 2,
        "matched_terms": 4,
        "source_trust_q16": 32768,
        "source_trust_category": "unknown",
        "citation": "report_q2.pdf#page=5",
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
    "numeric_conflicts": [
      {
        "metric": "budget",
        "left": "1.2B KZT",
        "right": "1.4B KZT"
      }
    ]
  }
  ```

---

### 2.9. POST `/v1/search?scope=<scope>&q=<query>`
Runs keyword, vector, ANN, or hybrid search.

* **Modes:** `keyword`, `vector`, `hybrid`, or `auto`.
* **Auto routing:** `mode=auto` selects `hybrid` when both text and vector are
  present, selected vector strategy when only vector is present, and `keyword`
  otherwise.
* **Response (200 OK):**
  ```json
  {
    "search_mode": "hybrid",
    "routing": {
      "requested_mode": "auto",
      "selected_strategy": "hybrid",
      "reason": "auto_text_and_vector_available",
      "text_available": true,
      "vector_available": true
    },
    "ann_report": null,
    "results": [
      {
        "cell_id": 1,
        "score": 32786,
        "lexical_score": 42,
        "vector_score": 100,
        "payload": "scope=project:investments\nstatus=ready\n..."
      }
    ]
  }
  ```

---

### 2.10. POST `/v1/search/explain?scope=<scope>&q=<query>`
Explains why search results ranked where they did.

* **Modes:** `keyword`, `vector`, or `hybrid`.
* **Hybrid:** pass `mode=hybrid&q=<query>&vector=<i16,...>`.
* **Response (200 OK):**
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

---

### 2.9. POST `/v1/ingest/json?scope=<scope>&source=<source_id>`
Ingests a structured JSON payload recursively flattening keys into multiple fact cells.

* **Response (200 OK):**
  ```json
  {
    "rows_ingested": 0,
    "chunks_ingested": 0,
    "facts_ingested": 10,
    "first_cell_id": 1000
  }
  ```

---

### 2.9. POST `/v1/ingest/csv?scope=<scope>&source=<source_id>`
Ingests a structured CSV table creating one document block cell per row.

* **Response (200 OK):**
  ```json
  {
    "chunks_ingested": 0,
    "facts_ingested": 0,
    "rows_ingested": 150,
    "first_cell_id": 2000,
    "job_id": 1
  }
  ```

---

### 2.10. Ingestion job lifecycle

Persisted ingestion jobs expose local progress and recovery operations:

| Route | Purpose |
| --- | --- |
| `GET /v1/ingest/jobs` | List persisted ingestion jobs. |
| `GET /v1/ingest/jobs/<job_id>` | Read one job. |
| `POST /v1/ingest/jobs/<job_id>/retry` | Move a failed job back to `queued`. |
| `POST /v1/ingest/jobs/<job_id>/cancel` | Cancel a queued or running job. |
| `DELETE /v1/ingest/jobs/<job_id>` | Delete a persisted job record. |

Job records are written atomically and include `status`, `total_items`,
`completed_items`, `failed_items`, `last_cell_id`, `message`, `retry_count`,
and `max_retries`. If CortexDB restarts while a local ingestion job is marked
`running`, `Database::open` requeues it as `queued` with a recovery message so
operators can retry or delete it instead of leaving a stale in-flight status.

---

## 3. Status Codes & Error Specification

If an error occurs, the server responds with a corresponding HTTP status code and a structured JSON body:

| Status Code | Code | Cause |
| --- | --- | --- |
| **`400 Bad Request`** | `bad_request` | Invalid parameters or malformed non-AQL input. |
| **`400 Bad Request`** | `invalid_tenant` | Tenant realm name fails charset, length, or path-safety validation. |
| **`400 Bad Request`** | `invalid_aql` | AQL parse/bind failure that is not a policy denial. |
| **`401 Unauthorized`** | `unauthorized` | Token auth required and missing or invalid. |
| **`403 Forbidden`** | `forbidden` | Non-AgentView authorization denial, including data-token access to admin/metrics routes. |
| **`403 Forbidden`** | `permission_denied` | AgentView or scope policy denied the query. |
| **`404 Not Found`** | `not_found` | Resource or route not found. |
| **`413 Payload Too Large`** | `payload_too_large` | Body size exceeds 2MB boundary. |
| **`429 Too Many Requests`** | `rate_limited` | Optional request-rate limit exceeded. |
| **`500 Internal Error`** | `storage_corruption` | Storage checksum, format, or invariant failure. |
| **`500 Internal Error`** | `internal` | Unexpected internal failure. |
| **`503 Service Unavailable`** | `database_busy` | Database actor queue or database lock is busy. |
| **`503 Service Unavailable`** | `service_unavailable` | Server component unavailable but not classified as queue/lock pressure. |

* **Error Format:**
  ```json
  {
    "code": "bad_request",
    "error": "bad_request",
    "message": "Detailed error context"
  }
  ```
