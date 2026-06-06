# Engine Determinism

Status: Epic 32 engine determinism audit.

Public engine outputs must be repeatable for the same database state, query,
agent view, and feature flags. This matters for snapshots, SDK contract tests,
release evidence, benchmark diffs, and operator trust.

## Public Ordering Rules

Search outputs:

- Rank by descending score.
- Break ties by ascending internal candidate id.
- Persisted results map candidate ids back to stable `CellId` values.

ContextPack outputs:

- Preserve AQL candidate order unless explicit feedback changes ranking.
- When feedback scores tie, preserve original candidate order.
- Emit anomalies in the order candidates are evaluated.

Verification outputs:

- Rank evidence by descending matched terms.
- Then rank by descending source trust.
- Break ties by ascending `CellId`.
- Conflict index output is sorted by fact, cell id, then relation cell id.

## Collection Rule

Public output paths must use deterministic collections such as `BTreeMap` and
`BTreeSet` when map/set iteration can affect output order.

Do not use `HashMap` or `HashSet` in these output paths:

- `crates/cortex-engine/src/search.rs`
- `crates/cortex-engine/src/search/database.rs`
- `crates/cortex-engine/src/search/persisted.rs`
- `crates/cortex-engine/src/context`
- `crates/cortex-engine/src/verification.rs`
- `crates/cortex-engine/src/verification`
- `crates/cortex-server/src/responses.rs`
- `crates/cortex-cli/src/cli_json.rs`
- `crates/cortex-cli/src/cli_json_types.rs`

## Enforcement

Required gate:

```bash
make engine-determinism-check
```

The gate checks:

- deterministic output docs are present;
- regression tests cover search, ContextPack, and VerificationReport output;
- public output paths do not introduce `HashMap` or `HashSet`.

Behavioral regression tests live in:

```text
crates/cortex-engine/tests/determinism.rs
```

The tests run repeated public calls before and after checkpoint and compare
their canonical snapshots.
