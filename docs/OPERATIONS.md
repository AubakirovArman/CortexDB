# Operations Guide

## 1) Local operation model

CortexDB is currently optimized for single-node Core Alpha operation.
Use one runtime process per database root.

- `cortex-server` provides HTTP access (`/v1/*`).
- `cortex-cli` provides local one-shot operations.
- backups, metrics, recovery scripts, and release gates are in `Makefile`.

## 2) Start server

```bash
CORTEXDB_AUTH_TOKEN=dev-token \
cargo run -p cortex-server -- ./data 127.0.0.1:8181
```

Health check:

```bash
curl http://127.0.0.1:8181/v1/health
```

## 3) Core sanity checks

```bash
cargo run -p cortex-cli -- validate ./data
cargo run -p cortex-cli -- stats ./data
cargo run -p cortex-cli -- wal-validate ./data
cargo run -p cortex-cli -- manifest-validate ./data
cargo run -p cortex-cli -- ann-validate ./data
```

Optional typed checks and smoke paths:

```bash
make openapi-contract-check
make sdk-contract-check
make sdk-smoke-test
make dashboard-smoke
```

## 4) Backup and recovery

```bash
cargo run -p cortex-cli -- backup ./data ./backups/data-$(date -u +%Y%m%dT%H%M%SZ)
cargo run -p cortex-cli -- backup-prune ./backups cortexdb- 5
cargo run -p cortex-cli -- restore ./backups/data-... ./data-restored
cargo run -p cortex-cli -- validate ./data-restored
```

Offsite staging:

```bash
cargo run -p cortex-cli -- backup-offsite-stage ./backups/data.tar.gz ./offsite cortexdb-$(date -u +%Y%m%dT%H%M%SZ)
```

## 5) Recovery and incident runbooks

- If startup reports lock conflict, inspect stale lock file and use:

```bash
cargo run -p cortex-cli -- unlock ./data --force
```

- If DB fails to start after crash, rerun validation first:

```bash
cargo run -p cortex-cli -- validate ./data
```

- For WAL corruption workflows:

```bash
cargo run -p cortex-cli -- wal-dump ./data
cargo run -p cortex-cli -- wal-truncate ./data
```

Then reopen and validate.

## 6) Performance/reliability smoke

- CLI/HTTP smoke: `scripts/smoke_test.sh`
- ANN/recall drift: `make ann-history-regression-check`, `make ann-drift-check`
- Recovery/fault: `make crash-fault-check`, `make chaos-restart-check`

## 7) Known operational limits

- Single-node model first.
- Production multi-node is experimental.
- HNSW is guarded; exact vector path remains the correctness fallback.
- For distributed security/compliance needs, wait for dedicated production hardening milestone.
