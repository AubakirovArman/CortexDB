# Decode Fuzzing

`EPIC-E10` uses a deterministic, no-new-dependencies fuzz-like gate for core
decode paths. It builds valid seed files through the normal writers, mutates the
bytes, and asserts malformed inputs return controlled results instead of
panicking.

Covered targets:

- WAL record and WAL file scanning;
- segment records, descriptors, candidate entries, and payload lookup;
- bitmap, lexical, vector, and HNSW index files;
- manifest load;
- AQL parser diagnostics.

Local gate:

```bash
make decode-fuzz-check
```

This target is also included in:

```bash
make check
```

Longer soak command for nightly/manual runs:

```bash
CORTEXDB_DECODE_FUZZ_EXTRA_CASES=2000 make decode-fuzz-check
```

The default gate is intentionally short enough for normal development. The
`CORTEXDB_DECODE_FUZZ_EXTRA_CASES` knob expands deterministic mutation rounds
without adding a separate fuzz toolchain.
