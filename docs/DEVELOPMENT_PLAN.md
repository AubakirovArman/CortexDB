# Development Plan

Current sequence:

```text
AQL hardening
-> ACLOG WAL
-> MemTable MVCC
-> Manifest recovery
-> .acs segments
-> .acb bitmap index
-> .aci lexical index
-> Context Pack
```

Do not start BM25, vector search, HNSW, server APIs, or distributed mode before
the durable write path and recovery path are integrated.
