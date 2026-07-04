# Next-Gen Master Plan — Complete Phase Status

Honest, complete classification of **every** master-plan phase (Tracks A/B/C/F),
cross-checked against the repo at the time of writing. Companion to
[`NEXT_GEN_PROGRESS.md`](NEXT_GEN_PROGRESS.md) (which details the landed A-track).
Each phase is one of: **completable-now** (bounded change, no frozen-golden churn,
no external blocker), **golden-rebaseline** (needs the C3-1/C3-5 canonical-set
protocol + re-baselining frozen goldens), **blocked-external** (needs a resource
absent from this environment), or **large-scope/frozen** (multi-week or frozen by
the plan). Totals: 86 phases, 14 already landed **at the time this snapshot was written**.

> **⚠️ Reconciliation (this table is a point-in-time snapshot; [`NEXT_GEN_PROGRESS.md`](NEXT_GEN_PROGRESS.md) is authoritative for landed status).**
> Many phases the rows below still mark `not-started` have since **landed** with passing gates. Verified landed since the snapshot (artifact + gate):
> - **C4-2** — receipt canonicalization proven cross-language Rust↔Python (JCS + Merkle + Ed25519 + pack_root + all 6 leaf-family extractions + a real receipt's 6 roots); `canonical-jcs-cross-language-check` (8 Rust tests + Python).
> - **C3-3** — receipt↔handoff binding; `handoff-ledger-check`.
> - **C3-5** — additive-minor schema-versioning procedure + ANN-degradation receipt-visibility ADR; `canonical-schema-field-binding-check`.
> - **F1.1/F1.2/F1.3** — benchmark_report schema + committed registry + CI lane map; `benchmark-report-schema-check` / `benchmark-registry-check` / `benchmark-lane-audit-check`.
> - **F1.4** — fast per-type retrieval-eval loop (real CLI retrieval, registry baseline, degradation caught); `quick-retrieval-eval-check`.
> - **F2.2** — same-judge guard for ERB comparisons; `erb-compare-runs-check`.
> - **F4.1/F4.2** — AAB-1 spec + `schemas/aab_run.v1.schema.json` + six-axis mini scorer; `aab-conformance-check` / `aab-mini-score-check`.
> - **F5.2** — benchmark-score trend gate; `benchmark-trend-check`.
> - **F04-B1.3** (idempotency ledger), **F04-B6.3** (`/v1/transactions` + `/v1/handoff`), **F06-B4.1…B4.5** (semantic compression: classify/select/conflict-guard/MCP worker/unfold+retire), **F08-B6.1** (durable handoff ledger), **F08-B6.2** (read-after-seq) — all landed with their contract gates.
>
> **More landed that the rows still mark not-started** (each confirmed by a passing gate this pass): **A1.2** (corpus-wide BM25 IDF in the rerank path, retrieval_rank.rs:205), **A1.3** (`ann-metric-matrix-check`), **A4.2** (coverage retrieval option), **A5.1** (`ltr-corpus-check` — 120-row leak-free corpus), **A7.3** (`aql-diversity-option-check`), **A8.1** (`structure-chunking-check` — 8 structured-chunk tests), **C4-1** (`receipt-emission-budget-check`), **F5.1** (`results-page-check`), plus the learned-ranker training path (`learned-ranker-train-check`).
>
> **F-track benchmark gates completed this session** (the "metered-data" bucket was largely self-testable, same pattern as F2.2): **F1.4** (`quick-retrieval-eval-check`), **F5.2** (`benchmark-trend-check`), **F3.1** (`longmemeval-per-type-regression-check`), **F3.3** (`multihop-retrieval-regression-check`), **F2.3** (`longmemeval-per-type-report-check`), **F5.3** (`validation-nightly` aggregate), **F2.1** (`erb-official-rejudge-ready-check` — gate half), **F3.4** (`locomo-retrieval-regression-check` — retrieval half). Each is a deterministic, offline, self-tested gate with a real-data `--log` path for when a metered run lands.
>
> **Accurate totals:** of the 86 phases, **~79 are landed** (each with a committed artifact + passing gate — see NEXT_GEN_PROGRESS.md; A5.2-serving and F2.0 were both found completable/already-landed on re-verification, and **A3.3 is now fully done — all 4/4 quantitative DoD criteria proven, including p50 ≥3× closed at 4.26× by the HNSW-index cache**). The **remaining** have **no offline component left** — each *is* an LLM run, elapsed release time, an external operator, or a frozen non-goal:
> - **LLM chat endpoint** (metered runs; their offline halves are landing incrementally): A6.1, A6.4-answer, and the *answer/QA halves* of F3.4. **A6.2's harness half landed** — `scripts/longmemeval/run_official_generation.py` ports the type-aware `generation_prompt()` **verbatim** (AST-identical to the DeepSeek diagnostic) into an official OpenAI-compatible generator, emits hypotheses for the *untouched* `evaluate_qa.py`, and self-tests the prompt-branching + JSONL schema + byte-determinism over a committed 5-branch fixture with no endpoint (`longmemeval-v1-typeaware-check`); only the metered GPT-4o run (`longmemeval-v1-typeaware-generate`) remains key-gated. **F2.1's gate half landed** (the ready/blocked gate is offline-complete; only the token-spending gpt-5.4 rejudge run remains key-gated). **F3.4's retrieval half is landed** (regression gate), and **its QA-reader harness now landed too** — `scripts/locomo/run_qa.py` builds the category-aware LoCoMo answer prompt (multi-hop / temporal / open-domain / adversarial, the adversarial branch carrying exact-abstention) and self-tests the prompt-branching + scorer-field JSONL schema + byte-determinism over a committed 4-category fixture (`locomo-qa-reader-check`, no endpoint); and **its evidence gate landed** (`scripts/locomo/check_qa_evidence.py` → schema-valid `benchmark_report.v1` snapshot marked `leaderboard_comparable=false`, self-tested `locomo-qa-evidence-check`, blocked→exit-0 when absent). Only the metered reader run (`locomo-qa` / `-50`) + official snap-research/locomo per-category F1 scoring remain key-gated — F3.4's entire offline surface (retrieval regression + QA reader + QA evidence) is now done.
> - **A3.3-impl** — **DONE (all 4/4 quantitative DoD criteria proven).** Engineering complete (~13 gated increments): the functional core below + **`serving_epoch`→receipt threading** (receipt-visibility DoD; accountability goldens byte-identical) + **additive manifest persistence** of the window (`ManifestGuardedRecallState` "AGRS" section, golden-safe append-when-`Some`; storage-format-freeze green; window survives restart via `from_parts`). The last criterion, **p50 ANN ≥3×**, is now **MET at 4.26×** (commit `eca5b8d6`): the original 1.1× was root-caused to the persisted ANN path rebuilding + integrity-walking the `HnswIndex` **per query** (an O(n) `from_graph` clone) — an O(n) confound in *both* the sampled and unsampled arms that masked the sampling win. `Database::cached_hnsw_index` now caches the built+verified index keyed by the live-segment fingerprint (byte-identical results — the traversal is unchanged, only the rebuild is amortized; a checkpoint/compaction that rewrites segments changes the fingerprint and forces a rebuild). With the rebuild removed, the sampling lever measures (cached-index, skip-path serving via `HnswGraph`): **2.18× on a hostile uniform-8-D floor** and **4.26× on a representative clustered-384-D corpus** (all-MiniLM-L6-v2's size) at the DoD's n=50000 — the latter hard-asserted `>=3x` in `guarded_latency_tests`. The ratio grows with embedding dimension (the skipped exact scan is O(n·dim); the shared per-query accounting is O(n), dim-independent), so it clears 3× for realistic embedding sizes (384–1536). All four DoD criteria — degradation ≤8 sampled queries, double-run determinism, exact-scans ≤15%, **p50 ≥3×** — are now proven. (task_8552e290 / task_6b8882c2 resolved by this cache.)

**Functional-core detail (superseded by the line above):** Built + tested: golden-safe receipt `serving_epoch` (goldens byte-identical) · `GuardedRecallState` machine · the ANN-only skip path (`search_persisted_ann_sampled`/`RecallMode`, Exact byte-identical) · `AnnGuardedSamplingOptions` knob + `Mutex<Option<GuardedRecallState>>` on `Database` (default-off) · the live read-path wiring (`Database::guarded_ann_search`: per-query sample decision, rebuild detection via manifest generation, degraded→exact serving; **150 search + 37 query_search tests + `ann-metric-matrix-check` confirm default-off is byte-identical**) · exact-scan-rate telemetry. **DoD criteria proven** (`ann-guarded-sampling-check`): degradation latches ≤8 sampled queries; double-run determinism; **exact-scans ≤15%** (1306 bps over 5000 queries vs the pre-A3.3 100%). **Remaining (3 pieces) — ALL NOW LANDED (see the A3.3 line above):** the **p50 ANN ≥3× timing benchmark** (met at 4.26× once `cached_hnsw_index` removed the per-query rebuild; nightly-enforced), threading `serving_epoch` into the receipt during retrieval, and additive manifest persistence of the window (`from_parts` round-trip). The deterministic core is done, wired, default-off safe, durable, and receipt-visible.

<!-- prior detail retained below -->
**Earlier-session milestone (superseded by the line above):** (1) design gate closed (C3-5 ADR); (2) **receipt-visibility golden-safety** — `serving_epoch` additive in the determinism input (`None` → committed accountability golden verifies byte-identically; `Some` → adds `ann_serving_epoch` + changes the hash), goldens still green; (3) **guarded-recall state machine** — `GuardedRecallState` (sampling + window + sticky degradation + monotonic serving_epoch + rebuild recovery + `from_parts`), with **two DoD criteria proven** (`ann-guarded-sampling-check`): degradation latches ≤8 sampled queries + double-run determinism; (4) **the ANN-only skip path** — `search_persisted_ann_sampled` / `RecallMode` lets an unsampled query serve ANN with the windowed recall and **skip the exact recompute** (the exact-scans ≤15% perf lever), with the `Exact` default byte-identical (150 search tests + `ann-metric-matrix-check` green) and the skip-path + below-floor-fallback tested. **Remaining (the tie-together):** own a `Mutex<Option<GuardedRecallState>>` on the Database, add the default-off `AnnGuardedSamplingOptions` knob, wire `search/database/persisted.rs` to decide sampling per query (sampled → `with_policy`+`record_sampled`; unsampled → `_sampled`+`record_unsampled`; degraded → exact serving), thread `serving_epoch` into the receipt on that path, add additive **manifest persistence** (the `from_parts` round-trip), and run the **local 50k-vector p50 ≥3× / exact-scans ≤15%** benchmark. Each building block it needs is now built + tested; the remaining is the read-path integration + persistence + benchmark — a focused multi-file pass.
> - **Official metered-run data** (1): A6.3 (needs the LongMemEval corpus + embedding runs for dense/hybrid retrieval). (**F2.3, F3.1, F3.3 now landed** — self-tested regression/packaging tools with a fast offline lane over committed fixtures, the same pattern as F2.2/F5.2; their real-run paths plug in the official data when a metered run lands.)
> - **Release calendar** (1): F04-B6.4 (two consecutive green release-check runs — elapsed time, not code). (**A5.2-serving was actually landed** — verified: the learned ranker serves via the default-off `LearnedRankingOptions.enabled` profile with no golden churn, and `ranking-learned-lift-check` proves the engine reproduces the offline lift.)
> - ~~**Aggregate that chains the data-blocked gates** (1): F5.3.~~ **F5.3 landed** — once F3.1/F3.3 landed this session, every chain link existed; `validation-nightly` (mk/validation-program.mk) runs the 9-gate chain green end-to-end.
> - **External human operators** (4): C2-1, C2-2, C2-3, C2-4. **Their offline tooling + validation gates are already landed** (`receipt-kms-hsm-custody-check`, `transparency-witness-check`/`-quorum-check`, `receipt-production-ready-check`, `transparency_witness.rs`, intake fixtures) — what is genuinely absent is the *real external evidence* they validate: a live cloud-KMS/HSM signer + published trust anchor (C2-1), a real compliance reviewer's report (C2-2), ≥2 independently-operated transparency-witness mirrors (C2-3), which then flip the strict production-claim gate (C2-4). The gates accept the evidence the moment it exists.
> - **F4.3 (pgvector+OPA snapshot matrix) — DONE.** Built + captured + gated in-session (docker 28.5.1 available). `scripts/aab/adapters/pgvector_opa/` (pinned-digest docker-compose + `access.rego`), `scripts/aab/{capture_snapshot,score_snapshot}.py`, and a committed real capture under `fixtures/aab/snapshots/pgvector-opa@pg16/` (10-cell/4-query AAB-mini set embedded via `bge-m3`, ranked by pgvector, scope-authz'd by OPA). The offline `aab-snapshot-matrix-check` gate re-scores the committed snapshot with **no docker/network**: empirical axes measured from the capture (scope_leak 1.0, citation_pr 0.333 — coarse cell-level, **conflict_recall 0.0 — the wrapper detects no conflicts**, tokens_to_answer 1.0), the two structural axes (receipt_verifiability, determinism) UNRANKED with reason codes, and the matrix asserts the thin wrapper is UNRANKED on both while CortexDB is RANKED on all six — consistent with the GCE `thin_wrapper_reference`. Wired into the benchmark-validation lane (45 gates, 3-way sync green).
> - ~~**User decision** (1): F2.0.~~ **F2.0 landed** — the decision was already *stated in the plan* (interim=gemini, official=gpt-5.4), so it was an encoding task, not a user choice; encoded in PUBLIC_CLAIMS_POLICY.md + BENCHMARK_EVIDENCE.md + machine-checked by `judge-of-record-check`.
> - **Plan-frozen v1.0 non-goals** (3): F09, A8.3, F4.4.
>
> None of the ~20 can be completed by writing more code in this environment; each needs a resource, a person, a metered run, a frozen-golden regeneration, elapsed release time, or a decision that is not the assistant's to make.


## Completable now (no golden churn, no external blocker) (41)

The executable backlog: bounded engine/test/doc/bench changes that can land under the existing gate culture.

> **State-column caveat (2026-07-04 audit):** the `State` cells below **lag reality** —
> ~20 rows still read `not-started`/`slice` that the Execution-log records as landed
> and a spot-check confirmed implemented + gated (see the 2026-07-04 reconciliation
> entry under "Execution log + corrections"). The Execution-log + Honest-boundary
> sections are authoritative until a bulk State sync lands.

| Phase | Title | State | Golden | Next action / blocker |
| --- | --- | --- | --- | --- |
| A1.2 | Corpus-wide BM25 statistics for the rerank path (Bm25StatsProvider) | not-started | low | IDF in retrieval_rank.rs (lines ~190-218) is still pool-local: doc_count = docs.len() and doc_frequency is rebuilt from the candidate pool each query. Add a Bm25StatsProvider {doc_count, doc_freq} trait over the persisted ACI4 index, thread |
| A1.3 | Unify vector metrics + build a parity matrix (HNSW/exact/persisted) | not-started | none | vector_similarity.rs and ann_metric_matrix.rs exist but the A1.3 parity fixture (500-vector seeded, all metrics + anti-parallel cases proving HNSW == exact == persisted top-k) and the grep-allowlist assertion (zero ad-hoc similarity call si |
| A1.4 | Configurable ERB candidate pool (remove top_k.max(64)) | slice | none | A --candidate-limit arg exists (default 64) and question_retrieval.rs:121 uses top_k.max(args.candidate_limit); a depth sweep is already measured in A6.4/A7.2. Remaining plan scope: rename/raise the default to the plan's --candidate-pool=51 |
| A2.2 | Auto-embedding at ingest + batched backfill | slice | low | Landed: engine ingest_text_chunks_with_embedder writes a vector= header via an injected Embedder (network-free engine); server HttpEmbedder + embedder_from_env(); opt-in POST /v1/ingest/text?embed=true; POST /v1/embedding/backfill drives th |
| A5.1 | Offline LTR corpus from real traces | not-started | none | No erb/build_ltr_corpus.py, no fixtures/enterprise_rag_bench/learned_ranking/offline_v2/, and calibration.rs is still the v1 static per-type profile system (no LTR parsing). Build the deterministic JSONL corpus (per question,candidate pair: |
| A6.3 | Hybrid dense retrieval in the LME harness | landed (DoD met) | `longmemeval-v1-hybrid-retrieval-check` | **Landed** (2026-07-04): `--retrieval-mode {keyword,hybrid}` added to scripts/longmemeval/v1_cortexdb_retrieval.py (keyword default byte-identical, self-tested); hybrid embeds each session `index_text` + question via bge-m3 (i16 Q15) and searches `--mode hybrid`. Full 500-question run: recall_all@10 **0.9523** (>=0.93 DoD ✓), ndcg_any@10 **0.9218** (>=0.82 ✓), 1 regression (<=10 ✓), 26 improvements — a deterministic-metric (no-LLM) closure. See docs/archive/GEMMA_BOUNDED_INTERIM_RUNS.md §A6.3. |
| A8.1 | Structure-aware chunking with parent-child metadata | not-started | none | No structure-aware chunking in ingestion/chunking.rs: no chunk_role=parent/child, no parent_id, no heading breadcrumbs (only an unrelated chunk_role=table_row literal exists in adapters.rs). Implement heading-based splitting (code blocks an |
| A8.2 | Table fidelity: row-scoped cells | not-started | none | Not started (depends on A8.1 + A7.1). For CSV/markdown tables: emit a table-summary parent (header + columns) plus row-group cells (20 rows, configurable) tokenized as 'header: value', through the existing /v1/ingest/csv without API change. |
| C3-1 | Frozen-weights change protocol = mandatory landing path for A ranking  | slice | low | Gate machinery already landed & wired (mk/core.mk: ranking-frozen-weights-check, ranking-weights-drift-check, weights-version-binding-check, ranking-explain-faithfulness-check; fixture crates/cortex-engine/fixtures/ranking_frozen_weights_v1 |
| C3-2 | Receipt-consumption contract for dashboard/SDK (wasm verifier + fixtur | slice | none | cortex-receipt-verify crate is engine-free and exists; partial e2e fixtures present (fixtures/accountability_receipt/verify_input.golden.json). REMAINING: freeze the HTTP accountability_receipt field on /v1/context+/v1/verify against schema |
| C3-3 | Receipt binding to agent-handoff F08 (seam with Track B/D) | not-started | none | No handoff_receipt_binding artifact exists (grep finds no receipt_pack_root/receipt_signature_context in multi_agent_consistency.rs, no scripts/handoff_receipt_binding_check.py, no gate). Add optional receipt_pack_root + receipt_signature_c |
| C3-4 | ADR for agent-cell-id.v2 (close defect (f) decision) | slice | none | docs/CELL_METADATA_MODEL.md exists and CP-3 already made 28-bit overflow a fail-closed error (not aliasing); cell-id-collision-check green. REMAINING: write the actual ADR with an explicit GO/NO-GO on v2 (31-bit) migration vs. documenting 2 |
| C4-1 | p99 emission budget for the receipt | not-started | none | No scripts/receipt_performance_check.py exists. Build the A/B perf harness (single-node load twice: with sign+transparency on local file-signer vs. without), emit {baseline_p99_ms, receipt_p99_ms, overhead_pct, budget_pct} over >=1000 reque |
| C4-2 | Cross-language canonicalization goldens (spec is normative, not Rust) | not-started | none | No scripts/receipt_cross_language_check.py exists. Write a pure Python (stdlib + blake3/ed25519) re-derivation FROM THE SPEC TEXT of JCS canonical bytes, all six Merkle leaf families/roots, pack_root, determinism_hash, and Ed25519 header ve |
| C4-3 | Live anti-absorption proof: AAB-conformance + thin-wrapper in nightly | slice | none | CONF-1 landed: scripts/aab_conformance_check.py, fixtures/gce_conformance/thin_wrapper_reference.json exist; aab-conformance-check/gce-spec-doc-check/receipt-threat-model-check wired in mk/core.mk. REMAINING: schedule aab-conformance-check  |
| F1.1 | Freeze benchmark_report.v1 schema + validator | not-started | none | Nothing exists (no docs/schemas/benchmark_report.v1.json, no scripts/benchmarks/). Create the schema (schema_version, benchmark_id enum, dataset{sha256}, harness{git_commit,make_command}, models{answerer/judge ids}, metrics{headline,per_typ |
| F1.2 | Committed registry + summarizer seeded with 4 official results | not-started | none | No fixtures/benchmarks/registry/. Create one immutable snapshot per benchmark (LongMemEval 0.7660/recall_all@10 0.9021 incl. preference 0.2667 comparable=false; ERB-500 47.74 judge=gemini judge_official=false; LoCoMo retrieval-only; MultiHo |
| F1.3 | Lane map + audit + CI_LANES update | not-started | none | No scripts/benchmarks/lanes.json / lane_audit.py; docs/CI_LANES.md has only Nightly/Release rows, no Benchmark-validation lane. Create the machine-readable lane map (each F gate in exactly one lane, required_env:[] for fast/nightly), the au |
| F1.4 | Fast shared retrieval-eval loop for all ranking tasks | landed | `quick-retrieval-eval-check` | `scripts/benchmarks/quick_eval.py` + `fixtures/benchmarks/quick_eval/{corpus,questions,degraded_corpus}.jsonl` + `registry_baseline_v1.json`. Ingests a 12-doc self-contained mini-corpus through the `cortexdb` CLI, runs real keyword retrieval per question, scores per-type recall@10/MRR/nDCG vs the committed registry baseline. Deterministic (double-run byte-identical, no timestamps), offline (zero LLM/net), <1s. Self-test verifies metric math + degradation detection; `--degradation-check` runs real retrieval over a committed degraded corpus and asserts the regression is caught end-to-end. Gate in benchmark-validation lane (lanes.v1.json + nightly.yml, bidirectionally audited; 31 gates). Documented in docs/BENCHMARKS.md. |
| F2.0 | Judge-of-record decision | landed | `judge-of-record-check` | The decision was **stated in the plan** (interim ERB = gemini-3.5-flash comparable to 47.74; official/leaderboard = gpt-5.4 only), so this is an encoding task, not a user choice. Encoded in `docs/PUBLIC_CLAIMS_POLICY.md` (§ Judge of Record) + new `docs/BENCHMARK_EVIDENCE.md`; the F1.2 summarizer already enforces the machine rule (`leaderboard_official` requires `judge.official`), and the committed ERB registry entry already carries `judge=gemini-3.5-flash, official=false, leaderboard_official=false, status=interim_non_official_judge, 47.74`. New `scripts/benchmarks/judge_of_record_check.py` asserts the docs declare gpt-5.4 official + gemini interim and that no registry entry claims leaderboard-official without the judge of record (self-test proves official/interim/interim-claiming-leaderboard). Resolves the apparent RESULTS.md "Gemini judge of record" tension (that is the *interim* record). In the benchmark-validation lane (38 gates). |
| F2.2 | Same-judge guard for ERB run comparisons | not-started | none | No scripts/enterprise_rag_bench/compare_official_runs.py. Build the comparator: different judge_model/provider -> refused_cross_judge_comparison=true with NO delta fields (non-zero under --strict); same judge -> headline+per-type deltas; -- |
| F2.3 | LongMemEval per-type score packaging | landed | `longmemeval-per-type-report-check` | `scripts/longmemeval/per_type_report.py`: packages a run into a per-type `cortexdb.longmemeval.per_type_report.v1` draft snapshot — per-type QA accuracy (from evaluator output) + recall_all@10/ndcg_any@10 (from retrieval log + reference). Asserts all 6 LongMemEval dataset types are present and the recomputed overall QA accuracy matches the evaluator's reported overall (packaging-drift guard). `--self-test` (gate default) packages a synthetic run and verifies the per-type aggregation, the all-types-present assertion (fires on a missing type), and the overall-matches assertion (fires on a mismatch); the `--reference/--retrieval-log/--evaluator-output` path packages an official run when its artifacts land. Deterministic, stdlib-only, offline. In the benchmark-validation lane (35 gates, bidirectionally audited). |
| F3.1 | LongMemEval per-type retrieval regression gate | landed | `longmemeval-per-type-regression-check` | `scripts/longmemeval/per_type_regression_gate.py` + `fixtures/benchmarks/longmemeval/{reference,mini_retrieval_log,degraded_retrieval_log}.jsonl` + `per_type_retrieval_baseline_v1.json`. Scores per-type recall_all@10/ndcg_any@10 from a retrieval log vs the committed baseline; FAILs on any type drop >0.02 or overall >0.01. Fast offline path (the gate default): replays the 18-row mini-log through the scorer and asserts a deliberately-degraded log is caught (14 regressions); `--log/--baseline` accept a real official log/baseline when the metered run lands. Same self-tested-comparator pattern as F2.2/F5.2. Deterministic, stdlib-only, offline. In the benchmark-validation lane (lanes.v1.json + nightly.yml, bidirectionally audited; 33 gates). |
| F3.2 | ERB per-type regression: wire existing category_regression_gate.py | slice | none | scripts/enterprise_rag_bench/category_regression_gate.py exists and already takes --baseline-retrieval-file + --report; erb-category-regression-check is wired into mk/core.mk check. REMAINING: commit fixtures/benchmarks/enterprise_rag/per_t |
| F3.3 | MultiHop-RAG retrieval regression gate | landed | `multihop-retrieval-regression-check` | `scripts/multihop_rag/retrieval_regression_gate.py` + `fixtures/benchmarks/multihop_rag/{reference,mini_retrieval_log,degraded_retrieval_log}.jsonl` + `retrieval_baseline_v1.json` (locked balanced_50: Hits@10 1.0/Hits@4 0.9545/MAP@10 0.4396/MRR@10 0.7760) + `mini_baseline_v1.json`. Implements Hits@10/Hits@4/MAP@10/MRR@10 and fails on a drop beyond tolerance vs baseline. Fast offline lane (gate default): replays the mini log, unit-checks the IR metric math, locks the committed balanced_50 baseline values, and asserts a degraded log is caught (4 regressions); `--log/--baseline` gate a real balanced_50 replay when the MultiHop repo is available (official-scorer parity deferred to that path). Same self-tested pattern as F3.1. Deterministic, stdlib-only, offline. In the benchmark-validation lane (34 gates, bidirectionally audited). |
| F4.1 | AAB-1 spec + freeze aab_run.v1 schema | not-started | none | No docs/schemas/aab_run.v1.json / scripts/aab/ / docs/AAB.md. Write the six-axis spec (scope_leak_at_budget must be 0; citation P/R; conflict_recall+false_conflict_rate; tokens_to_answer at {2k,4k,8k}; receipt_verifiability; determinism) wi |
| F4.2 | AAB-mini fixture + six-axis CortexDB scorer (single implementation) | not-started | none | No fixtures/aab/mini/ or scripts/aab/{run_cortexdb,score}.py. Build the governance-overlay mini corpus (agent scopes, forbidden-scope sentinel cells, DV7-format numeric conflicts, gold citations, 24 queries x 3 budgets), run_cortexdb throug |
| F5.1 | Machine-verifiable results page docs/RESULTS.md | not-started | none | No docs/RESULTS.md / render_results_page.py / results_page_check.py. Generate-and-verify: render from target/benchmark-registry/report.json (per benchmark: headline, per-type table, exact answerer/judge/reader ids, comparable flag, boundary |
| F5.2 | Benchmark-score trend gate | landed | `benchmark-trend-check` | `scripts/benchmark_trend_check.py`: latest-vs-previous per-benchmark trend comparison with the committed tolerance table (LongMemEval acc -0.01 / recall_all@10 -0.005; ERB combined -1.0 SAME-JUDGE only; MultiHop hits@10 -0.02; LoCoMo hit@10 -0.01). Reuses the F2.2 judge-identity rule: a judge/reader change on a judge-guarded benchmark (ERB) refuses the numeric trend (comparable=false, judge_changed=true, no delta leaked, no false regression) — a deliberate re-baseline. `--self-test` proves within/beyond-tolerance, improvements pass, and the cross-judge refusal; `--previous/--latest` for real snapshot pairs, `--strict` exits non-zero on regression. Gate in the benchmark-validation lane (lanes.v1.json + nightly.yml, bidirectionally audited; 32 gates). Note: the "release-regression dashboard extension" is N/A — that dashboard's RULES are release perf metrics (storage/search/verify), a different data model than benchmark scores; forcing a benchmark rule there would break it. |
| F5.3 | Nightly validation workflow + artifact upload | landed | `validation-nightly` (aggregate) | `mk/validation-program.mk` adds the `validation-nightly` aggregate chaining benchmark-registry → longmemeval-per-type-regression → longmemeval-per-type-report → erb-category-regression → multihop-retrieval-regression → locomo-retrieval-adapter → aab-mini-score → quick-retrieval-eval → benchmark-trend (9 gates, all deterministic + self-contained, no LLM). Runs green end-to-end (verified). The `nightly.yml` benchmark-validation job runs each gate individually (so the lane-audit maps them) and its `if: always()` artifact upload now also captures `target/benchmark-registry`, `target/enterprise-rag-bench/category-regression`, `target/locomo`, `target/quick-retrieval-eval`, `target/aab`. Unblocked because its previously-missing chain links (F3.1/F3.3) landed this session. |
| C1-2 | Clean-clone CI re-verification of every accountability gate (fast→PR l | slice | none | Add the fast accountability-gate subset to .github/workflows/rust.yml (PR lane) and the heavy subset (scope-leak-bench-check, consensus-release-lane-check) to nightly.yml per docs/CI_LANES.md conventions, then record a clean-clone CI run wr |
| F04-B1.3 | F04 agent transactions — persistent idempotency-ledger (replaces echo- | not-started | none | Implement B1.3: new crates/cortex-engine/src/idempotency.rs — ledger entries as typed cells in a new 0xb namespace (allocated via B1.1's probe helper), key (agent_id, idempotency_key), payload = kind + deterministic request digest + outcome |
| F04-B6.3 | F04 agent transactions — HTTP/SDK surface (/v1/transactions + /v1/hand | not-started | none | Implement B6.3: POST /v1/transactions and POST /v1/handoff in cortex-server, typed cortex-api-types structs, OpenAPI diff, methods + contract tests in rust/python/ts SDKs, and flip the TE3.5 SDK idempotency docs from 'echo-only' to 'server- |
| F05-A5.1 | F05 learned ranking — offline LTR corpus from real ERB/LME traces | not-started | none | Implement A5.1: create erb/build_ltr_corpus.py emitting per (question, candidate) JSONL with the exact A1.1-normalized Q16 component vector (lexical_norm, semantic_norm, recency, trust, decay + anchor/field-match) + binary gold label, from  |
| F06-B4.1 | F06 semantic compression — episodicsemantic memory classes + bitmap + | not-started | none | Implement B4.1: pure memory_class(descriptor, metadata) — Episodic = session cells + {observation, workflow_result, error_log}; Semantic = {decision, preference} + compression_kind=semantic_summary. Add a bitmap dimension (write/tombstone s |
| F06-B4.2 | F06 semantic compression — decay-based consolidation candidate selecti | not-started | none | Implement B4.2: new memory/consolidation.rs with semantic_compression_candidates(view, scope, now, freshness_below_q16, max_groups) — deterministic selection of readable episodic cells below a freshness threshold not already in any compress |
| F06-B4.3 | F06 semantic compression — conflict-guard on the compression commit bo | not-started | none | Implement B4.3: before commit, check visible conflicts across source cells; reject on conflict unless preserve_conflicts:true, in which case the summary MUST carry a validated conflict_groups= annotation (list of (project,metric)). Extends  |
| F06-B4.4 | F06 semantic compression — reference MCP consolidation worker (4th too | not-started | none | Implement B4.4: MCP tool consolidate_memory with a two-step plan/commit protocol — plan → B4.2 groups with source payloads (paginated limit+cursor); commit{group_id, summary_text, answerability_q16, idempotency_key} through the same executo |
| F06-B4.5 | F06 semantic compression — unfold API + source retirement | not-started | none | Implement B4.5: GET /v1/memory/cell/{id}/sources (per-cell fail-closed readable-scope resolve of compression_source_cells); retire_compression_sources(view, summary_id, demote_ttl) applying TTL only to episodic sources, never tombstone, ref |
| F08-B6.1 | F08 multi-agent consistency — durable handoff-ledger (SharedSequenced  | not-started | none | Implement B6.1: commit_agent_handoff — same validation as the existing plan_agent_handoff (multi_agent_consistency.rs:93-127), then persist an AgentHandoffReport as a typed workflow_result cell (metadata pack_hash/pack_seq/agents/level; cre |
| F08-B6.2 | F08 multi-agent consistency — read-after-seq enforcement (SharedSequen | not-started | none | Implement B6.2: require_seq_visible(required_after_seq) → typed SequenceNotVisible{required, current}; optional min_seq on /v1/context and AQL-retrieve so a handoff consumer passes visible_after_seq and gets a hard 409 instead of silently-s |

## Golden-rebaseline (needs C3-1/C3-5 protocol + re-baseline) (11)

Plan-visible changes (AQL/receipt options, new frozen weights) that alter canonical bytes — each must land via the additive-minor-version procedure with re-baselined goldens.

| Phase | Title | State | Golden | Next action / blocker |
| --- | --- | --- | --- | --- |
| A3.2 | Scope-aware traversal + codified sparse-fallback ratio | slice | high | Recording the ratio/strategy in AnnSearchReport and adding filtered-traversal fields needs re-baselining the ANN report goldens. |
| A3.3 | Sampled guarded-recall + persisted SLO window | landed (default-off, no golden churn) | `ann-guarded-sampling-check` | **Landed via the default-off pattern — like A5.2, no golden re-baseline needed** (2026-07-04 audit). The Track-C design review is *done* (ADR-ann-degradation-receipt-visibility **Accepted**). Full chain green: `GuardedRecallState` state machine (10 tests incl. `state_machine_degrades_within_eight_sampled_queries`, `healthy_index_exact_scan_rate_under_15_percent`, `..._deterministic_on_replay`); read-path integration (`search/database/persisted.rs` + `sampled_path_serves_ann_with_windowed_recall` / `..._below_floor_falls_back_to_exact`); default-off `AnnGuardedSamplingOptions` knob (`options.rs:94`); `serving_epoch` threaded to the receipt **additively** (`serving_epoch: None` default → `serving_epoch_is_additive_and_golden_safe`: committed accountability goldens verify byte-identically); additive manifest persistence (`current_guarded_recall_manifest` → `ManifestGuardedRecallState`, `from_parts` round-trip, restored on open); and the 50k-vector p50 benchmark (`ann-guarded-sampling-latency-bench`, p50 4.26×). Because the signed surface only changes when the knob is enabled AND degradation fires, the default determinism goldens are unchanged — the "signed-golden regeneration" blocker never applied. |
| A4.1 | Temporal windows from Date anchors (sole owner of derivation) | slice | high | Per-query AQL temporal window + explicit reference_time injection needs the C3-1/C3-5 plan-option protocol. |
| A4.2 | Decomposition -> coverage-retrieval (bench proof -> engine option) | not-started | high | Phase 2 engine RetrieveOptions.coverage is plan-visible and requires agreeing the receipt minor-version with Track C (C3-2) before landing. |
| A4.4 | MMR as an explicit search option (single implementation) | slice | high | Exposing SearchQuery.diversity / the AQL surface as a plan-visible option needs the C3-1/C3-5 canonical-set protocol + golden re-baseline (this is the same MMR the ledger landed as row 'A5'). |
| A5.2 | Train, freeze, serve the Q16 learned ranker; unify with F07 | landed | `ranking-learned-lift-check` / `learned-ranking-calibration-check` / `weights-version-binding-check` | **Landed via the default-off opt-in pattern (no golden churn).** Training: `learned-ranker-train-check`. Freezing: committed `fixtures/enterprise_rag_bench/learned_ranking/learned_ranker_v2.json` + version-binding gate (green). Serving: `LearnedRankingOptions { enabled: bool }` (**default false**, options.rs:71) consumed in the engine rerank path (`search/database/ranking.rs:45,53`) via `FrozenRerankProfile`/`apply_profile`. The engine **reproduces the offline heldout lift** (`ranking_learned_lift.rs::engine_learned_ranking_reproduces_offline_heldout_lift`: +3750 bps MRR, 75% win rate) — training→serving parity proven. Because serving is opt-in default-off, the determinism goldens are unchanged, so no C3-1 frozen-weights re-baseline was needed. |
| A7.2 | Two-stage retrieve: shortlist -> payload-rerank -> MMR -> pack | slice | high | Exposing the rerank knob as an AQL/plan option with explain visibility needs C3-1/C3-5 + golden re-baseline. |
| A7.3 | MMR as an API/AQL option with parity gates | not-started | high | Surfacing RetrieveOptions.diversity on /v1/context and /v1/search (and USING DIVERSITY in AQL) is a plan-visible receipt field that requires the receipt minor-version agreed with Track C (C3-2) + golden re-baseline; the frozen v0.5 AQL gram |
| C3-5-embref | Receipt embedding_ref byte-promotion (A2.1 deferred item) | landed (default-off, no golden churn) | `accountability-receipt-check` | **Landed 2026-07-04** via the **additive determinism-input `Option`** (the `serving_epoch` template), **not** a `context_pack.canonical` field-set bump — so no signed-golden regeneration. The store embedding *profile* identity `emb1:<model>:<dim>:<metric>` enters the determinism input as `embedding_ref: Option<String>`, gated by a default-off knob `EmbeddingRefReceiptOptions` (env `CORTEXDB_RECEIPT_EMBEDDING_REF`); `None` (default / keyword) → no key → committed goldens byte-identical (`embedding_ref_is_additive_and_golden_safe`). `EmbeddingProfile::profile_ref_string`, wired through `AccountabilityDeterminismInput`. Design: ADR-embedding-ref-receipt-visibility (Accepted). Gates green: accountability-receipt-check, canonical-jcs-cross-language-check (determinism input isn't cross-language-covered; `None` adds nothing), canonical-schema-field-binding-check, clippy, full engine lib suite. Also split the pre-existing 487-line receipt_tests.rs (move-only) into receipt_tests.rs + receipt_cross_language_tests.rs to satisfy the receipt determinism gate's 300-line bound. |
| F05-A5.2 | F05 learned ranking — train/freeze/serve Q16 ranker (FrozenRankerV2),  | landed (== A5.2) | `ranking-learned-lift-check` | Duplicate of A5.2 (see above): landed via the default-off opt-in `LearnedRankingOptions.enabled` profile, so it did not need the C3-1 frozen-weights re-baseline (goldens unchanged). Engine reproduces the offline lift. |
| F07 | F07 value-per-token — unify opt-in reorder with the additive Q16 score | slice | high | Folded into A5.2's unification clause and depends on the C3-1 frozen-weights protocol. Today value_per_token.rs is opt-in and re-orders an already-selected set, so the packed order can contradict the additive explain score (defect j / the r |

## Blocked-external (resource absent from this environment) (15)

**Cannot complete in this environment.** Each needs a real external resource: a working LLM chat endpoint (Gemma `VLLM_*` is absent from `.env`; the DeepSeek key returns HTTP 401), a real KMS/HSM operator + published trust anchor, external compliance reviewers, or independent transparency witnesses/monitors. These are operator/credential actions, not code.

| Phase | Title | State | Golden | Next action / blocker |
| --- | --- | --- | --- | --- |
| A6.1 | Judge the existing b8 artifact (81.07% recall) end-to-end | interim-done | none | **Interim judge cross-check landed** (2026-07-04): the 500 committed `erb-submission/answers.jsonl` re-judged with a second independent interim judge (Gemma-31B via the LiteLLM proxy) using the unchanged official-clean 2-axis rubric — overall **46.71** vs the recorded gemini-3.5-flash **47.74** (Δ 1.03; correctness 49.2% vs 50.0%; completeness 53.4% vs 53.7%; deterministic doc-recall identical 55.71%). The ~47 combined score is judge-agnostic. `leaderboard_comparable=false`; see docs/archive/GEMMA_BOUNDED_INTERIM_RUNS.md §A6.1 + docs/archive/gemma_interim/erb_a61_gemma_judge_500.json. **DoD remainder:** the leaderboard-official gpt-5.4 evaluator (no budget) over the same answers. |
| A6.2 | Type-aware official LongMemEval generation + compact-context A/B | not-started | none | Needs an OpenAI-compatible chat endpoint for generation (GPT-4o officially, or any working chat endpoint). The type-aware prompt branches are proven only on DeepSeek; the official generation path has no working chat endpoint (no LLM chat en |
| A6.4 | Exit-proof: ERB-500 through the engine product path | slice (combined half now interim-measured, exit bar not met) | none | **Endpoint unblocked + combined half measured at interim** (2026-07-04): with the proxy Gemma chat endpoint, the committed b8 `erb-submission/answers.jsonl` combined score is now measured (A6.1 cross-check) — **46.71 (Gemma judge) / 47.74 (gemini)**, doc recall **55.71%**. Both are **below** the A6.4 exit bar (combined **>=60**, recall **>=75%@10**), and the master-plan "~62-68 within days" projection is **not** borne out at interim. So A6.4's *measurement* is unblocked but its *exit narrative is not satisfied*: closing it needs real retrieval/answer quality gains to actually reach >=60 (A-track levers) and/or the official gpt-5.4 judge — not merely more endpoint access. Honest measured gap now recorded; see docs/archive/GEMMA_BOUNDED_INTERIM_RUNS.md §A6.1. |
| C2-1 | KMS/HSM signing-key custody runbook + intake | slice | none | Real cloud-KMS/HSM operator evidence (external command-signer, RECEIPT_KMS_HSM_EXPECTED_PUBLIC_KEY_HEX, published anchor). scripts/receipt_kms_hsm_custody_check.py + receipt_production_evidence_preflight.py + evidence_origin.py already exis |
| C2-2 | External compliance certification + immutability evidence | not-started | none | Real external reviewer + immutability attestation (redacted_external_report / immutability_attestation with non-local refs). scripts/compliance_boundary_check.py exists; needs a real report, not fixtures. |
| C2-3 | Deploy external transparency witnesses + monitors (live anti-equivocat | slice | none | >=2 independent witness mirrors (each own Ed25519 key) + >=2 independent HTTPS monitors + continuous >=7-day gap-free SLO window. Engine machinery landed (accountability/transparency_witness.rs, transparency_quorum.rs, transparency_slo.rs + |
| C2-4 | Flip strict production claim gate + upgrade claims | not-started | none | Depends on C2-1/C2-2/C2-3 operator evidence existing. Until then receipt-production-ready-check stays advisory and claims are capped at 'signed receipt (beta, local trust anchor, not externally witnessed)'. |
| F2.1 | ERB gpt-5.4 re-judge: target + ready/blocked gate | landed (gate; metered rejudge run remains key-gated) | `erb-official-rejudge-ready-check` | The phase's deliverable is the **target + ready/blocked gate**, and it is offline-complete: `scripts/enterprise_rag_bench/check_official_rejudge.py` + `fixtures/benchmarks/erb/official_rejudge_target.v1.json` pin the judge of record (gpt-5.4) and the `answers.jsonl` SHA-256, verify the committed answers still match that SHA **before any tokens are spent** (a drifted file hard-FAILs), then report `ready` (judge key present) or `blocked` (no key → exit 0, no tokens, *not* a failure — the gate doing its job). Self-test covers ready/blocked/integrity + that committed answers match the recorded SHA. In the benchmark-validation lane (36 gates, bidirectional). The actual token-spending re-judge run against gpt-5.4 remains gated on `ENTERPRISE_RAG_BENCH_REJUDGE_API_KEY`. |
| F3.4 | LoCoMo: retrieval regression gate [landed] + first QA end-to-end [key-gated] | landed (retrieval half; QA half key-gated) | `locomo-retrieval-regression-check` | Retrieval half landed: `scripts/locomo/retrieval_regression_gate.py` + `fixtures/benchmarks/locomo/{reference,mini_retrieval_log,degraded_retrieval_log}.jsonl` + `retrieval_baseline_v1.json` (locked hit@1 0.3199 / hit@10 0.6312) + `mini_baseline_v1.json`. Implements hit@1/hit@10, fails on a drop >0.01 vs baseline; fast lane replays the mini log, locks the committed baseline, and catches a degraded log; `--log/--baseline` gate a real LoCoMo replay. Same self-tested pattern as F3.1/F3.3. In the benchmark-validation lane + `validation-nightly` (37 gates). QA half (`run_qa.py`/`check_qa_evidence.py`) needs an OpenAI-compatible reader endpoint and stays key-gated. |
| F4.3 | Competitor snapshot harness + offline matrix (absorbs TE4.2) | not-started | none | One-time capture requires an operator machine with docker (pinned postgres+pgvector + OPA thin-wrapper). The scoring half is offline once snapshots are captured. |
| C2-1 | Operator KMS/HSM custody evidence runbook + intake (production-receipt | not-started | none | Requires real external signer custody (cloud KMS or HSM) + a real published trust anchor; validators (receipt_kms_hsm_custody_check.py, receipt_production_evidence_preflight.py) deliberately reject fixtures/target/loopback/self-attested anc |
| C2-2 | External compliance certification + immutability evidence (production- | not-started | none | Requires engaging a real external compliance reviewer producing a redacted_external_report + immutability_attestation satisfying compliance_boundary_check.py's closed v1 schema. External human/procurement dependency absent from the environm |
| C2-3 | Deploy real external transparency witnesses + monitors (anti-equivocat | not-started | none | Requires standing up ≥2 independent witness mirrors (each own Ed25519 key) + ≥2 independent HTTPS monitors + a continuous ≥7-day gap-free SLO window. The transparency code exists and is gated on fixture-shaped evidence; real independent inf |
| C2-4 | Flip the strict production-claim gate + claims upgrade | not-started | none | Gated on C2-1/C2-2/C2-3 real evidence, which is all external. receipt-production-ready-check strictly requires production_ready=true through the production preflight. |
| F04-B6.4 | F04 agent transactions — promotion to stable (flags default-on) | not-started | none | Acceptance is a CALENDAR requirement: two consecutive green release-check runs with B1.3/B6.1/B6.3 gates, with run-ids recorded. Cannot be satisfied in a single cycle — slack is deliberately placed in Phase 4. |

## Large-scope / frozen (multi-week or frozen by the plan) (5)

F01 tiered-storage productization is multi-week; F02/F03 (replication/consensus) and F09 (managed cloud) are frozen v1.0 non-goals; A8.3/F4.4 are explicitly cut to post-v1.

| Phase | Title | State | Golden | Next action / blocker |
| --- | --- | --- | --- | --- |
| A8.3 | Adapter conformance (PDF/OCR contracts) -- deferred post-v1.0 | not-started | none | Explicitly cut/deferred to post-v1.0 by the plan's cut-list; not in scope for this cycle. |
| F4.4 | Signed leaderboard-pack (DEFERRED / cut) | not-started | none | Explicitly cut to the tail of Phase 4/5 by the plan (principle 7); non-critical, only on release-owner request. |
| F01 | F01 tiered storage v2 — flag-prototype → production (zstd blocks, form | slice | low | No dedicated productization task exists anywhere in the master plan — F01 is described only in section 6 and the D5 current-state note. It is flag-gated with a `ZstdReserved` placeholder (options.rs:57-65); non-goals in docs/TIERED_STORAGE_ |
| F02-F03 | F02 replication / F03 consensus — FROZEN (Raft = passive state machine | slice | low | Frozen by the project's own criteria. Raft is a full set of passive state machines (election/consensus/joint-consensus/snapshot/TCP transport) with NO driver: no election timer, no heartbeat loop; current_term/voted_for not persisted (real  |
| F09 | F09 managed cloud — FROZEN | not-started | none | Frozen; explicitly a v1.0 non-goal. All reports keep managed_cloud_ready=false. |

## Execution log + corrections (this cycle)

**2026-07-04 readiness verification of the external-blocked set.** To confirm the
"one run when the external resource arrives" claim is real (gates *ready*, not
silently *broken*), every preflight/readiness gate for the blocked-external phases
was executed. Result: **7 gates cleanly report ready/blocked (exit 0)** —
`receipt-kms-hsm-custody-check` (C2-1), `compliance-boundary-check` (C2-2),
`transparency-witness-check` / `-witness-quorum-check` / `-slo-check` /
`-consistency-check` (C2-3), `erb-official-rejudge-ready-check` (F2.1/official
judge). **2 gates correctly *strict-fail*** — `receipt-production-ready-check` +
`receipt-production-evidence-preflight-check` demand real
`COMPLIANCE_CERTIFICATION_EVIDENCE` / KMS-HSM custody and refuse to pass without
it (`--require-production-ready`); they are **correctly excluded from CI** so the
required lanes stay green. That refusal is the accountability design *working*: the
strict production-claim gate (C2-4) **cannot be flipped without real external
evidence** — the system structurally prevents fabricating production-readiness. No
gate is broken; the remainder is verifiably external-only.

**2026-07-04 reconciliation audit + interim runs.** A code-vs-tracker audit found
the top **"Completable now"** table's *State* column heavily **stale** — it still
shows ~20 rows as `not-started` that this same Execution-log records as landed, and
that a direct spot-check confirmed implemented + gated: **C4-2**
(`canonical-jcs-cross-language-check` green), **F2.2** (`erb-compare-runs` self-test
green), **F5.1** (`results-page-check` green), **F4.3** (`aab-snapshot-matrix` green),
the 9-gate `validation-nightly` aggregate green, **A1.2**
(`retrieval_rank::bm25_corpus_tests` 2/2, incl. `no_corpus_stats_is_byte_identical_to_pool_local`),
**A8.1/A8.2** (`ingestion::chunking::structured_tests` 8/8), **F06**
(`semantic_compression` 3/3), **C4-1** (`receipt-emission-budget-check` wired),
**A5.1** (`build_ltr_corpus.py` present). Treat the Execution-log + Honest-boundary
below as authoritative over the table's *State* cells until a bulk sync lands.
Newly done **this session** (evidence committed): **A6.3** hybrid retrieval closed
(recall_all@10 0.9523, DoD met — deterministic metric); and, using the now-working
LiteLLM proxy Gemma, the full-scale **interim** LLM-judged runs — **A6.2** (500,
overall 0.682), **F3.4-QA** (1,986, overall 0.592), **A6.1** (ERB judge cross-check,
500 → 46.71 combined vs the recorded gemini 47.74, i.e. judge-agnostic), and **A6.4**'s
combined half now measured (46.71, below the >=60 exit bar). All `leaderboard_comparable=false`
(the official gpt-5.4/GPT-4o judge is budget-blocked). Artifacts under
docs/archive/GEMMA_BOUNDED_INTERIM_RUNS.md + docs/archive/gemma_interim/.

Landed this cycle under gates (14 phases): **A1.2** (corpus-BM25 IDF), **A1.3**
(cosine-single-implementation allowlist), **A1.4** (`--candidate-pool` default
512), **A5.1** (offline LTR corpus builder + leak-free split), **A8.1**
(structure-aware chunking), **A8.2** (table-row cells), **C3-1** (ranking-change
protocol doc), **C3-4** (agent-cell-id.v2 NO-GO ADR), **F1.1** (benchmark_report.v1
schema freeze), **F4.1** (aab_run.v1 schema freeze), **F4.2** (AAB-mini six-axis
scorer), **F5.1** (machine-verifiable `RESULTS.md`), plus the phase-status map and
the F3.2/C3-2 already-landed corrections. A8.1/A8.2/A5.1 are complete engine/
tooling features; A1.2 is a golden-safe slice pending engine-path wiring.

Extended sweep (39 phases this cycle) — added on top of the six above: **A5.1**
(LTR corpus builder), **A8.1/A8.2** (structure + table chunking), **F1.1/F4.1/F4.2**
(benchmark + AAB schemas/scorer), and the **entire AQL-surface cluster golden-safe**
— `USING DIVERSITY` (A5/A7.3), `USING RERANK` (A7.2), `SUPPRESS SUPERSEDED` (A4.2),
`RECENCY WINDOW` (A4.1) — plus **A3.2** (ANN sparse-fallback ratio in telemetry),
**A5.2-training** (learned-ranker trainer + held-out lift), **C4-1** (p99
receipt-emission budget), **C4-3** (anti-absorption proof wired into nightly), and
**F5.2/F5.3** (benchmark floor gate + nightly validation job).

**Benchmark-honesty cluster closed (F1.2/F1.3/F2.2).** On top of F1.1's schema and
F5.1's `RESULTS.md`, the final three interlock into one anti-overclaim system:
**F1.2** — `fixtures/benchmarks/registry/*.json`, one immutable snapshot per
benchmark whose ERB numbers are machine-verified against
`erb-submission/official_results.json` (drift fails CI, proven by a negative test)
and where no entry may claim `leaderboard_official` without an official judge
(LoCoMo/MultiHop honestly `no_headline_claimed`); **F2.2** — the run comparator
that *refuses* a cross-judge delta (`refused_cross_judge_comparison`, no delta
leaked); **F1.3** — the machine-readable CI lane map (`lanes.v1.json`, 29 gates /
3 lanes) whose audit fails if any gate is defined-but-unscheduled or
scheduled-but-unmapped, checked **bidirectionally** against the real
`nightly.yml`. Every one of these is dependency-free, deterministic, and wired
into the nightly `benchmark-validation` job.

**Track-B agent-database cluster (32 phases this cycle).** Beyond the
benchmark-honesty system, the golden-safe, default-off engine features of the
F06/F08 tracks landed natively: **F06-B4.1** `memory_class` (pure Episodic/
Semantic classifier), **F06-B4.2** `semantic_compression_candidates` (the
read-only selection half the existing commit half consumes — deterministic,
freshness-gated, subtype-grouped), **F06-B4.5-unfold** `compression_sources`
(fail-closed provenance resolver, the `/v1/memory/cell/{id}/sources` core), and
**F08-B6.2** `require_seq_visible` (read-after-seq enforcement with a typed
`SequenceNotVisible` error), **F04-B1.3** the persistent idempotency ledger
(reserved `0xb` cells, content-addressed with collision-safe linear probing,
replay/reuse/cross-restart tested), and **F08-B6.1** the durable handoff-ledger
(reserved `0xc` cells, auditable read-back). All behind default-off flags, so no default
behaviour or goldens move; each gated (`memory-consolidation-check`,
`read-after-seq-check`) with engine API-freeze + fmt + clippy clean.

**F04-B6.3 completed end-to-end in-session (39-phase cycle).** What was first
deferred as "fresh-context, blast-radius" surface work was dissolved layer by
layer, each verified: the **wire contract** (`cortex-api-types`), **both live
server routes** (`POST /v1/transactions` — a conflict is a `200` body, not a
409-taxonomy change; `POST /v1/handoff` — target view via `load_agent_view`),
and **all four SDK clients** — Rust sync + async (`cargo test`), Python
(`pytest`, capture-opener), and TypeScript (`node --experimental-strip-types`
type-smoke + `node --test`, rebuilt ESM/CJS bundles). Every layer is behind the
default-off `agent_transactions` flag. The general lesson of this cycle: nearly
every "needs a fresh session" boundary, when actually traced, turned out to be
additive and testable in-session — B1.3, B6.1, both routes, and all four SDKs
were done here, not deferred.

The B4.5 retire half (TTL-demote) and the receipt-visible parts of A3.3/C3-5
remain golden-rebaseline (signed canonical bytes). **B4.4** (MCP consolidation
worker) is now *pattern-unblocked* — the same server-endpoint → SDK → client
chain just proven for B6.3 applies (its engine halves B4.2/B4.3 exist) — but it
is a full new chain (a `/v1/memory/consolidate` plan+commit endpoint + SDK +
MCP tool), not a one-file change.

**Key reclassification:** the AQL-surface bucket was *not* a golden-rebaseline. The
canonical receipt hashes the *resulting pack* (`CONTEXT_PACK_HASHED_FIELDS`), not
the query's option flags, so a default-off AQL option is byte-identical when unused
— verified by pack-determinism + public-API-freeze. That collapsed the biggest
"golden-rebaseline" sub-bucket into completable-now, and it is now done.

### Honest boundary of what genuinely remains

1. ~~**Large cross-crate surface work** — B6.3 and B4.4~~ **BOTH DONE.** B6.3
   (`/v1/transactions` + `/v1/handoff` HTTP/SDK) completed in-session (see the F04-B6.3
   log entry). B4.4 (MCP consolidation worker) is **confirmed landed** (2026-07-04
   audit): `consolidate_plan`/`consolidate_commit` MCP tools registered in
   `cortex-mcp/src/server.rs`, wired through `sdk_executor` to the engine's
   `commit_semantic_memory_compression` path, with a `/v1/memory/consolidate` server
   route, the `calls_consolidate_tools` test, and gates `memory-consolidation-check`
   + `memory-consolidate-route-check`. **No large cross-crate feature remains
   implementable** — the completable-now backlog is exhausted.
2. **Run-dependent** — ~~A6.3 (LME hybrid)~~ **DONE 2026-07-04** (full 500-question
   run, recall_all@10 0.9523, DoD met — see the A6.3 table row). A5.2-serving still
   needs a real corpus for non-degenerate weights.
3. ~~**Blocked on a factual decision (the user's)** — F2.0 judge-of-record~~
   **RESOLVED / landed** (`judge-of-record-check`): gpt-5.4 official + gemini-3.5-flash
   interim are encoded in docs/PUBLIC_CLAIMS_POLICY.md + docs/BENCHMARK_EVIDENCE.md;
   the "Gemini judge of record" in RESULTS.md is the *interim* record. No open user
   decision here.
4. ~~**Signed-golden regen**~~ — **FULLY DISSOLVED 2026-07-04**: every receipt-option
   surface landed via the **default-off / additive-`Option` pattern** (like **A5.2**),
   byte-identical when unused, so **no golden re-baseline was needed** — **A3.3
   receipt-visibility** (`serving_epoch: None` default,
   `serving_epoch_is_additive_and_golden_safe` green), **F08-B6.1** (durable
   handoff-ledger, reserved `0xc` cells), the **A4.x/A7.x receipt-option surfaces**
   (default-off AQL options; the receipt hashes the *resulting pack*, not the option
   flags), and now **C3-5-embref** — the one item previously thought to need a
   canonical-byte change. Rather than an in-place per-cell canonical bump, the store
   embedding *profile* identity was promoted into the **determinism input** as an
   additive `Option` (the `serving_epoch` template), gated default-off
   (`EmbeddingRefReceiptOptions`) → `None` on the keyword/default path adds no key, so
   committed goldens verify byte-for-byte (`embedding_ref_is_additive_and_golden_safe`,
   accountability-receipt-check + canonical-jcs-cross-language-check green). **No
   signed-golden task remains** — the whole "needs regeneration tooling" bucket is empty.
5. **Blocked-external** (15) — **partially unblocked 2026-07-04**: a working LLM chat
   endpoint now exists (the LiteLLM proxy Gemma, `google/gemma-4-31B-it`, via
   `.env` `CORTEXDB_EMBEDDING_*`), so the **interim** LLM-judged halves are now done
   at full scale — **A6.2** (LongMemEval type-aware, 500, overall 0.682 / preference
   0.767), **F3.4-QA** (LoCoMo, 1,986, overall 0.592 / adversarial 0.960), **A6.1**
   (ERB judge cross-check, 500, 46.71 vs gemini 47.74 — judge-agnostic), and
   **A6.4**'s combined half is now measured (46.71, below the >=60 exit bar). What
   genuinely remains external: the **leaderboard-official judge** (GPT-4o / gpt-5.4,
   no budget) to convert those interims to DoD numbers, plus a real KMS/HSM operator +
   published anchor, external compliance reviewers, and >=2 independent transparency
   witnesses/monitors. The judge is budget-blocked; the rest are operator/credential
   actions, **not code**.
6. **Frozen / cut** (5) — F02/F03/F09 v1.0 non-goals; A8.3/F4.4 cut; F01 multi-week.

So the clean + golden-safe in-session bucket is done (30 phases this cycle). What is
left needs one focused engine session (1), a multi-hour run (2), a user decision
(3), regeneration tooling (4), external operators/credentials (5), or is a declared
non-goal (6).

Corrections found while executing — several "completable" rows were **already
landed** (the repo is far past the plan's baseline), so the true remaining count
is smaller than the inventory implies:

- **F3.2** ERB per-type regression — already wired: `erb-category-regression-check`
  runs in the main `check:` target.
- **C3-2** receipt-consumption schema freeze — already gated:
  `accountability-receipt-schema-check`.
- **C3-1 / A1.3** enforcement machinery was already present; only the prose /
  allowlist guard were missing (now landed).

The genuinely net-new remaining completable work is a smaller set of **medium-large
features that each need a run or deep engine work** — A5.1 (offline LTR corpus),
A6.3 (LME hybrid: embedded index + A/B run), A8.1 (structure-aware chunking),
A8.2 (table-row cells) — plus benchmark-infra polish. These are multi-session,
not quick wins.

## How this closes

The completable-now backlog lands incrementally under gates. The golden-rebaseline
cluster lands via the C3-5 procedure ([canonical-schema-field-binding](../mk/core-contracts.mk)).
The blocked-external and frozen phases are **not** completable by code in this
environment — they require operator credentials/endpoints or are deliberate v1.0
non-goals; they are recorded here so "done" is never rounded up over them.

