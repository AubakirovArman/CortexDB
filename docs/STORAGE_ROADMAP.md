# Storage Roadmap

## Implemented

- ACLOG WAL v0 binary codec.
- ACLOG reader scan with safe truncate offset.
- ACLOG writer actor with strict and balanced durability modes.
- In-memory MVCC MemTable skeleton.

## Next

1. Connect WAL replay to `cortex-core::MemTable`.
2. Persist and recover manifest generations.
3. Add `.acs` immutable cell segment files.
4. Add `.acb` bitmap index files.
5. Add `.aci` lexical index files.
6. Add compaction and segment retirement.

## Not Yet

- BM25 ranking.
- Vector search.
- HNSW.
- Server API.
- Distributed mode.
