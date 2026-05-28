# CortexDB Engineering Agent Instructions

## Core identity
- Project: CortexDB, Rust workspace.
- Scope: single-node durable context DB with AQL + MemTable + WAL + checkpoint/compact + typed APIs.
- Work mode: spec-driven, minimal-change, test-first when behavior changes.

## Prompt / task protocol
For every non-trivial task, include:
1) Goal: one sentence
2) Scope: explicit file list (`#file ...`)
3) Constraints (Do/Don’t)
4) Success criteria (commands)
5) Response format: only `done / remaining / next / risks`

## Engineering rules
- No new dependencies unless explicitly requested.
- Prefer preserving public contracts and behavior unless migration is requested.
- Any response format/schema change must include tests and OpenAPI/snapshot checks if relevant.
- Avoid verbose internal reasoning in user-facing output.

## Mandatory verification (minimum)
- `cargo fmt --check`
- `cargo test --workspace --all-features`
- `cargo clippy --workspace --all-targets -- -D warnings`
- If API touched: `make openapi-contract-check`
- If CLI touched: relevant `cargo test -p cortex-cli` (full workspace preferred)

## Safety and reliability conventions
- Keep file-level edits bounded and focused.
- Use typed structs over ad-hoc JSON maps.
- Add regression tests for bugfixes and edge cases.
- Do not use `unwrap`, avoid panics in production paths unless intentional and justified.

## Completion style
- Always finish with:
  - `done`: concrete result
  - `remaining`: what still open
  - `next`: immediate next action
  - `risks`: explicit risks/tradeoffs

