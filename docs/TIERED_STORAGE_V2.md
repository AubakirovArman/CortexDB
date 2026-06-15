# Tiered Storage V2

Status: accepted design and guarded prototype.

F01 is a bounded continuation of A08 lazy payload residency. The current
implementation does not change the segment format and does not add a compression
dependency. It defines the hot/cold contract, exposes the prototype flag, and
proves cold payload readback with a bounded hot cache.

## Goals

- Serve checkpointed payloads from cold segment storage without retaining all
  payload bytes in the MemTable.
- Keep a bounded hot payload cache with explicit capacity, resident bytes, hit,
  miss, segment-load, and eviction counters.
- Leave page-block compression and query-plan prefetch as format-compatible
  follow-up work.

## Non-goals

- No storage-format migration in this phase.
- No zstd crate or compressed block writer in this phase.
- No claim that the 10M-100M production workload is complete; this phase is the
  design review plus local prototype gate.

## Placement Policy

Hot data:

- live WAL-tail payloads that have not been checkpointed;
- inline payloads when `PayloadResidency::Memory` is selected;
- recently read checkpoint payload pages in `SegmentPayloadCache`.

Cold data:

- checkpointed segment payload bytes addressed by `(segment_id, candidate_id)`;
- descriptors and indexes remain available for planning without forcing payload
  bytes into memory when `PayloadResidency::Lazy` is selected.

The prototype is enabled through `DatabaseOptions::tiered_storage.enabled` or
`CORTEXDB_TIERED_STORAGE_V2=true`. For useful hot/cold behavior it should be
paired with `PayloadResidency::Lazy` / `CORTEXDB_PAYLOAD_RESIDENCY=lazy`.

## Page Model

The current logical page is one persisted candidate payload. This matches the
existing segment readback address and keeps the prototype compatible with A08.

Future page blocks may group adjacent payload pages inside a segment. That work
must preserve candidate-addressable readback and must keep descriptors readable
without inflating the full payload block into the MemTable.

## Compression Policy

`TieredStorageCompressionPolicy::None` is the only active policy in this phase.
`TieredStorageCompressionPolicy::ZstdReserved` records the planned page-block
compression path without adding a new dependency or changing bytes on disk.

Before enabling real compression, the format change must include:

- segment-format versioning;
- block checksum coverage;
- dual-read compatibility for old uncompressed payload blocks;
- tests for partial block corruption and descriptor-only recovery.

## Query-Plan Prefetch

Prefetch is planned after the physical query plan has selected a bounded
candidate window. It must not prefetch all matching segment payloads. The future
executor hook should accept `(segment_id, candidate_id)` refs from the selected
window and fill only the remaining hot-cache budget.

The current prototype deliberately keeps reads demand-driven. This avoids a
silent memory regression while the planner prefetch boundary is still design
only.

## RAM Budget And Metrics

The hot cache is bounded by `DatabaseOptions::payload_cache_bytes` /
`CORTEXDB_PAYLOAD_CACHE_BYTES`. `PayloadCacheStats` reports:

- `max_bytes`;
- `resident_bytes`;
- `entries`;
- `hits`;
- `misses`;
- `segment_loads`;
- `evictions`.

For F01 acceptance, `resident_bytes <= max_bytes` must hold after cold readback,
and a sequential workload larger than the cache must evict old hot pages while
still serving later reads from cold segment storage.

## Verification

Local gate:

```bash
make tiered-storage-v2-check
```

The gate validates this design surface, runs the cache eviction unit test, and
runs the lazy checkpoint readback integration test:

- `cache_returns_payload_and_evicts_least_recently_used_entry`;
- `tiered_storage_v2_serves_cold_payloads_with_bounded_hot_cache`.

