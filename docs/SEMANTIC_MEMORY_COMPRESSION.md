# Semantic Memory Compression

`EPIC-F06` defines semantic compression as an opt-in engine commit contract for
summaries produced by an external worker. CortexDB validates and stores the
compressed memory, but it does not call an LLM or embed model inside the engine.

## Contract

- Compression is disabled by default and requires
  `DatabaseOptions.semantic_compression.enabled = true` or
  `CORTEXDB_SEMANTIC_COMPRESSION=1`.
- The summary is a normal `type=memory` cell in the requested writable scope.
- The external worker must provide the summary payload, worker id, answerability
  score, source cell references, and optional idempotency key.
- Every source cell must exist and be readable by the supplied `AgentView`.
- The summary payload must carry audit metadata:
  - `compression_kind=semantic_summary`
  - `compression_source_cells=<cell ids>`
  - `compression_answerability_q16=<score>`
  - `compression_worker=<external worker id>`
- The configured `min_answerability_q16` threshold rejects low-quality
  summaries before commit.

## Loss Boundaries

The engine accepts semantic compression only as a lossy summary with preserved
auditability:

- The original source cells remain the source of truth.
- Summary text may omit details, but it must not erase provenance links to the
  source cells used to produce it.
- Answerability metadata is treated as a quality gate, not a correctness proof.
- Retrieval can include the compressed cell directly, and clients can unfold
  provenance by reading `compression_source_cells`.
- The external worker is responsible for LLM prompt safety, hallucination
  controls, and summary generation. CortexDB only validates the commit boundary.

## Engine Surface

`Database::commit_semantic_memory_compression` validates the request and commits
the summary cell through the normal WAL-backed write path. The returned
`SemanticCompressionReport` records the committed sequence, source cell ids,
source reference count, answerability, worker id, and whether the stored summary
is auditable.

The runtime flag is intentionally separate from learned ranking and agent
transactions. Semantic compression is research/prototype scope and disabled in
default database options.

## Acceptance Check

Run:

```bash
make semantic-memory-compression-check
```

The command writes:

```text
target/semantic-memory-compression/report.json
```

The report records contract marker checks and the F06 regression test command.
