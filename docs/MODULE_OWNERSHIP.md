# Module Ownership

The stable embedded API is the crate-root `cortex_engine` / `cortex-engine`
facade documented in
`docs/ENGINE_API.md`.

## Stable facade

Implementation modules may remain visible for workspace tests and internal
tooling, but they are not the compatibility boundary unless their types are
listed in `fixtures/engine/public_api_freeze_v1.json`.

## Internal modules

Private helper modules checked by the public API freeze gate include `cleanup`,
`config`, `database_files`, `lock`, and `options`.
