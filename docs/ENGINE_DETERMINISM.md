# Engine Determinism

Status: Epic 32 engine determinism audit.

Public engine outputs must be repeatable for the same database state, query,
agent view, and feature flags. This matters for snapshots, SDK contract tests,
release evidence, benchmark diffs, operator trust, and the accountability
receipt roadmap.

## Public Ordering Rules

Search outputs:

- Rank by descending score.
- Break ties by ascending internal candidate id.
- Persisted results map candidate ids back to stable `CellId` values.

ContextPack outputs:

- Preserve AQL candidate order unless explicit feedback changes ranking.
- When feedback scores tie, preserve original candidate order.
- Emit anomalies in the order candidates are evaluated.
- Canonical accountability bytes are emitted through
  `cortex_engine::canonical::canonical_context_pack_bytes`.

Verification outputs:

- Rank evidence by descending matched terms.
- Then rank by descending source trust.
- Break ties by ascending `CellId`.
- Conflict index output is sorted by fact, cell id, then relation cell id.
- Canonical accountability bytes are emitted through
  `cortex_engine::canonical::canonical_verification_report_bytes`.

## Canonical Accountability Surface

Phase 0 of the accountability roadmap introduces one canonical byte surface for
future roots and verifier work:

- `ContextPack` uses `context_pack.canonical.v1`;
- `VerificationReport` uses `verification_report.canonical.v1`;
- object keys are recursively sorted before writing bytes;
- current hashed-field allowlists live in `crates/cortex-engine/src/canonical.rs`;
- top-level canonical structs must classify new fields as hashed,
  exported-only, or telemetry before the gate passes;
- cross-process fixture runs must produce identical canonical byte hex;
- telemetry fields such as `elapsed_nanos`, `total_elapsed_nanos`, `Instant`,
  and `SystemTime` are explicitly excluded from the canonical surface.

Required gate:

```bash
make canonical-serialization-check
```

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
their canonical snapshots. The accountability canonical-byte tests live in
`crates/cortex-engine/src/canonical.rs`.
