# Segment Bundles

Segment bundles are the unit that ties persisted row data and indexes together.

```text
segment-{id}.acs  data cells
segment-{id}.acb  bitmap index
segment-{id}.aci  lexical index
```

## Current API

- `SegmentBundle::new(root, id)` builds stable bundle paths.
- `Database::live_segment_bundles()` exposes manifest live bundles.
- `Database::retired_segment_bundles()` exposes manifest retired bundles.
- `Database::garbage_collect_retired_segments()` removes retired bundle files
  and clears retired manifest entries.

## Invariants

1. Live bundles must have all three files readable.
2. Retired bundles are not used for reads or validation of live data.
3. Retired bundle GC must not remove live bundle files.
4. GC is explicit; compaction only retires old bundles.
5. Candidate mappings are validated across live bundles.
