# Good First Issues

Status: starter task map for contributors.

Use this page to pick a small, reviewable issue. Each item is intended to fit in
one focused PR and should include evidence from the listed gate.

## Starter Map

| Area | Example task | Files | Gate |
| --- | --- | --- | --- |
| AQL diagnostics | Add a golden parser diagnostic case for one invalid query. | `crates/cortex-aql/tests`, `docs/AQL_V0_4.md` | `cargo test -p cortex-aql` |
| ContextPack docs | Add one documented example for anomalies or citations. | `docs/CONTEXT_PACK.md`, `examples/eval/context_pack_quality.jsonl` | `make context-pack-quality-check` |
| Verification fixtures | Add a deterministic support/legal/technical verification case. | `examples/eval/verification_cases.jsonl` | `make verification-quality-check` |
| Use-case packs | Add a small scoped scenario with fixture and smoke command. | `examples/use_cases`, `docs/USE_CASE_PACKS.md` | `make use-case-pack-check` |
| CLI docs | Clarify a command example and update the matching smoke check if needed. | `docs/CLI.md`, `crates/cortex-cli` | `cargo test -p cortex-cli` |
| API docs | Add schema text for an existing response field. | `docs/API_JSON_SCHEMAS.md`, `docs/openapi.yaml` | `make openapi-contract-check` |
| Dashboard docs | Document one existing dashboard panel or marker. | `docs/DASHBOARD_UI.md`, `crates/cortex-server/src/dashboard` | `make dashboard-product-check` |
| Operations docs | Add one troubleshooting row with a command and expected result. | `docs/OPERATIONS.md` | `make operations-runbook-check` |

## Labels To Use

Suggested labels for GitHub issues:

- `good first issue`
- `docs`
- `tests`
- `aql`
- `contextpack`
- `cli`
- `api`
- `operations`

## Review Checklist

Before opening a PR:

1. Confirm the task has one clear owner surface.
2. Run the gate listed in the table.
3. Run `cargo fmt --check`.
4. Avoid public-claims drift.
5. Include `done / remaining / next / risks` in the PR summary.
