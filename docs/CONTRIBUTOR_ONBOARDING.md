# CortexDB Contributor Onboarding

Status: 15-minute first-session path for new contributors.

This path is designed to give a contributor a runnable local proof without
requiring production infrastructure, hosted embeddings, cloud credentials, or
registry publishing permissions.

## Epic 139 Contributor Onboarding v2 Contract

This page closes the contributor-onboarding epic by giving a new contributor
four concrete entry points:

| Epic task | Evidence |
| --- | --- |
| Add module map | The `Module Map` section points to crates, ownership docs, and first files to read. |
| Add good first issues | `docs/GOOD_FIRST_ISSUES.md` lists bounded starter tasks with owner files and gates. |
| Add test commands | `Minimum Local Gates` separates doc-only, behavior, API, and CLI checks. |
| Add issue templates | `.github/ISSUE_TEMPLATE/` includes bug, feature, design, and good-first templates with scope and success criteria. |

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

## Module Map

| Area | Primary path | Ownership / contract doc | First file to read |
| --- | --- | --- | --- |
| AQL compiler | `crates/cortex-aql` | `docs/AQL_V0_4.md` | `crates/cortex-aql/src/lib.rs` |
| Core data model | `crates/cortex-core` | `docs/CORE_ENGINE.md` | `crates/cortex-core/src/lib.rs` |
| Storage formats | `crates/cortex-storage` | `docs/STORAGE_FORMATS.md` | `crates/cortex-storage/src/lib.rs` |
| Engine facade | `crates/cortex-engine` | `docs/ENGINE_API.md` | `crates/cortex-engine/src/lib.rs` |
| CLI | `crates/cortex-cli` | `docs/CLI.md` | `crates/cortex-cli/src/main.rs` |
| HTTP server | `crates/cortex-server` | `docs/API.md` | `crates/cortex-server/src/lib.rs` |
| SDK | `crates/cortex-sdk` | `docs/SDK_QUICKSTART.md` | `crates/cortex-sdk/src/lib.rs` |

For ownership boundaries across modules, use:

```text
docs/MODULE_OWNERSHIP.md
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

## Issue Templates

Use these templates to keep first contributions bounded:

- `.github/ISSUE_TEMPLATE/bug_report.md` for reproducible defects;
- `.github/ISSUE_TEMPLATE/feature_request.md` for scoped user-facing changes;
- `.github/ISSUE_TEMPLATE/design_task.md` for architecture or format decisions;
- `.github/ISSUE_TEMPLATE/good_first_issue.md` for starter tasks with explicit
  gates.

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
