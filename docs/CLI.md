# CLI Reference

The `cortexdb` binary is a local command-line tool that operates directly on the database files. It does **not** require a running server.

## Global Flags

| Flag | Description |
|------|-------------|
| `--json` | Print machine-readable JSON when supported. Applies to `stats`, `validate`, `ann-validate`, `context`, `verify`, and `search-vector-eval`. |
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

#### `repair <path>`
Attempt to repair storage inconsistencies.

```bash
cortexdb repair ./db
```

#### `backup <path> <backup_path>`
Create a validated offline-copy backup. The command opens the source database,
holds the source lock, flushes the WAL writer, validates storage, copies stable
files, and excludes `db.lock` plus known temporary files.

```bash
cortexdb backup ./db ./db.backup
```

#### `restore <backup_path> <path>`
Restore a backup into a new target directory and validate the restored database.
The target path must not already exist, which prevents accidental overwrite.

```bash
cortexdb restore ./db.backup ./db.restored
cortexdb validate ./db.restored
```

### Search

#### `search <path> <scope> <query>`
Keyword search (BM25).

```bash
cortexdb search ./db project:investments "budget solar"
```

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

### AQL

#### `aql <path> <scope> <aql>`
Execute an AQL query.

```bash
cortexdb aql ./db project:investments \
  'GET CONTEXT 3 CELLS WHERE scope == "project:investments" IN BRAIN default;'
```

#### `context <path> <scope> <aql> [--json]`
Execute CONTEXT PACK AQL.

```bash
cortexdb context ./db project:investments \
  'GET CONTEXT 3 CELLS WHERE scope == "project:investments" IN BRAIN default;' \
  --json
```

#### `remember <path> <scope> <aql>`
Store a query result into a memory cell.

```bash
cortexdb remember ./db project:investments \
  'REMEMBER "investment summary" AS CELLS WHERE scope == "project:investments" IN BRAIN default;'
```

#### `verify <path> <scope> <aql> [--json]`
Run VERIFY FACT.

```bash
cortexdb verify ./db project:investments \
  'VERIFY FACT "Solar Plant budget is 1.2B KZT" IN BRAIN default;' \
  --json
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
