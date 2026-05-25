# Storage Roadmap

## Implemented

- ACLOG WAL v0 binary codec.
- ACLOG reader scan with safe truncate offset.
- ACLOG writer actor with strict and balanced durability modes.
- In-memory MVCC MemTable skeleton.
- Usable single-node loop through `cortex-engine`.
- Durable operation `CommitSeq` in WAL `CellCore`.
- Initial `.acs`, `.acb`, and `.aci` file foundations.
- Minimal `cortexdb` CLI for local put/get/tombstone checks.

## Next

1. Persist and recover manifest generations.
2. Integrate `.acs` segments with engine flush.
3. Integrate `.acb` bitmap indexes with AQL retrieve planning.
4. Integrate `.aci` lexical index with ranking.
5. Add compaction and segment retirement.

## Not Yet

- BM25 ranking.
- Vector search.
- HNSW.
- Server API.
- Distributed mode.
