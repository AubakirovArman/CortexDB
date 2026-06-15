# Learned Ranking Calibration

`EPIC-F05` accepts learned/calibrated ranking only as an offline, opt-in
calibration path. Default CortexDB retrieval remains deterministic and
domain-neutral.

## Contract

- The engine must not train online or call an LLM/neural model for ranking.
- Calibration data is split into `train` and `heldout` rows before weights are
  selected.
- `question_id` and `document_id` values must not overlap between train and
  heldout splits.
- Heldout evaluation compares learned linear weights against the deterministic
  baseline profile.
- A learned profile is accepted only if heldout MRR improves and no heldout row
  regresses.
- Runtime use is disabled by default and requires `CORTEXDB_LEARNED_RANKING=1`
  or `DatabaseOptions.learned_ranking.enabled = true`.

## Fixture

The checked-in fixture is intentionally small:

```text
fixtures/enterprise_rag_bench/learned_ranking/offline_v1.jsonl
```

Each row contains:

- `split`: `train` or `heldout`
- `question_id`
- `question_type`
- `expected_doc_ids`
- `candidates[]`: `document_id`, `base_score`, `lexical_score`, `vector_score`

The fixture models the calibration boundary. It is not an official
EnterpriseRAG answer score and does not replace larger retrieval or answer
benchmarks.

## Runtime Flag

The default Database `HybridRerank` path uses balanced RRF and fixed reranker
weights. When learned ranking is enabled, Database `HybridRerank` switches to
the calibrated RRF/reranker profile. Lower-level `SearchIndexes` defaults remain
fixed unless the caller explicitly supplies hybrid RRF weights.

## Acceptance Check

Run:

```bash
make learned-ranking-calibration-check
```

The command writes:

```text
target/learned-ranking/calibration/report.json
```

The report includes trained linear profiles, heldout baseline MRR, heldout
learned MRR, lift, win rate, and regression rows.
