# CortexDB HTTP API Contract Specification (v0.1.0-core-alpha candidate)

CortexDB exposes a lightweight, ultra-high-performance HTTP JSON API for interacting with Agent Memory, Retrieving Context, and Verifying Facts.

---

## 1. Global Server Information

* **Default Base URL:** `http://127.0.0.1:8181`
* **Content-Type:** `application/json`
* **Max Payload Boundary:** 2MB (Requests exceeding 2MB will return `413 Payload Too Large`)
* **OpenAPI contract:** [`openapi.yaml`](openapi.yaml)
* **Tenant routing:** database endpoints accept optional `tenant=<realm>`. Omit
  it or use `tenant=default` for the root database; any other value routes to a
  per-tenant realm under the server data directory.

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

### 2.6. POST `/v1/context?scope=<scope>`
Executes an AQL query and compiles a budgeted, deduplicated, and scored `ContextPack`.

* **Request Body:** Raw AQL query string.
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

### 2.7. POST `/v1/verify?scope=<scope>`
Verifies a specific factual claim against the available database knowledge using AQL.

* **Request Body:** Raw AQL query string.
* **Response (200 OK):**
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
        "citation": "report_q1.pdf#page=3",
        "payload_text": "scope=project:investments\nstatus=ready\n..."
      }
    ],
    "contradicting_evidence": [
      {
        "cell_id": 2,
        "matched_terms": 4,
        "source_trust_q16": 32768,
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

### 2.8. POST `/v1/ingest/json?scope=<scope>&source=<source_id>`
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
    "first_cell_id": 2000
  }
  ```

---

## 3. Status Codes & Error Specification

If an error occurs, the server responds with a corresponding HTTP status code and a structured JSON body:

| Status Code | Code | Cause |
| --- | --- | --- |
| **`400 Bad Request`** | `bad_request` | Invalid parameters, AQL parsing syntax failures. |
| **`401 Unauthorized`** | `unauthorized` | Token auth required and missing or invalid. |
| **`404 Not Found`** | `not_found` | Resource or route not found. |
| **`413 Payload Too Large`** | `payload_too_large` | Body size exceeds 2MB boundary. |
| **`500 Internal Error`** | `internal_error` | Storage engine IO failures or OS lock issues. |

* **Error Format:**
  ```json
  {
    "error": "bad_request",
    "message": "Detailed error context"
  }
  ```
