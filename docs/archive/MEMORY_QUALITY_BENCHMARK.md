# Memory Quality Benchmark

Status: Epic 142 local deterministic benchmark gate.

This benchmark covers the first memory-quality behaviors that must stay stable
before CortexDB can treat agent memory as more than durable storage.

## Benchmark Contract

| Plan task | Benchmark case | Evidence |
| --- | --- | --- |
| Benchmark update handling | Latest memory payload replaces the visible older memory version for the same cell. | `memory_quality_update_handling_prefers_latest_payload` |
| Benchmark stale memory detection | Expired memory is detected, scored, tombstoned, and excluded from retrieval. | `memory_quality_stale_memory_detection_expires_and_scores_memory` |
| Benchmark preference retrieval | Positive feedback moves the preferred memory to the front of ContextPack selection. | `memory_quality_preference_retrieval_uses_feedback_signal` |
| Benchmark temporal changes | Old read transactions keep old memory while current reads see the updated memory. | `memory_quality_temporal_changes_preserve_snapshot_visibility` |

## What This Proves

- Memory updates are MVCC-safe and do not leak stale payload into latest
  retrieval.
- TTL and decay behavior is fixed-point and deterministic.
- Feedback is a query-time preference signal, not just stored metadata.
- Temporal changes preserve snapshot isolation.

## Boundary

This is a local deterministic benchmark. It does not measure natural-language
memory synthesis, learned preference models, or long-running real user memory
quality. Those remain future benchmark layers.

## Gate

Run:

```bash
make memory-quality-benchmark-check
```

The gate runs the Rust benchmark test and verifies that this document and the
production epic plan still mention the four required memory-quality tasks.
