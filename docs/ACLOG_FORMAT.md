# ACLOG WAL v0

ACLOG is CortexDB's append-only write-ahead log format.
The current format is frozen as v0 for Core Alpha. Breaking layout changes
require a `WAL_FORMAT_VERSION` bump and compatibility notes.

## File Header

```text
magic[8]        = "ACLOGv0\0"
header_len u16  = 16
version u16     = 0
flags u16
reserved u16
```

All multi-byte fields are little-endian.

## Record Header

```text
magic u32        = 0x41434c52
header_len u16
record_type u16
lsn u64
payload_len u32
section_count u16
flags u16
payload_crc32c u32
header_crc32c u32
```

The section directory follows the fixed header. Each `SectionEntry` is 16 bytes:

```text
tag u16
reserved u16
offset u32
len u32
reserved u32
```

Sections are 8-byte aligned in the payload. Padding bytes are zero-filled.

## Record Types

- `PutCellBatch`
- `PatchCellBatch`
- `TombstoneBatch`
- `PutEdgeBatch`
- `Checkpoint`
- `ManifestSwitch`

## Section Tags

- `CellCore`
- `PayloadInline`
- `SourceRef`
- `RedundancyMeta`
- `NumericGuards`
- `VectorRef`
- `EdgeHints`

Unknown section tags are retained in decoded section metadata and skipped from
the known `WalRecord.sections` view.

## Recovery

Readers validate file headers, header CRC32C, and payload CRC32C. Partial tails
return a safe truncate offset at the end of the last valid record.

Strict recovery fails on checksum or header corruption. Best-effort recovery
stops at the last valid record and lets the engine truncate the unsafe tail
before appending new records.

## Writer Backpressure

`WalWriter::start_with_options` accepts `WalWriterOptions`.

```rust
WalWriterOptions {
    durability_mode: DurabilityMode::Balanced,
    queue_capacity: Some(1024),
}
```

`queue_capacity = None` keeps the legacy unbounded queue. `Some(n)` creates a
bounded writer channel, so callers naturally block when the writer falls behind.
In `Balanced` durability mode the actor drains a short append batch and performs
one `sync_data` before acknowledging that batch. `Strict` still syncs each
record independently.

## Diagnostics

`WalDiagnostics::summarize` reports record count, safe truncate offset, payload
bytes, known sections, unknown sections, and last LSN. The CLI exposes this as:

```bash
cortexdb wal-validate ./data
cortexdb wal-dump ./data
```
