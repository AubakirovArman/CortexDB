# Engine API Evidence

Last local engine API run: 2026-05-31.

Run:

```bash
make engine-api-check
```

Primary artifacts:

```text
target/engine-api/report.json
target/engine-api/*.log
```

Latest local status: passed.

## Matrix

| Suite | Purpose |
| --- | --- |
| public API compile | Compiles and runs the stable `cortex-engine` facade test. |
| engine doctests | Compiles public documentation examples. |
| engine docs build | Builds `cortex-engine` rustdoc without dependencies. |

## Boundary

The local gate proves:

- stable embedded engine API docs exist;
- public API compile test passes;
- engine doctests compile;
- engine rustdoc builds.

The gate does not prove:

- every internal module is stable;
- no future breaking changes;
- C ABI or non-Rust embedded API stability.
