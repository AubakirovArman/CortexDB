# Storage Formats

CortexDB Core Alpha uses small binary formats with explicit magic values,
little-endian integer fields, and CRC32C validation.

Upgrade, rollback, and migration rules are defined in
[`UPGRADE_MIGRATION.md`](UPGRADE_MIGRATION.md). Any breaking storage format
change requires a migration note there and in the target release notes.

## Version Policy

| Format | File | Magic | Version state | Compatibility rule |
| --- | --- | --- | --- | --- |
| ACLOG WAL | `.aclog` | `ACLOGv0\0` | `version = 0` in file header | Breaking changes require a new WAL version. |
| Segment | `.acs` | `ACS1` | magic carries v1 | Breaking changes require a new magic. |
| Bitmap index | `.acb` | `ACB0` | magic carries v0 | Breaking changes require a new magic. |
| Lexical index | `.aci` | `ACI2` | magic carries v2 | `ACI0` and `ACI1` remain read-only compatible. |
| Vector index | `.acv` | `ACV0` | magic carries v0 | Breaking changes require a new magic. |
| HNSW graph | `.ach` | `ACH0` | magic carries v0 | Breaking changes require a new magic. |
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
magic[4] = "ACI2"
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
term_frequency_term_count u32
repeat term_frequency_term_count:
  term_len u16
  term utf8 bytes
  frequency_count u32
  repeat frequency_count:
    candidate_id u32
    weighted_term_frequency u32
crc32c u32 over all previous bytes
```

Legacy `ACI0` files omit the doc length and term-frequency tables. Legacy
`ACI1` files include doc lengths but omit term frequencies. Both remain
readable.

## Vector Index `.acv`

```text
magic[4] = "ACV0"
vector_count u32
repeat vector_count:
  candidate_id u32
  dimension u32
  repeat dimension:
    value i16
crc32c u32 over all previous bytes
```

The current vector path is exact integer dot-product scan, not ANN.
All vectors in one `.acv` file must be non-empty and have the same dimension.
Storage validation reports mixed dimensions before query execution; search
paths ignore vectors whose dimension does not match the query vector.

## HNSW Graph `.ach`

```text
magic[4] = "ACH0"
link_count u32
repeat link_count:
  candidate_id u32
  neighbor_count u32
  repeat neighbor_count:
    neighbor_candidate_id u32
dimension u32
metric u32
upper_layer_count u32
repeat upper_layer_count:
  layer_id u32
  layer_link_count u32
  repeat layer_link_count:
    candidate_id u32
    neighbor_count u32
    repeat neighbor_count:
      neighbor_candidate_id u32
optional build_config:
  magic[4] = "HCFG"
  max_neighbors u32
  ef_search u32
  layer_count u32
crc32c u32 over all previous bytes
```

Candidate id `0` is rejected for both node ids and neighbor ids. Checkpoint and
compact write one graph file per segment next to `.acs/.acb/.aci/.acv`.
The upper-layer trailer is optional for compatibility with earlier `ACH0`
files; missing upper layers are interpreted as a valid single-layer graph.
The optional `HCFG` trailer records the HNSW build profile used when checkpoint
or compact wrote the graph. Older files without this trailer remain valid.
Storage validation treats mixed live-segment `HCFG` profiles as an invariant
error because ANN recall and latency SLOs cannot be interpreted consistently
when one live graph was built with a different shape than another.

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
optional hnsw_profile:
  magic[4] = "HNSW"
  max_neighbors u32
  ef_search u32
  layer_count u32
  metric u32
crc32c u32 over all previous bytes
```

The optional `HNSW` trailer records the intended collection-level HNSW build
profile independently from individual `.ach` graph files. Storage validation
compares this manifest policy against every live graph profile so a mixed or
accidentally rewritten ANN graph cannot be served as if it matched the current
collection SLO.

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
