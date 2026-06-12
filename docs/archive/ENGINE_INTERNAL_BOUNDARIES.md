# Engine Internal Boundaries

Status: Epic 31 internal boundary audit.

The compatibility boundary for downstream crates is the `cortex_engine` crate
root. Server, SDKs, and external clients should import root facade symbols such
as `Database`, `EngineError`, `CellMetadata`, `VerificationReport`, and
`StorageStats`.

## Rule

Do not depend on implementation module paths from server or SDK code:

```text
cortex_engine::Database
-cortex_engine::database::Database
```

```text
cortex_engine::VerificationReport
-cortex_engine::verification::VerificationReport
```

This keeps refactors inside `cortex-engine/src/*` possible without breaking
server/API/SDK contracts.

## Current Boundary

The following crates/directories are checked by `make engine-internal-boundary-check`:

| Path | Rule |
| --- | --- |
| `crates/cortex-server` | Must use `cortex_engine` crate-root facade. |
| `crates/cortex-sdk` | Must not import `cortex_engine` implementation modules. |
| `sdk` | Must not reference Rust implementation modules in generated/published SDK artifacts. |

`cortex-cli` is operator tooling inside the repository. It should prefer the
crate-root facade, but the strict Epic 31 gate is focused on server and SDK
contract consumers.

## Allowed Internal Usage

Internal module paths are allowed inside:

- `crates/cortex-engine`
- engine tests and examples that specifically exercise internals;
- implementation scripts that inspect source layout rather than consume runtime
  APIs.

## Enforcement

`scripts/engine_internal_boundary_check.py` parses the top-level module names
from `crates/cortex-engine/src/lib.rs` and fails if checked server/SDK paths use
`cortex_engine::<module>::...`.

Required gate:

```bash
make engine-internal-boundary-check
```

The gate proves that server and SDK code do not currently depend on known
top-level engine implementation modules. It does not prove Rust visibility has
been fully tightened; reducing `pub mod` exposure remains future cleanup where
tests/tooling allow it.
