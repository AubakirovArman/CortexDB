# CortexDB Epic Task Pools

This backlog converts the 200 roadmap epics into execution pools. Each epic
must move through the same gates before it is considered done:

```text
scope -> design note -> implementation -> focused tests -> docs -> quality gates
```

Quality gates for every code epic:

```bash
cargo check --workspace
cargo test --workspace --all-features
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
```

## Pool Definitions

| Pool | Task pool |
| --- | --- |
| `CORE` | Freeze scope, audit invariants, harden lifecycle, add restart/corruption tests, update core docs. |
| `CANDIDATE` | Define mapping invariant, remove truncation/overflow, persist mapping, validate segments, add multi-checkpoint tests. |
| `WAL` | Freeze format, enforce `CommitSeq`, define recovery policy, test corruption/tails, expose WAL diagnostics. |
| `MVCC` | Define visibility rules, harden tombstones/patches, add stats and historical-read tests, document semantics. |
| `AQL` | Freeze grammar, improve diagnostics, bind/runtime plan tests, add CLI/HTTP query surfaces, update grammar docs. |
| `SEARCH` | Define search API, improve lexical scoring/tokenization, add vector hooks, add quality and benchmark fixtures. |
| `CONTEXT` | Build ContextPack structs, token budget, citations, packing policy, diagnostics, CLI/HTTP/SDK outputs. |
| `INGESTION` | Define cell schema, metadata sections, document/API loaders, enrichment queue, progress reporting. |
| `MEMORY` | Persist AgentView, private/shared scopes, TTL/decay, remember/forget/verify flows, feedback storage. |
| `OPS` | Harden server/CLI/SDK, auth/errors, validation commands, packaging, observability, public demos. |

## Execution Matrix

| Epic | Pool | First task | Done gate |
| --- | --- | --- | --- |
| 001 Core Alpha Freeze | CORE | Freeze Core Alpha scope in docs. | Release checklist has explicit included/excluded scope. |
| 002 Full Workspace Green CI | CORE | Keep check/test/fmt/clippy green. | CI passes on default branch. |
| 003 Core Invariants Test Suite | CORE | Add invariant tests for WAL/MVCC/restart. | Tests fail on ordering or visibility regressions. |
| 004 Database Lifecycle Hardening | CORE | Audit open/close/drop/lock/shutdown. | Double-open, close, drop, and restart tests pass. |
| 005 Lock File Reliability | CORE | Add stale lock owner policy. | Stale lock behavior is documented and tested. |
| 006 Core Error Taxonomy | CORE | Split user/corruption/invariant errors. | External errors are safe and actionable. |
| 007 Database Validation v1 | CORE | Expand validation report coverage. | Bad manifest/segment/index/WAL is detected. |
| 008 Database Stats v1 | CORE | Expand stats fields. | CLI/server expose current storage state. |
| 009 Repair Mode v1 | CORE | Define best-effort repair. | Partial WAL/orphan cleanup has tests. |
| 010 Crash Simulation Harness | CORE | Create fault injection harness. | Checkpoint/compact crash matrix is runnable. |
| 011 Restart Matrix Tests | CORE | Add restart tests for all writes. | Put/patch/tombstone/checkpoint/compact survive restart. |
| 012 Storage Corruption Matrix | CORE | Corrupt each storage file type. | Strict recovery rejects corrupted durable files. |
| 013 Atomic File Write Audit | CORE | Audit all critical writes. | Critical writes use temp/fsync/rename/fsync-dir. |
| 014 Core Benchmark Baseline | CORE | Add put/get/replay benchmarks. | Baseline numbers are reproducible. |
| 015 Storage Format Versioning | CORE | Add compatibility policy. | Format versions are documented per file type. |
| 016 Core Alpha Documentation | CORE | Write Core Alpha status. | README links current guarantees and limits. |
| 017 Core Invariants Documentation | CORE | Document safety invariants. | Invariants map to tests or TODOs. |
| 018 Failure Scenarios Documentation | CORE | Document crash behavior. | Failure docs cover WAL/checkpoint/compact. |
| 019 Release Checklist | CORE | Create release checklist. | Every release gate is checkable. |
| 020 Tag v0.1.0-core-alpha | CORE | Prepare release notes. | Tag exists only after all Core gates pass. |
| 021 Candidate Mapping Completeness | CANDIDATE | Audit all candidate paths. | No CellId-to-u32 truncation remains. |
| 022 CandidateId Newtype | CANDIDATE | Replace raw candidate ids. | Candidate 0 is impossible or rejected. |
| 023 Candidate Overflow Safety | CANDIDATE | Make allocation fallible. | Overflow returns error, never panic/saturate. |
| 024 Segment Candidate Persistence | CANDIDATE | Persist mapping across segments. | Checkpoint/compact/restart preserve mapping. |
| 025 Candidate Mapping Validation | CANDIDATE | Validate duplicate/missing ids. | Validation catches mapping corruption. |
| 026 Segment Bundle Model | CANDIDATE | Define `.acs/.acb/.aci` bundle. | Manifest references bundle metadata. |
| 027 Manifest Bundle Awareness | CANDIDATE | Store bundle references. | Manifest and files are consistent. |
| 028 Segment Generation Semantics | CANDIDATE | Define generation rules. | Delta and compact generations are distinct. |
| 029 Retired Segment Lifecycle | CANDIDATE | Define retired state. | Retired segments are safe to inspect/GC. |
| 030 Segment Garbage Collection | CANDIDATE | Add retired GC plan. | GC never removes live data. |
| 031 Manifest Atomic Switch Tests | CANDIDATE | Simulate switch crash. | Old manifest remains loadable. |
| 032 Manifest Rollback Safety | CANDIDATE | Ignore orphan segments. | Open ignores unreferenced files safely. |
| 033 Manifest Forward Compatibility | CANDIDATE | Define unknown field policy. | Reader behavior is deterministic. |
| 034 Segment Cell Count Validation | CANDIDATE | Compare manifest and segment count. | Mismatch fails validation. |
| 035 Segment Sort Order | CANDIDATE | Choose candidate/cell ordering. | Segment order is documented and tested. |
| 036 Segment Lookup Index | CANDIDATE | Add segment lookup helper. | Lookup is faster than full scan. |
| 037 Segment Tombstone Semantics | CANDIDATE | Formalize tombstone records. | Tombstone-only records never resurrect cells. |
| 038 Segment Compaction Input Planner | CANDIDATE | Plan compact inputs. | Compact does not always require full snapshot. |
| 039 Incremental Checkpoint Planner | CANDIDATE | Track changed cells. | Delta checkpoint reuses candidates correctly. |
| 040 Storage Bundle Validator | CANDIDATE | Validate bundle as unit. | Missing/corrupt member fails validation. |
| 041 WAL Record Format Stabilization | WAL | Freeze ACLOG v0 spec. | Format changes require version bump. |
| 042 WAL CommitSeq Hard Requirement | WAL | Reject new records without seq. | New operation WAL always has durable seq. |
| 043 WAL BestEffort Recovery Policy | WAL | Define safe stop rules. | BestEffort stops at safe offset. |
| 044 WAL Strict Recovery Policy | WAL | Define strict failures. | Strict fails on checksum/header corruption. |
| 045 WAL Truncate Safety | WAL | Truncate only safe tails. | Safe truncate offset is tested. |
| 046 WAL Writer Backpressure | WAL | Add bounded queue design. | Overload behavior is explicit. |
| 047 WAL Group Commit | WAL | Batch fsync in balanced mode. | Batch policy has latency/size tests. |
| 048 WAL Writer Metrics | WAL | Count writes/fsyncs/latency. | Stats expose writer health. |
| 049 WAL Replay Metrics | WAL | Count replay work. | Replay reports records/bytes/duration. |
| 050 WAL Rotation | WAL | Design multi-file WAL. | Rotation survives restart. |
| 051 WAL Checkpoint Boundary | WAL | Define base seq after checkpoint. | New WAL starts from correct boundary. |
| 052 WAL Archive Policy | WAL | Decide keep/delete rules. | Archived WAL cannot be needed for recovery. |
| 053 WAL Compression Sections | WAL | Add optional compression hook. | Compressed payload roundtrips. |
| 054 WAL Encryption Hook | WAL | Define encryption interface. | Encryption can wrap sections later. |
| 055 WAL Section Registry | WAL | Register section semantics. | Unknown sections are handled safely. |
| 056 WAL Unknown Section Compatibility | WAL | Skip/preserve unknown tags. | Future tags do not break reader. |
| 057 WAL Fuzz Tests | WAL | Add random-byte tests. | Decoder never panics. |
| 058 WAL Replay Idempotency | WAL | Replay same input twice. | Resulting MemTable is unchanged. |
| 059 WAL Apply Atomicity | WAL | Apply operation atomically. | Partial apply returns error without mutation. |
| 060 WAL Debug Tooling | WAL | Add dump/validate/truncate tools. | CLI can inspect WAL safely. |
| 061 MemTable Stats v2 | MVCC | Expand stats. | Stats report live/deleted/depth. |
| 062 MemTable Memory Accounting | MVCC | Estimate memory use. | Payload/version bytes are surfaced. |
| 063 ReadTxn Lifecycle | MVCC | Prepare read snapshots. | Txn behavior is stable across writes. |
| 064 Historical Reads | MVCC | Add `read_at(seq)`. | Historical visibility has tests. |
| 065 Snapshot Isolation Tests | MVCC | Build visibility matrix. | Old/new txn behavior is covered. |
| 066 Tombstone Version Model | MVCC | Model tombstone as version. | Deletes compose with historical reads. |
| 067 Patch Semantics v1 | MVCC | Document full replacement patch. | Patch behavior is named and tested. |
| 068 Section-Level Patch Model | MVCC | Design section merge ops. | Merge policy is deterministic. |
| 069 CellAccumulator v2 | MVCC | Implement reducer. | Base+patch+tombstone reduce correctly. |
| 070 Delta Depth Policy | MVCC | Track high delta depth. | Compaction can prioritize deep cells. |
| 071 MemTable Flush Boundary | MVCC | Define checkpoint boundary. | Flush does not break readers. |
| 072 MemTable GC After Checkpoint | MVCC | Remove persisted old versions. | GC keeps visible history needed by readers. |
| 073 IndexDebt Integration | MVCC | Use debt counters. | Debt influences indexing/compaction. |
| 074 MemTable Iterators | MVCC | Add live/deleted/changed iterators. | Iterators are deterministic. |
| 075 MemTable Range Scan | MVCC | Add CellId range scan. | Range results obey MVCC. |
| 076 MemTable Concurrent Read Prep | MVCC | Prepare shared snapshots. | Server can safely read concurrently later. |
| 077 MemTable Serialization Tests | MVCC | Roundtrip via segment. | MemTable -> segment -> MemTable is stable. |
| 078 MVCC Formal Docs | MVCC | Write visibility docs. | Docs match tests. |
| 079 MVCC Edge Cases | MVCC | Test duplicate/out-of-order seq. | Replay rejects invalid seq behavior. |
| 080 Core Cell Model v1 | MVCC | Define cell metadata model. | Core schema replaces payload-line parsing. |
| 081 AQL Grammar Freeze v0.4 | AQL | Freeze grammar docs. | Parser tests cover grammar examples. |
| 082 AQL Explain | AQL | Add explain AST/plan. | CLI can print bitmap bytecode. |
| 083 AQL Parse Diagnostics | AQL | Improve parse errors. | Line/column/kind are stable. |
| 084 AQL Bind Diagnostics | AQL | Improve bind errors. | Safe and internal messages are distinct. |
| 085 AQL Runtime Diagnostics | AQL | Add plan stats. | Runtime returns candidate counts/timings. |
| 086 AQL WHERE IN Operator | AQL | Complete `IN` support. | Parser/binder/VM tests pass. |
| 087 AQL Numeric Filters | AQL | Add metadata numeric filters. | Numeric filters do not use floats. |
| 088 AQL Time Filters | AQL | Add seq/freshness filters. | Time predicates bind to bitmaps/guards. |
| 089 AQL Memory Queries | AQL | Query memory cells. | Memory type filters work end-to-end. |
| 090 AQL VERIFY FACT Execution | AQL | Execute verify plans. | Verification report is deterministic. |
| 091 AQL REMEMBER Execution | AQL | Execute remember writes. | Policy-gated memory writes hit WAL. |
| 092 AQL LIMIT Semantics | AQL | Split candidate/final limit. | Limits are unambiguous. |
| 093 AQL REQUIRE Semantics | AQL | Enforce quality requirements. | Requirements affect plans/results. |
| 094 AQL Query Plan Optimizer | AQL | Add plan rewrites. | Optimizer preserves semantics. |
| 095 AQL Permission Runtime Mask | AQL | Enforce runtime AgentAllowed. | Provider mask is agent-specific. |
| 096 AQL Multi-Brain Prep | AQL | Make BrainId real. | Brain-scoped catalog is tested. |
| 097 AQL Query Cache | AQL | Cache parse/bind plans. | Repeated queries avoid reparse. |
| 098 AQL Golden Tests | AQL | Add golden fixtures. | AST/plan outputs are stable. |
| 099 AQL CLI Command | AQL | Add CLI AQL execution. | CLI returns retrieve results. |
| 100 AQL HTTP Endpoint | AQL | Add HTTP AQL endpoint. | Server returns retrieve results. |
| 101 Search API v1 | SEARCH | Define public search API. | API supports keyword/vector/hybrid modes. |
| 102 BM25 Correctness Upgrade | SEARCH | Implement formal BM25 fields. | Scores match golden cases. |
| 103 Unicode Tokenizer | SEARCH | Replace ASCII tokenizer. | RU/KZ/EN token tests pass. |
| 104 Token Normalization | SEARCH | Add normalization hooks. | Case/punctuation behavior is stable. |
| 105 Stopword Strategy | SEARCH | Define per-language stopwords. | Stopwords are configurable. |
| 106 Field-Aware Lexical Index | SEARCH | Add field weights. | Title/body/source weights are tested. |
| 107 Persisted Lexical Index v2 | SEARCH | Add postings/doc length. | `.aci` supports BM25 metadata. |
| 108 BM25 Over Persisted Segments | SEARCH | Query persisted lexical data. | Search works after checkpoint/restart. |
| 109 Vector API v1 | SEARCH | Define vector payload API. | Exact vector scan is callable. |
| 110 Vector Normalization | SEARCH | Store norm policy. | Dot/cosine modes are deterministic. |
| 111 Vector Persistence `.acv` | SEARCH | Add vector segment file. | Vectors survive restart. |
| 112 Vector Flat Search | SEARCH | Implement exact scan. | Results match brute force. |
| 113 HNSW Correctness v1 | SEARCH | Validate insert/search. | HNSW recall tests pass. |
| 114 HNSW Persistence | SEARCH | Persist graph links. | HNSW survives restart. |
| 115 HNSW Build From Segment | SEARCH | Background build plan. | Segment vectors build graph. |
| 116 Hybrid Search Fusion | SEARCH | Implement RRF. | Hybrid ranking has golden tests. |
| 117 Hybrid Search Plan | SEARCH | Add query planner choice. | Planner selects lexical/vector/hybrid. |
| 118 Reranker Hook | SEARCH | Define external reranker API. | Reranker is optional and bounded. |
| 119 Search Benchmarks | SEARCH | Add benchmark suite. | Bench history is reproducible. |
| 120 Search Quality Tests | SEARCH | Add golden dataset. | Retrieval quality can regress-test. |
| 121 ContextPack Struct | CONTEXT | Define pack structs/API. | Pack returns selected cells and anomalies. |
| 122 Token Budget Estimator | CONTEXT | Improve estimator. | Budgeting is deterministic and tested. |
| 123 MMR Packing v1 | CONTEXT | Add relevance/redundancy. | Duplicate context is reduced. |
| 124 Weighted Jaccard Redundancy | CONTEXT | Add sparse redundancy. | Redundant terms are penalized. |
| 125 Dense Cosine Redundancy Hook | CONTEXT | Add vector redundancy hook. | Dense hook is optional. |
| 126 Numeric Guards | CONTEXT | Prevent numeric contradiction collapse. | Conflicting numbers surface anomalies. |
| 127 Entity Guards | CONTEXT | Add entity overlap guard. | Entity conflicts are visible. |
| 128 SourceRef Model | CONTEXT | Define provenance model. | Source refs are structured. |
| 129 Citation Requirement Enforcement | CONTEXT | Enforce citation requirements. | Missing citations produce diagnostics. |
| 130 Contradiction Reporting | CONTEXT | Add anomaly types. | Contradictions are reported. |
| 131 ContextPack JSON Output | CONTEXT | Stabilize JSON shape. | Server output is documented. |
| 132 ContextPack Binary Cache | CONTEXT | Design cache format. | Cache invalidation is safe. |
| 133 Context Compiler Pipeline | CONTEXT | Compose filters/retrieval/packing. | Pipeline stages are inspectable. |
| 134 Context Compiler Diagnostics | CONTEXT | Explain included/excluded cells. | Diagnostics are stable. |
| 135 Audit Mode Context Policy | CONTEXT | Add audit policy. | Audit requires citations/trust. |
| 136 Fast Mode Context Policy | CONTEXT | Add fast policy. | Fast avoids expensive stages. |
| 137 Semantic Mode Context Policy | CONTEXT | Add semantic policy. | Semantic can use dense hooks later. |
| 138 ContextPack CLI | CONTEXT | Add `cortexdb context`. | CLI returns pack summary. |
| 139 ContextPack HTTP API | CONTEXT | Add `POST /v1/context`. | HTTP returns pack JSON. |
| 140 Agent SDK Context API | CONTEXT | Design SDK method. | SDK exposes retrieve context. |
| 141 KnowledgeCell Schema v1 | INGESTION | Define schema. | Metadata is no longer payload-only. |
| 142 Metadata Encoding | INGESTION | Add metadata section. | Scope/status/type persist structurally. |
| 143 PutCell Metadata API | INGESTION | Add metadata write API. | Metadata writes hit WAL. |
| 144 DocumentBlock Cell Type | INGESTION | Define document block cells. | Blocks include source refs. |
| 145 Fact Cell Type | INGESTION | Define fact cells. | Facts are queryable. |
| 146 Entity Cell Type | INGESTION | Define entity cells. | Entity lookup is possible. |
| 147 Relation Cell Type | INGESTION | Define relation cells. | Relations connect cells. |
| 148 Memory Cell Type | INGESTION | Define memory cells. | Memory type filters work. |
| 149 Tool Cell Type | INGESTION | Define tool registry cells. | Tools are permissioned. |
| 150 SourceRef Cell Type | INGESTION | Define source cells. | Provenance is first-class. |
| 151 Raw Object Registry | INGESTION | Define raw object refs. | Files/objects can be referenced. |
| 152 Ingestion API v1 | INGESTION | Define ingestion endpoint. | Text/JSON ingestion is pluggable. |
| 153 Text Ingestion | INGESTION | Add TXT/Markdown loader. | Documents become cells. |
| 154 JSON/API Ingestion | INGESTION | Add JSON path loader. | JSON source refs are retained. |
| 155 CSV Ingestion | INGESTION | Add CSV row loader. | Rows become structured cells. |
| 156 Excel Ingestion Hook | INGESTION | Add external adapter hook. | Excel can plug in later. |
| 157 PDF Ingestion Hook | INGESTION | Add extraction adapter hook. | PDF text can plug in later. |
| 158 Enrichment Queue | INGESTION | Add job model. | Metadata/vector jobs are tracked. |
| 159 IndexDebt Workflow | INGESTION | Integrate observed/indexed states. | Index debt drives background work. |
| 160 Ingestion Progress API | INGESTION | Add progress surface. | Jobs expose status. |
| 161 AgentView Persistence | MEMORY | Store AgentView. | Agent policies survive restart. |
| 162 Agent Private Scope | MEMORY | Persist private scope. | Private memory is isolated. |
| 163 Shared Project Scope | MEMORY | Define shared project scope. | Project memory is shared safely. |
| 164 Organization Scope | MEMORY | Define tenant scope. | Org knowledge is isolated. |
| 165 Memory TTL | MEMORY | Enforce TTL. | Expired memory is excluded. |
| 166 Memory Decay Policy | MEMORY | Add freshness decay. | Decay affects ranking/packing. |
| 167 Remember Execution | MEMORY | Execute AQL REMEMBER. | Remember writes memory cells. |
| 168 Forget Operation | MEMORY | Add soft delete. | Forget creates tombstone. |
| 169 Memory Search | MEMORY | Filter memory cells. | Memory search is scoped. |
| 170 Agent Feedback Loop | MEMORY | Store usefulness feedback. | Feedback affects future packs. |
| 171 Tool Registry | MEMORY | Persist tool schemas. | Tool cells are retrievable. |
| 172 Tool Permission Model | MEMORY | Add tool access policy. | Tool suggestions respect policy. |
| 173 Tool Suggestion Retrieval | MEMORY | Query tools by task. | Relevant tools are suggested. |
| 174 Verification Report v1 | MEMORY | Build verify report. | Supported/contradicted/mixed is returned. |
| 175 Source Trust Model | MEMORY | Add trust scoring. | Trust affects verification/packing. |
| 176 Conflict Index | MEMORY | Index contradictions. | Conflicts are queryable. |
| 177 Agent Session Model | MEMORY | Add session context. | Temporary context is bounded. |
| 178 ContextPack Feedback Storage | MEMORY | Store used context/outcome. | Pack feedback is durable. |
| 179 Agent API SDK | MEMORY | Add agent-facing API. | SDK has retrieve/verify/remember. |
| 180 Agent Runtime Examples | MEMORY | Add demo agents. | Examples run against local DB. |
| 181 Server Framework Upgrade | OPS | Plan axum/hyper migration. | Manual HTTP limits are documented. |
| 182 Server Error Format | OPS | Standardize JSON errors. | All endpoints share error codes. |
| 183 Server Request Size Limits | OPS | Add body limits. | Oversized requests fail safely. |
| 184 Server Concurrency Model | OPS | Define shared DB handle. | Concurrent requests are safe. |
| 185 Server Auth v2 | OPS | Integrate AgentView/RBAC. | Auth maps to scoped views. |
| 186 Server AQL Endpoint | OPS | Add `POST /v1/aql`. | HTTP can run retrieve queries. |
| 187 Server Context Endpoint | OPS | Harden `/v1/context`. | Context endpoint is stable. |
| 188 Server Search Endpoint | OPS | Add `POST /v1/search`. | Search API is exposed. |
| 189 CLI AQL Command | OPS | Add `cortexdb aql`. | CLI executes AQL. |
| 190 CLI Search Command | OPS | Add `cortexdb search`. | CLI executes search. |
| 191 CLI Context Command | OPS | Harden `cortexdb context`. | CLI emits stable pack format. |
| 192 CLI WAL Tools | OPS | Add wal dump/validate/truncate. | WAL can be inspected safely. |
| 193 CLI Manifest Tools | OPS | Add manifest dump/validate. | Manifest can be inspected safely. |
| 194 Python SDK | OPS | Build minimal client. | Python can put/get/context. |
| 195 TypeScript SDK | OPS | Build minimal client. | TS can put/get/context. |
| 196 Rust SDK API | OPS | Stabilize public API. | Rust API has versioned surface. |
| 197 Docker Image | OPS | Build server image. | Image starts `cortexdb-server`. |
| 198 Observability | OPS | Add metrics/logging/tracing. | Runtime health is visible. |
| 199 Performance Dashboard | OPS | Record bench history. | Performance trends are tracked. |
| 200 Public Demo Dataset | OPS | Build investment demo dataset. | Demo can run end-to-end. |

## Current Execution Slice

The active slice is:

```text
Epic 161 -> agent memory foundation
Epic 165 -> memory TTL expiry
Epic 170 -> feedback weighting
Epic 175 -> source trust model
Epic 174 -> VERIFY FACT report v0
```

Vector search, HNSW, real BM25 ranking, ingestion adapters, distributed
consensus, SDKs, and LLM integration remain outside this slice.
