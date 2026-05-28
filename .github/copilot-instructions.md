# CortexDB Copilot Instructions

## Project context
- Workspace path: `/mnt/hf_model_weights/arman/3bit/sites/CortexDB`
- Crates: `cortex-aql`, `cortex-core`, `cortex-storage`, `cortex-engine`, `cortex-server`, `cortex-cli`, `cortex-sdk`.
- Current status: Core Alpha is stable and in active hardening.

## Preferred style for AI-assisted work
- Prefer short, deterministic outputs.
- Always include file references explicitly (`#file path`) in prompts and responses.
- Keep responses in this order:
  1) summary (1–2 lines), 2) concrete changes, 3) verification commands/results, 4) risks.
- Use strict constraints (whitelist/blacklist) in every edit request.
- Keep functions and file sizes pragmatic; avoid broad refactors unless explicitly requested.

## Engineering constraints
- Do not add new crates/dependencies unless required for correctness.
- Do not use floating-point math for scoring/quality unless explicitly requested.
- Keep public API changes minimal and explicitly documented.
- Preserve existing behavior when patching; add tests for behavior changes.
- For any format/schema changes, update:
  - typed response structs
  - server route contracts
  - OpenAPI sync checks
  - snapshot/API contract tests

## Mandatory validation for each change batch
Run at minimum:
- `cargo fmt --check`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets -- -D warnings`
- if API/server touched: `make openapi-contract-check`
- if CLI touched: `cargo test -p cortex-cli --features ???` (or full workspace test already)

## Task protocol
- Do not change lock-invariants accidentally: first inspect current behavior + tests, then patch minimally, then run validation.
- For technical debt cleanup: prefer dedicated regression tests over comments.
- Before suggesting a design change, provide a short risk/impact paragraph + migration note.

## Prompt template (for cache friendliness)
- Provide short, stable preface each time:
  - `Role`: what to change
  - `Scope`: list of #file paths
  - `Constraints`: explicit do/don't list
  - `Success criteria`: exact commands/tests
- Avoid pasting huge chunks of source code in prompts unless necessary.
- Use consistent phrasing for repeated command-oriented requests to improve cache reuse.
