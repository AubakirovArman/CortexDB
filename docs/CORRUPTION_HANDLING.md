# Corruption Handling

CortexDB Core handles corruption with an operator-first rule:

```text
detect precisely -> report one recovery action -> avoid unsafe in-place repair
```

## Commands

Use these commands first:

```bash
cortexdb validate <db>
cortexdb doctor <db>
cortexdb repair --dry-run <db>
```

`validate` works from the database path and does not require `Database::open` to
succeed. This is intentional: corrupt manifests or live segments can prevent
normal open, but operators still need an actionable report.

## Recovery Classes

| Issue kind | Safe automatic action | Recovery command |
| --- | --- | --- |
| `wal` | truncate only to the best-effort safe offset | `cortexdb wal-validate <db> && cortexdb repair --dry-run <db>` |
| `vector_index` | rebuild vector/HNSW artifacts | `cortexdb ann-validate <db> && cortexdb vector rebuild <db> --experimental-hnsw` |
| `hnsw_graph` | rebuild vector/HNSW artifacts | `cortexdb ann-validate <db> && cortexdb vector rebuild <db> --experimental-hnsw` |
| `manifest` | no in-place repair | `cortexdb validate <db>; cortexdb restore <backup_path> <restore_path>` |
| `segment` | no in-place repair | `cortexdb validate <db>; cortexdb restore <backup_path> <restore_path>` |
| `bitmap_index` | no in-place repair yet | `cortexdb validate <db>; cortexdb restore <backup_path> <restore_path>` |
| `lexical_index` | no in-place repair yet | `cortexdb validate <db>; cortexdb restore <backup_path> <restore_path>` |
| `candidate_mapping` | no in-place repair | `cortexdb validate <db>; cortexdb restore <backup_path> <restore_path>` |
| `manifest_reference` | no in-place repair | `cortexdb validate <db>; cortexdb restore <backup_path> <restore_path>` |

## Quarantine Policy

CortexDB does not automatically quarantine live manifest, segment, bitmap, or
lexical files in Core Alpha. Moving those files aside can make a partially
corrupt database look healthy while silently hiding committed data.

The safe behavior is:

1. report the corrupt artifact and recovery action;
2. preserve the original database path for inspection;
3. restore into a separate path from a verified backup;
4. only remove or archive the old path after the restored path validates.

WAL tails and orphan temporary files are the exception. `repair` may truncate a
WAL only to the best-effort safe offset and may remove orphan temp files. Use
`--dry-run` before applying.

## Validation Output

Each validation issue has:

```text
kind
message
recovery_action
recommended_command
requires_restore
```

Text output is meant for humans. JSON output is meant for automation:

```bash
cortexdb --json validate <db>
```

`repair --dry-run` also prints validation issues so an operator can see whether
repair will actually help or whether restore/rebuild is required.
