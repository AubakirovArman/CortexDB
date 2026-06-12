# Engine API Compatibility

Status: Epic 27 external embedded API compatibility gate.

This check proves that a crate outside the CortexDB workspace can depend on the
local `cortex-engine` path dependency and use the frozen crate-root facade.

## Sample Crate

```text
examples/engine_api_compat/
```

The sample crate opts out of the parent workspace with its own `[workspace]`
section. That makes it behave like an external embedded user instead of another
workspace member.

## Covered Flow

The sample runs this compatibility path:

```text
Database::open
-> put_cell
-> get_latest_cell
-> search_keyword
-> context_pack_from_aql
-> verify_fact_aql
-> checkpoint
-> close
-> Database::backup_path
-> Database::restore_from_backup
-> restored Database::open_with_options
```

## Gate

Run:

```bash
make engine-api-compat-check
make engine-api-check
```

Reports:

```text
target/engine-api-compat/report.json
target/engine-api-compat/external_sample_run.log
```

## Boundary

This gate proves source-level embedded Rust compatibility for the current local
path dependencies. It does not prove published crate compatibility, SemVer
across releases, HTTP/SDK compatibility, or C ABI stability.
