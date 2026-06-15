# LLM Context Value Optimization

`EPIC-F07` adds an opt-in ContextPack planning mode that ranks candidates by
expected answerability value per token. The goal is to spend a tight LLM context
budget on cells that add the most useful query coverage, not merely the first
or highest raw retrieval score.

## Contract

- The optimizer is disabled by default for compatibility.
- Callers enable it with `ContextPackOptions.optimize_value_per_token = true`.
- The optimizer runs after AQL retrieval, policy rewrite, AgentView filtering,
  and candidate limiting. It cannot introduce hidden cells or bypass
  permissions.
- The optimizer reorders only the already retrieved candidate set before
  ContextPack budget packing.
- It is deterministic and does not call an LLM, tokenizer API, embedding model,
  or benchmark judge.

## Cost Model Inputs

The value-per-token rank uses local, auditable inputs:

- marginal query-term coverage not already selected;
- total matched query-term coverage;
- canonical ContextPack BM25 score;
- source trust bonus;
- source freshness bonus;
- citation availability when citations are required;
- decayed feedback bonus;
- redundancy penalty against already planned cells;
- deterministic token cost using the configured `ContextTokenProfile` and
  citation overhead.

The planner scores each remaining candidate greedily, adds the best
value-per-token cell to the planned order, then recomputes marginal coverage and
redundancy for the next candidate.

## Budget Behavior

The existing budget packer still enforces token limits, large-cell policy,
span-level packing, citation anomalies, redundancy anomalies, answerability
scoring, and conflict visibility. Value-per-token planning changes candidate
order only; final packed cells still pass through the same ContextPack
selection and audit path.

This makes the feature safe for research/prototype use: a caller can compare
legacy candidate order against value-per-token order under the same budget,
without changing AQL or retrieval semantics.

## Acceptance Check

Run:

```bash
make context-pack-value-per-token-check
```

The command writes:

```text
target/context-pack-quality/value-per-token-report.json
```

The report records contract marker checks and the focused ContextPack
regression test that proves better budget allocation on a tight-budget fixture.
