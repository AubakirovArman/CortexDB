# C20 Baseline Comparison

This is an honest local comparison against a small naive stack: SQLite FTS5, deterministic exact hashed vectors, and hybrid RRF. It also lists CortexDB feature evidence that the naive stack does not provide by itself.

## Command

```bash
make baseline-comparison-check
```

## Retrieval And Latency

| Domain | SQLite FTS5 recall | Vector recall | Hybrid recall | CortexDB gate recall | Hybrid p95 ms | CortexDB p95 ms |
|---|---:|---:|---:|---:|---:|---:|
| investment_projects | 92.50% | 75.00% | 87.50% | 95.00% | 1.440 | 0.117 |
| legal_policies | 100.00% | 100.00% | 100.00% | 100.00% | 0.172 | 0.015 |
| support_tickets | 100.00% | 100.00% | 100.00% | 100.00% | 0.211 | 0.014 |
| technical_docs | 100.00% | 100.00% | 100.00% | 100.00% | 0.189 | 0.015 |

## Feature Matrix

| Feature | Naive stack | CortexDB | Evidence |
|---|---|---|---|
| Top-k retrieval quality | Measured on the same four real-domain corpora with SQLite FTS5, deterministic exact vector search, and hybrid RRF. | Measured by the CortexDB retrieval-quality beta gate on the same four real-domain corpora. | target/retrieval-quality/beta-report.json |
| Retrieval latency | Measured per query in the baseline runner. | Measured by the CortexDB retrieval-quality beta gate. | target/retrieval-quality/beta-report.json |
| Agent permissions | Not built in; every app must add its own ACL filter before or after retrieval. | Built into AgentView, PolicyRewrite, and permission-aware index pruning. | B04, B16, C09, E09 gates |
| Token budget and ContextPack | Not built in; app code must trim and order chunks. | ContextPack gate checks evidence coverage, deterministic order, token reduction, and redundancy suppression. | target/context-pack-quality/v3-report.json |
| Citations and provenance | Not built in; app code must preserve chunk ids and source refs. | Typed provenance and ContextPack citation pressure are first-class gates. | B06, B14, target/context-pack-quality/v3-report.json |
| VerifyFact and conflict visibility | Not built in; app code must run a separate verifier/conflict store. | VerifyOp, numeric conflict index, temporal guard, and context conflict visibility gates. | B07, B08, B09, C13, C14 |

## CortexDB ContextPack Evidence

- status: `passed`
- external datasets: `4`
- cases: `105`
- evidence coverage: `100.00%`
- citation coverage: `100.00%`
- token reduction: `56.65%`

## Boundary

- This report does not claim CortexDB always beats a naive retriever on raw recall.
- The dense baseline is deterministic exact hashed-vector search so the gate stays dependency-free; it is the CI-safe stand-in for a FAISS sidecar.
- The CortexDB differentiation shown here is retrieval plus built-in governance: permissions, token budgets, citations/provenance, and VerifyFact/conflict gates.
