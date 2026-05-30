# CortexDB Demo Output Reference

This document shows the actual CLI output you should see when running the
`examples/demo/investment_projects/run.sh` demo script.

It demonstrates the key CortexDB differentiators over a plain RAG pipeline:

- **Permission-safe scope filtering** (private cells are excluded)
- **ContextPack** with token budgets, citations, and explain metadata
- **VERIFY FACT** with numeric conflict detection

---

## Step 1 — Load data

```bash
$ cargo run -q -p cortex-cli -- put ./db 1 "scope=project:investments
status=ready
type=fact
source=report_q1.pdf#page=3
project=Solar Plant
metric=budget
value=1200000000
currency=KZT

Solar Plant budget is 1.2B KZT in Q1."
```

Repeat for cells 2 (Q2 budget = 1.4B KZT), 3 (private risk note), and 4 (board minutes).

---

## Step 2 — Basic search

```bash
$ cargo run -q -p cortex-cli -- search ./db project:investments Solar
```

Output:
```text
cell_id=1 score=1488896 lexical_score=1488896 vector_score=0 payload=...
cell_id=2 score=1488896 lexical_score=1488896 vector_score=0 payload=...
cell_id=4 score=1352704 lexical_score=1352704 vector_score=0 payload=...
```

> **Note:** Cell 3 (`scope=private`) is excluded because the search is scoped to
> `project:investments`.

---

## Step 3 — AQL retrieve

```bash
$ cargo run -q -p cortex-cli -- context ./db project:investments \
  'RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN default LIMIT 10 CANDIDATES;'
```

Output:
```text
cells=3 estimated_tokens=163 token_budget=1000 truncated=false anomalies=0
```

Three cells returned, zero anomalies, token budget respected.

---

## Step 4 — ContextPack JSON

```bash
$ cargo run -q -p cortex-cli -- context ./db project:investments \
  'RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN default LIMIT 10 CANDIDATES;' --json
```

Output (abbreviated):
```json
{
  "token_budget_tokens": 1000,
  "estimated_tokens": 163,
  "truncated": false,
  "citations_required": false,
  "cells": [
    {
      "cell_id": 1,
      "estimated_tokens": 64,
      "citation": "report_q1.pdf#page=3",
      "payload_text": "Solar Plant budget is 1.2B KZT in Q1.",
      "explain": {
        "score": 72768,
        "matched_terms": ["is", "solar", "plant", "budget"],
        "base_bm25": 40000,
        "source_trust_bonus": 32768,
        "redundancy_penalty": 0
      },
      "source_ref": {
        "source_id": "report_q1.pdf#page=3",
        "confidence_q16": 32768
      }
    }
  ],
  "anomalies": []
}
```

> **Key fields:** `token_budget_tokens`, `estimated_tokens`, `truncated`, `citations`,
> `explain`, `source_ref`, `anomalies`.

---

## Step 5 — Verify Fact

```bash
$ cargo run -q -p cortex-cli -- verify ./db project:investments \
  'VERIFY FACT "Solar Plant budget is 1.2B KZT" IN BRAIN default;'
```

Output:
```text
status=mixed evidence=2 contradictions=1 fact=Solar Plant budget is 1.2B KZT
contradiction_cell_id=2 matched_terms=4 source_trust_q16=32768
guard=numeric_mismatch cell_id=2 message=payload numeric claim differs from fact numeric claim
```

> CortexDB detected that Q1 says **1.2B KZT** while Q2 says **1.4B KZT**.

---

## Step 6 — Verify Fact JSON

```bash
$ cargo run -q -p cortex-cli -- verify ./db project:investments \
  'VERIFY FACT "Solar Plant budget is 1.2B KZT" IN BRAIN default;' --json
```

Output:
```json
{
  "verdict": "mixed_evidence",
  "supporting": [
    {
      "cell_id": 1,
      "citation": "report_q1.pdf#page=3",
      "matched_terms": 7,
      "payload_text": "Solar Plant budget is 1.2B KZT in Q1.",
      "source_trust_q16": 32768
    }
  ],
  "contradicting": [
    {
      "cell_id": 2,
      "citation": "report_q2.pdf#page=5",
      "matched_terms": 4,
      "payload_text": "Solar Plant budget is 1.4B KZT in Q2.",
      "source_trust_q16": 32768
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

## Comparison: Classic RAG vs CortexDB

| Feature | Classic RAG | CortexDB |
|---------|-------------|----------|
| Returns | Raw top-k chunks | **ContextPack** with token budgets |
| Conflicts | May hide contradictions | **VERIFY FACT** detects numeric conflicts |
| Permissions | No scope isolation | **AgentView** enforces readable scopes |
| Citations | Unstructured | **SourceRef** with confidence scores |
| Anomalies | None | **Anomaly report** per pack |
| Query language | Simple similarity | **AQL** with policy clauses |

---

## Run the full demo

```bash
make demo
# or
./examples/demo/investment_projects/run.sh
```
