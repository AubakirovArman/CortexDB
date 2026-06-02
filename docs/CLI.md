# CLI Reference

The `cortexdb` binary is a local command-line tool that operates directly on the database files. It does **not** require a running server.

## Global Flags

| Flag | Description |
|------|-------------|
| `--json` | Print machine-readable JSON when supported. Applies to `stats`, `validate`, `ann-validate`, `audit`, `context`, `verify`, and `search-vector-eval`. |
| `--help` | Show help for any subcommand. |
| `--version` | Print version. |

## Commands

### Quickstart

```bash
cortexdb put ./db 1 "hello world"
cortexdb get ./db 1
cortexdb stats ./db --json
```

### Database Operations

#### `put <path> <cell_id> <payload>`
Store a cell directly.

```bash
cortexdb put ./db 42 "scope=project:investments\n\nSolar Plant budget 1.2B KZT."
```

#### `get <path> <cell_id>`
Retrieve a cell by ID.

```bash
cortexdb get ./db 42
```

#### `tombstone <path> <cell_id>`
Mark a cell as deleted (soft delete).

```bash
cortexdb tombstone ./db 42
```

#### `flush <path>`
Write memtable to disk (checkpoint).

```bash
cortexdb flush ./db
```

#### `compact <path>`
Merge segments and reclaim space.

```bash
cortexdb compact ./db
```

### Health & Diagnostics

#### `doctor <path>`
Run a health check: open, stats, validate, ANN metrics.

```bash
cortexdb doctor ./db
```

#### `stats <path> [--json]`
Show storage statistics.

```bash
cortexdb stats ./db --json
```

#### `validate <path> [--json]`
Run integrity checks.

```bash
cortexdb validate ./db
```

#### `repair [--dry-run] <path>`
Attempt to repair storage inconsistencies.

```bash
cortexdb repair --dry-run ./db
cortexdb repair ./db
```

`--dry-run` reports orphan temp files and WAL truncation need without mutating
the database. The apply form removes known temp files and truncates only to the
best-effort WAL safe offset.

#### `audit <audit_jsonl_path>`
Review a persisted server audit JSONL file. The command supports route, status,
action, and tenant filters plus an automated redaction check.

```bash
cortexdb audit ./audit/http.jsonl --summary --redaction-check
cortexdb audit verify ./audit/http.jsonl
cortexdb audit ./audit/http.jsonl --route /v1/cell --status 403
cortexdb audit ./audit/http.jsonl --action write --tenant-filter tenant-alpha
cortexdb --json audit ./audit/http.jsonl --summary --redaction-check
```

#### `backup <path> <backup_path>`
Create a validated offline-copy backup. The command opens the source database,
holds the source lock, flushes the WAL writer, validates storage, copies stable
files, and excludes `db.lock` plus known temporary files.

```bash
cortexdb backup ./db ./db.backup
```

#### `backup-encrypted <path> <archive_path>`
Create a passphrase-protected single-file backup archive. Prefer
`--passphrase-env` or `CORTEXDB_BACKUP_PASSPHRASE` instead of typing secrets
directly into shell history.

```bash
export CORTEXDB_BACKUP_PASSPHRASE="choose-a-long-local-passphrase"
cortexdb backup-encrypted ./db ./db.cdbenc --passphrase-env CORTEXDB_BACKUP_PASSPHRASE
```

#### `backup-drill <path> <backup_path> <restore_path>`
Create a backup, restore it into a new drill target, and validate the restored
database. Use this before trusting a backup procedure, because it proves the
copy can be opened and replayed.

```bash
cortexdb backup-drill ./db ./db.backup ./db.drill-restored
```

#### `backup-prune <backup_root> <prefix> <keep_latest>`
Remove old backup directories after a successful drill. Matching is
prefix-based and lexicographic, so use sortable names such as
`cortexdb-20260530T220000Z`. `keep_latest` must be greater than zero.

```bash
cortexdb backup-prune ./backups cortexdb- 7
```

#### `backup-offsite-stage <backup_path> <offsite_root> <backup_id>`
Preflight-restore a local backup, copy it under an offsite staging root,
validate the staged copy, then atomically publish it as
`<offsite_root>/<backup_id>`.

```bash
cortexdb backup-offsite-stage ./db.backup ./offsite cortexdb-20260530T000000Z
```

Release/runbook automation can use `make backup-drill-check` to create a
repeatable evidence report at `target/backup-drill/report.json`.
Use `make backup-offsite-check` to create a staged offsite-copy evidence report
at `target/backup-offsite/report.json`.

#### `restore <backup_path> <path>`
Restore a backup into a new target directory and validate the restored database.
The target path must not already exist, which prevents accidental overwrite.

```bash
cortexdb restore ./db.backup ./db.restored
cortexdb validate ./db.restored
```

#### `restore-encrypted <archive_path> <path>`
Restore a passphrase-protected archive into a new target directory and validate
the restored database before returning success.

```bash
export CORTEXDB_BACKUP_PASSPHRASE="choose-a-long-local-passphrase"
cortexdb restore-encrypted ./db.cdbenc ./db.restored --passphrase-env CORTEXDB_BACKUP_PASSPHRASE
cortexdb validate ./db.restored
```

#### Ingestion job commands

```bash
cortexdb ingest-jobs ./db
cortexdb ingest-job ./db 1
cortexdb ingest-job-retry ./db 1
cortexdb ingest-job-cancel ./db 1
cortexdb ingest-job-delete ./db 1
```

These commands expose the same persisted local job records as the HTTP
`/v1/ingest/jobs` routes. They are for local operator review and recovery, not a
distributed background-job system. Job files are written atomically; if the
database restarts with a job still marked `running`, it is shown as `queued` so
an operator can retry, cancel, or delete the record explicitly.

### Search

#### `search <path> <scope> <query> [--mode keyword|vector|hybrid|auto] [--vector <i16,...>] [--algorithm ann|exact]`
Search with explicit or automatic routing. Default mode is `keyword`.

```bash
cortexdb search ./db project:investments "budget solar"
cortexdb --json search ./db project:investments "budget solar" \
  --mode auto --vector "1,2,3"
```

Human output starts with the selected routing strategy. JSON output includes a
`routing` object with `selected_strategy` and `reason`.

#### `search-vector <path> <scope> <vector>`
Approximate nearest neighbor search.

```bash
cortexdb search-vector ./db project:investments "[0.1, 0.2, 0.3, ...]"
```

#### `search-vector-exact <path> <scope> <vector>`
Exact brute-force vector search.

```bash
cortexdb search-vector-exact ./db project:investments "[0.1, 0.2, 0.3, ...]"
```

#### `search-vector-eval <path> <scope> <vector> [--json]`
Compare ANN vs exact search recall.

```bash
cortexdb search-vector-eval ./db project:investments "[0.1, 0.2, ...]"
```

#### `search-explain <path> <scope> <query> [--mode keyword|vector|hybrid] [--vector <i16,...>]`
Explain lexical, vector, and hybrid contribution details for ranked search
results.

```bash
cortexdb search-explain ./db project:investments "solar budget"
cortexdb search-explain ./db project:investments "solar budget" \
  --mode hybrid --vector "1,2,3"
```

### AQL

#### `aql <path> <scope> <aql>`
Execute an AQL query.

```bash
cortexdb aql ./db project:investments \
  'RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects WHERE space = project:investments LIMIT 10 CANDIDATES;'
```

#### `context <path> <scope> <aql> [--json] [--format summary|json|prompt|markdown]`
Execute CONTEXT PACK AQL.

```bash
cortexdb context ./db project:investments \
  'RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects WHERE space = project:investments LIMIT 10 CANDIDATES;' \
  --json

cortexdb context ./db project:investments \
  'RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects WHERE space = project:investments LIMIT 10 CANDIDATES;' \
  --format prompt

cortexdb context ./db project:investments \
  'RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects WHERE space = project:investments LIMIT 10 CANDIDATES;' \
  --format markdown
```

#### `remember <path> <scope> <aql>`
Store a query result into a memory cell.

```bash
cortexdb remember ./db project:investments \
  'REMEMBER "investment summary" AS CELLS WHERE scope == "project:investments" IN BRAIN default;'
```

#### `verify <path> <scope> <aql> [--json] [--format summary|json|markdown|audit]`
Run VERIFY FACT.

```bash
cortexdb verify ./db project:investments \
  'VERIFY FACT "Solar Plant budget is 1.2B KZT" IN BRAIN default;' \
  --json

cortexdb verify ./db project:investments \
  'VERIFY FACT "Solar Plant budget is 1.2B KZT" IN BRAIN default;' \
  --format markdown
```

### Ingestion

#### `ingest-text <path> <scope> <file>`
Ingest a plain text file.

```bash
cortexdb ingest-text ./db project:investments report.txt
```

#### `ingest-json <path> <scope> <file>`
Ingest a JSON file.

```bash
cortexdb ingest-json ./db project:investments data.json
```

#### `ingest-csv <path> <scope> <file>`
Ingest a CSV file.

```bash
cortexdb ingest-csv ./db project:investments data.csv
```

#### `load-fixture <path> <fixture_path>`
Load a fixture bundle.

```bash
cortexdb load-fixture ./db ./fixtures/demo.json
```

### Low-Level Tools

#### `wal-dump <path>`
Dump WAL contents.

#### `wal-validate <path>`
Validate WAL integrity.

#### `wal-truncate <path>`
Truncate WAL to safe offset.

#### `manifest-dump <path>`
Dump manifest contents.

#### `manifest-validate <path>`
Validate manifest.

#### `gc-retired <path>`
Garbage-collect retired segments.

#### `ann-validate <path> [--json]`
Validate ANN/HNSW index.

#### `unlock <path> [--force]`
Remove stale lock file. Use `--force` to bypass safety checks.

```bash
cortexdb unlock ./db --force
```

### Shell Completions

#### `completions <shell>`
Generate shell completions for bash, zsh, or fish.

```bash
cortexdb completions bash > /usr/share/bash-completion/completions/cortexdb
cortexdb completions zsh > /usr/local/share/zsh/site-functions/_cortexdb
cortexdb completions fish > ~/.config/fish/completions/cortexdb.fish
```

### Demo

#### `demo`
Run the interactive investment projects demo.

```bash
cortexdb demo
```

## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success |
| `1` | CLI parse error or invalid argument |
| `2` | Database error (corruption, lock conflict, etc.) |

## Error Messages

The CLI prints actionable error messages:

- **Database locked** → suggests `cortexdb unlock <path>` or `--force`
- **Corruption detected** → suggests `cortexdb repair <path>`
- **Cell not found** → shows the cell_id that was queried
- **Invalid AQL** → prints the parse error location

## Environment Variables

| Variable | Affected Commands | Description |
|----------|-------------------|-------------|
| `RUST_LOG` | All | Set to `debug` or `trace` for verbose output. |
| `CORTEXDB_NO_COLOR` | All | Disable ANSI color codes in output. |
