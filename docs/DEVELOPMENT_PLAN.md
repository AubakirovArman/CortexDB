# Development Plan

Current status (2026-05-31):

```text
1) Rust/core milestones
   AQL hardening ✅
   ACLOG WAL ✅
   MemTable MVCC ✅
   WAL replay into MemTable ✅
   Manifest recovery ✅
   .acs/.acb/.aci persistence ✅
   Context Pack ✅

2) Runtime + APIs
   Local single-node database loop ✅
   Typed HTTP API + OpenAPI contracts ✅
   CLI + SDK contracts ✅
   Replication/log/consensus modules ✅
   Browser dashboard ✅ (developer console)
```

Current sprint focus:

1) ANN/HNSW production hardening
   - keep recall/latency gates passing on every merge
   - enforce `production_safe=true` semantics in CI evidence
   - expand drift history checks for larger real-domain corpus baselines

2) Real distributed consensus
   - continue partition/rejoin/failure matrix expansion
   - harden snapshot install + topology/repair races

3) Product surface
   - deliver richer dashboard UX and broader screenshot/fidelity coverage
   - keep SDK package release gates as mandatory for public versions

4) Public release hygiene
   - keep migration/deprecation policy docs current
   - publish release evidence with every release gate set

Long-running rules:

- No new production feature families (new major search/index paradigms) until Core Alpha
  and recovery invariants remain green.
- No API contract changes without OpenAPI/SDK snapshot updates.
- No storage format changes without compatibility and recovery validation.
