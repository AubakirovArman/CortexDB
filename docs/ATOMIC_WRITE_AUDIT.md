# Atomic Write Audit

Core Alpha durable files must be published with the same pattern:

```text
encode bytes
-> write unique temp file
-> fsync temp file
-> rename temp -> final
-> fsync parent directory
-> remove legacy temp name if present
```

## Current Audit

| File | Writer | Atomic helper | Checksum | Validation |
| --- | --- | --- | --- | --- |
| `.aclog` | WAL writer actor | append-only, fsync by durability mode | per-record header and payload CRC32C | WAL reader strict/best-effort scan |
| `.acs` | `SegmentWriter::write` | `write_atomic` | CRC32C footer | `SegmentReader::read` |
| `.acb` | `BitmapIndex::write` | `write_atomic` | CRC32C footer | `BitmapIndex::read` |
| `.aci` | `LexicalIndex::write` | `write_atomic` | CRC32C footer | `LexicalIndex::read` |
| `.acm` | `StorageManifest::store` | `write_atomic` | CRC32C footer | `StorageManifest::load` |

## Temp Naming

The current temp name is:

```text
<file>.tmp.<pid>.<counter>
```

Known legacy temp names such as `<file>.tmp` are still cleaned up during open
and repair so older interrupted writes do not block recovery.

## Invariants

1. The manifest is published only after segment and index files are durable.
2. Checkpoint and compact truncate WAL only after manifest publication.
3. Orphan temp files never participate in recovery.
4. Orphan segment/index bundles are ignored unless referenced by manifest.
5. Validation reports missing or corrupt live bundle members.
