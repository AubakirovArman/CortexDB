# CortexDB API JSON Schemas Specification (v1.0.0-stable)

CortexDB uses strictly typed, serde-driven JSON schemas for all server responses, eliminating manual string formatting entirely. This document provides the formal JSON schema specification for client integrations.

---

## 1. StatsResponse Schema
Returned by `GET /v1/stats`.

```json
{
  "type": "object",
  "properties": {
    "current_seq": { "type": "integer" },
    "checkpoint_seq": { "type": "integer" },
    "live_segments": { "type": "integer" },
    "retired_segments": { "type": "integer" },
    "memtable_cells": { "type": "integer" },
    "memtable_versions": { "type": "integer" },
    "wal_size_bytes": { "type": "integer" },
    "wal_writer_records": { "type": "integer" },
    "wal_writer_bytes": { "type": "integer" },
    "wal_writer_fsyncs": { "type": "integer" },
    "wal_writer_batches": { "type": "integer" }
  },
  "required": [
    "current_seq",
    "checkpoint_seq",
    "live_segments",
    "retired_segments",
    "memtable_cells",
    "memtable_versions",
    "wal_size_bytes",
    "wal_writer_records",
    "wal_writer_bytes",
    "wal_writer_fsyncs",
    "wal_writer_batches"
  ]
}
```

---

## 2. ContextPackResponse Schema
Returned by `POST /v1/context`.

```json
{
  "type": "object",
  "properties": {
    "token_budget_tokens": { "type": "integer" },
    "estimated_tokens": { "type": "integer" },
    "truncated": { "type": "boolean" },
    "citations_required": { "type": "boolean" },
    "cells": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "cell_id": { "type": "integer" },
          "estimated_tokens": { "type": "integer" },
          "citation": { "type": ["string", "null"] },
          "payload_text": { "type": "string" },
          "explain": {
            "type": "object",
            "properties": {
              "score": { "type": "integer" },
              "matched_terms": { "type": "array", "items": { "type": "string" } },
              "why_selected": { "type": "string" },
              "base_bm25": { "type": "integer" },
              "source_trust_bonus": { "type": "integer" },
              "redundancy_penalty": { "type": "integer" }
            }
          }
        }
      }
    },
    "anomalies": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "cell_id": { "type": "integer" },
          "code": { "type": "string" },
          "message": { "type": "string" }
        }
      }
    }
  }
}
```

---

## 3. VerificationReportResponse Schema
Returned by `POST /v1/verify`.

```json
{
  "type": "object",
  "properties": {
    "fact": { "type": "string" },
    "status": { "type": "string" },
    "verdict": { "type": "string" },
    "evidence": { "type": "array" },
    "contradicting_evidence": { "type": "array" },
    "guards": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "cell_id": { "type": ["integer", "null"] },
          "code": { "type": "string" },
          "message": { "type": "string" }
        }
      }
    },
    "numeric_conflicts": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "metric": { "type": "string" },
          "left": { "type": "string" },
          "right": { "type": "string" }
        }
      }
    }
  }
}
```
