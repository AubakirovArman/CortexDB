# Storage Formats

CortexDB Core Alpha uses small binary formats with explicit magic values,
little-endian integer fields, and CRC32C validation.

## Version Policy

| Format | File | Magic | Version state | Compatibility rule |
| --- | --- | --- | --- | --- |
| ACLOG WAL | `.aclog` | `ACLOGv0\0` | `version = 0` in file header | Breaking changes require a new WAL version. |
| Segment | `.acs` | `ACS1` | magic carries v1 | Breaking changes require a new magic. |
| Bitmap index | `.acb` | `ACB0` | magic carries v0 | Breaking changes require a new magic. |
| Lexical index | `.aci` | `ACI1` | magic carries v1 | `ACI0` remains read-only compatible. |
| Manifest | `.acm` | `ACM0` | magic carries v0 | Breaking changes require a new magic. |

All multi-byte integer fields are little-endian.

## Segment `.acs`

```text
magic[4] = "ACS1"
cell_count u32
repeat cell_count:
  cell_id u64
  candidate_id u32
  created_seq u64
  deleted_seq u64, 0 means none
  payload_len u32
  payload bytes
crc32c u32 over all previous bytes
```

Writers persist cells in ascending `candidate_id` order.
`SegmentReader::read_lookup` builds an in-memory lookup for `candidate_id` and
full `cell_id` access without repeated segment scans.

## Bitmap Index `.acb`

```text
magic[4] = "ACB0"
bitmap_count u32
repeat bitmap_count:
  bitmap_handle u64
  value_count u32
  repeat value_count:
    candidate_id u32
crc32c u32 over all previous bytes
```

## Lexical Index `.aci`

```text
magic[4] = "ACI1"
term_count u32
repeat term_count:
  term_len u16
  term utf8 bytes
  value_count u32
  repeat value_count:
    candidate_id u32
doc_length_count u32
repeat doc_length_count:
  candidate_id u32
  document_length u32
crc32c u32 over all previous bytes
```

Legacy `ACI0` files omit the doc length table and remain readable.

## Manifest `.acm`

```text
magic[4] = "ACM0"
generation u64
checkpoint_seq u64
live_segment_count u32
repeat live_segment_count:
  id u64
  generation u64
  checkpoint_seq u64
  cell_count u32
retired_segment_count u32
repeat retired_segment_count:
  id u64
  generation u64
  checkpoint_seq u64
  cell_count u32
crc32c u32 over all previous bytes
```

The CLI can inspect the manifest without opening a database writer:

```bash
cortexdb manifest-validate ./data
cortexdb manifest-dump ./data
```

## Candidate Rules

- Candidate ids are internal compact `u32` ids.
- Candidate id `0` is invalid.
- Candidate ids must map back to full `CellId(u64)` without truncation.
- Across live segments, a candidate id may only refer to one `CellId`.
