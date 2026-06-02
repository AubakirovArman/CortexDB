# CortexDB Contributor Onboarding

Status: 15-minute first-session path for new contributors.

This path is designed to give a contributor a runnable local proof without
requiring production infrastructure, hosted embeddings, cloud credentials, or
registry publishing permissions.

## 15-minute Path

1. Clone and enter the repository:

   ```bash
   git clone https://github.com/AubakirovArman/CortexDB.git
   cd CortexDB
   ```

2. Run the lightweight onboarding gate:

   ```bash
   make contributor-onboarding-check
   ```

3. Run one runnable scenario:

   ```bash
   make use-case-pack-check
   ```

4. Read the module map:

   ```text
   docs/MODULE_OWNERSHIP.md
   ```

5. Pick one small task from:

   ```text
   docs/GOOD_FIRST_ISSUES.md
   ```

## Choose A Surface

| Surface | Start here | Good first task type |
| --- | --- | --- |
| AQL | `crates/cortex-aql`, `docs/AQL_V0_4.md` | Parser golden test or diagnostic copy fix. |
| Engine | `crates/cortex-engine`, `docs/ENGINE_API.md` | Report field docs or deterministic fixture coverage. |
| ContextPack | `crates/cortex-engine/src/context`, `docs/CONTEXT_PACK.md` | Quality fixture or explain-field docs. |
| CLI | `crates/cortex-cli`, `docs/CLI.md` | Help text, JSON snapshot, or command docs. |
| Server/API | `crates/cortex-server`, `docs/API_JSON_SCHEMAS.md` | Typed response docs or OpenAPI snapshot. |
| Docs | `docs/DOCUMENTATION_INDEX.md` | Cross-link, stale wording, or claims-boundary fix. |

## Minimum Local Gates

For doc-only changes:

```bash
make contributor-onboarding-check
cargo fmt --check
```

For behavior changes:

```bash
cargo test --workspace --all-features
cargo clippy --workspace --all-targets -- -D warnings
```

For API changes:

```bash
make openapi-contract-check
```

For CLI changes:

```bash
cargo test -p cortex-cli
```

## Contribution Boundaries

Do:

- keep changes small and scoped;
- add regression tests for behavior changes;
- prefer typed structs over ad-hoc JSON;
- keep public claims aligned with `docs/PUBLIC_CLAIMS_POLICY.md`.

Do not:

- add hosted credentials or secrets;
- claim production distributed consensus, managed cloud, legal-grade
  verification, or fallback-free HNSW;
- change response schemas without OpenAPI/snapshot coverage;
- add dependencies without an explicit reason.

## First PR Shape

```text
Goal: one sentence
Scope: exact files changed
Verification: commands run
Boundary: what the change does not claim
```
