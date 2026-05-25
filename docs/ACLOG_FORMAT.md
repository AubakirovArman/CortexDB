# ACLOG WAL v0

ACLOG is CortexDB's append-only write-ahead log format.

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
