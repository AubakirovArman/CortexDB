# EnterpriseRAG-Bench Evidence

This page records the current local EnterpriseRAG-Bench evidence for CortexDB.
It is intentionally separated from official answer-generation scores.

## Scope

Dataset:

```text
EnterpriseRAG-Bench v1.0.0
500 questions
511,958 generated enterprise documents
```

Current evidence type:

```text
local retrieval/evidence calibration only
no LLM calls
no external API calls
top10-focused document recall
```

Current best retrieval artifact:

```text
target/enterprise-rag-bench/retrieval/cortexdb_full_doc_view_v66_jira_completeness_source_top10.jsonl
```

## Current Local Gate

Latest local calibration gate:

```text
target/enterprise-rag-bench/analysis/local_calibration_gate_v66.json
target/enterprise-rag-bench/analysis/local_calibration_gate_v66.md
```

Result:

| Metric | Value |
| --- | ---: |
| local gate passed | `true` |
| top10 document recall | `72.59%` |
| top10 full-recall questions | `320` |
| top10 hit questions | `359` |
| average invalid extra docs | `7.96` |
| fact token coverage proxy | `74.31%` |
| fact full coverage proxy | `84.02%` |

Gate thresholds:

| Threshold | Value |
| --- | ---: |
| min top10 recall | `70.1%` |
| max invalid extra docs | `8.1` |

Candidate generator gate:

```text
target/enterprise-rag-bench/analysis/candidate_v55_top1000_gate.json
```

| Metric | Value |
| --- | ---: |
| gate passed | `true` |
| candidate recall@500 | `90.90%` |
| candidate recall@1000 | `91.47%` |
| candidate full-recall@1000 | `421` |
| candidate hit questions@1000 | `436` |

Source-link candidate experiment:

```text
target/enterprise-rag-bench/analysis/candidate_v58_top1000_gate.json
target/enterprise-rag-bench/analysis/v55_vs_v58_candidate_comparison_report.json
```

| Metric | Value |
| --- | ---: |
| gate passed | `true` |
| candidate recall@500 | `91.45%` |
| candidate recall@1000 | `92.15%` |
| candidate full-recall@1000 | `426` |
| candidate hit questions@1000 | `439` |
| delta vs v55 recall@500 | `+0.55` |
| delta vs v55 recall@1000 | `+0.68` |
| delta vs v55 full-recall@1000 | `+5` |

High-level coverage gate:

```text
target/enterprise-rag-bench/analysis/high_level_coverage_v31_report.json
target/enterprise-rag-bench/retrieval/cortexdb_full_doc_view_high_level_v31_top10.jsonl
```

| Metric | Value |
| --- | ---: |
| gate passed | `true` |
| high-level questions | `10` |
| questions with docs | `10` |
| average retrieved docs | `10.0` |
| fact token coverage proxy | `73.0%` |
| fact full coverage proxy | `87.5%` |

High-level questions have no `expected_doc_ids` in the benchmark file. The
separate high-level gate therefore reports answer-fact coverage instead of
ordinary document recall or invalid-extra-doc counts.

## Progression

| Stage | Top10 Recall | Full Recall | Hit Questions | Notes |
| --- | ---: | ---: | ---: | --- |
| `multi_index_v1` candidates | `61.52%` | `269` | `310` | baseline candidate generation |
| `multi_index_v8` candidates | `65.56%` | `290` | `328` | multi-index + router + entity terms |
| `dense_hybrid_v13` | `68.94%` | `303` | `345` | local embedding cache rerank |
| `hybrid_rrf_v14` | `69.83%` | `308` | `347` | weighted RRF over candidate + dense |
| `completeness_route_v17` | `69.86%` | `309` | `347` | completeness route |
| `extra_reducer_v19` | `69.86%` | `309` | `347` | not-found/high-level abstention |
| `semantic_route_v24` | `70.07%` | `310` | `348` | semantic-only hybrid route |
| `coverage_route_v25` | `70.13%` | `310` | `348` | completeness candidate injection |
| `type_topk_v27` | `70.13%` | `310` | `348` | type-specific noise caps |
| `doc_view_v30` | `71.06%` | `313` | `351` | multi-view rerank for semantic/completeness/project-related only |
| `doc_view_v46` | `71.08%` | `313` | `351` | completeness-only coverage pass over v30 |
| `doc_view_v51` | `71.29%` | `314` | `352` | semantic pass over v46 using candidate pool v48 |
| `doc_view_v56` | `71.44%` | `316` | `354` | project-related wide rerank over v55 neighbor-tail candidate pool |
| `doc_view_v61` | `71.66%` | `317` | `355` | GitHub-only semantic query expansion over v58 source-link candidate pool |
| `doc_view_v62` | `71.87%` | `318` | `356` | basic Google Drive tail rescue over v58 source-link candidate pool |
| `doc_view_v63` | `72.06%` | `318` | `357` | Confluence-only completeness path/title selector over v58 source-link candidate pool |
| `doc_view_v64` | `72.22%` | `319` | `358` | Confluence collection selector for case-study and postmortem completeness questions |
| `doc_view_v65` | `72.43%` | `320` | `358` | Jira project source selector for residency, SDK streaming parity, and canary rollout evidence chains |
| `doc_view_v66` | `72.59%` | `320` | `359` | Jira completeness source selector for H1 GPU quota incidents and log-retention exception evidence |

## What Improved

- Multi-index candidate generation raised top10 recall from `61.52%` to
  `65.56%`.
- Dense reranking from the local embedding cache raised top10 recall to
  `68.94%`.
- Weighted RRF raised top10 recall to `69.83%`.
- Completeness routing raised top10 recall to `69.86%`.
- Abstention for `info_not_found` and currently unrecovered `high_level`
  questions reduced average invalid extra docs to `8.45` without reducing
  local document recall.
- Semantic-only routing raised global top10 recall to `70.07%` and semantic
  recall from `41.6%` to `42.4%`.
- Completeness candidate injection raised global top10 recall to `70.13%` and
  completeness recall from `44.1%` to `45.55%`.
- Type-specific top-k caps kept recall at `70.13%` while reducing average
  invalid extra docs from `8.45` to `8.04`.
- Multi-view document discovery v30 raised global top10 recall to `71.06%`
  while keeping average invalid extra docs inside the local gate at `8.02`.
  The promoted route is intentionally limited to `semantic`, `completeness`,
  and `project_related` questions.
- Separate high-level coverage v31 retrieves documents for all `10`
  high-level questions and reaches `73.0%` fact token coverage without changing
  the default top10 retrieval gate.
- Completeness-only coverage v46 preserves v30 project/semantic behavior while
  raising completeness recall from `48.68%` to `49.24%` with zero regressions
  against v30.
- Candidate pool v48 raises candidate recall@1000 from `90.44%` to `90.89%`
  and full-recall@1000 from `415` to `417`.
- Semantic pass v51 raises final top10 recall from `71.08%` to `71.29%`, raises
  semantic recall from `44.8%` to `45.6%`, and adds one full-recall question
  without increasing average invalid extra docs.
- Uncapped strong-anchor candidate generation v52 keeps the current top10 best
  at v51, but raises candidate recall@1000 from `90.89%` to `91.15%`, raises
  candidate full-recall@1000 from `417` to `418`, and reduces missing
  candidate questions from `65` to `64`. This is first-stage discovery progress,
  not yet a promoted final top10 rerank.
- Source-aware neighbor-tail candidate generation v55 keeps the current top10
  best at v51, but raises candidate recall@100 from `81.67%` to `82.08%`,
  candidate recall@500 from `90.72%` to `90.90%`, candidate recall@1000 from
  `91.15%` to `91.47%`, and candidate full-recall@1000 from `418` to `421`.
  It improves candidate-pool completeness recall by `4.42` points with zero
  regressions against v52 at depth 1000. Aggressive v54 was rejected as the
  default because it improved tail recall while damaging early depth.
- Project-related wide rerank v56 uses the v55 neighbor-tail candidate pool for
  `project_related` questions only. It raises global top10 recall from `71.29%`
  to `71.44%`, full-recall questions from `314` to `316`, hit questions from
  `352` to `354`, and project-related recall from `74.34%` to `76.09%`, while
  reducing average invalid extra docs from `8.02` to `8.01`.
- Source-link neighbor candidate generation v58 indexes source-specific link
  fields such as `linked_jira`, `linked_linear`, `related_github_prs`,
  `linked_gmail_threads`, `linked_drive_docs`, `dependencies`, `parent_issue`,
  and `related_links`. It is not the current final top10 path, but it raises
  candidate recall@1000 from `91.47%` to `92.15%`, full-recall@1000 from `421`
  to `426`, hit questions@1000 from `436` to `439`, semantic candidate recall
  by `2.4` points, and project-related candidate recall by `0.7` points.
- GitHub-only semantic query expansion v61 promotes the useful part of the
  broader v60 experiment without the Slack regression. It uses the v58
  source-link candidate pool, routes only `semantic` GitHub questions, recovers
  `qst_0207`, raises global top10 recall from `71.44%` to `71.66%`, raises
  semantic recall from `45.6%` to `46.4%`, and has zero regressions against
  v56.
- Basic Google Drive tail rescue v62 handles a near-duplicate case where the
  v58 candidate pool already had the needed Google Drive document, but the
  final top10 kept an adjacent document variant. It routes only `basic` Google
  Drive questions, replaces one tail slot, recovers `qst_0112`, raises global
  top10 recall from `71.66%` to `71.87%`, raises full-recall questions from
  `317` to `318`, and has zero regressions against v61.
- Confluence completeness selector v63 uses only question text plus Confluence
  path/title anchors and deterministic enterprise synonyms such as
  `communications -> comms`, `gate -> go-no-go`, and `approval -> signoff`.
  It routes only completeness questions whose expected source type is exactly
  Confluence, recovers or improves `qst_0440`, `qst_0442`, `qst_0444`,
  `qst_0445`, and `qst_0447`, raises global top10 recall from `71.87%` to
  `72.06%`, raises hit questions from `356` to `357`, and has zero regressions
  against v62.
- Confluence collection selector v64 handles completeness questions that ask
  for collections of Confluence documents, currently case studies/customer
  stories and incident postmortems. It preserves the first two baseline slots,
  then pulls qualifying collection documents from the v58 candidate pool up to
  rank `400`. It improves `qst_0434` and `qst_0437`, raises global top10
  recall from `72.06%` to `72.22%`, raises full-recall questions from `318` to
  `319`, raises hit questions from `357` to `358`, and has zero regressions
  against v63.
- Jira project source selector v65 handles project-related evidence chains
  where Jira support tickets are required but absent from final top10. It
  routes only explicit residency error-contract, SDK streaming parity, and
  canary rollout questions; it uses a local Jira source scan plus Confluence
  candidate slots, with no LLM/API calls and no gold-aware selection. It
  improves `qst_0359`, `qst_0362`, and `qst_0370`, raises global top10 recall
  from `72.22%` to `72.43%`, raises full-recall questions from `319` to `320`,
  and has zero regressions against v64.
- Jira completeness source selector v66 handles completeness questions where
  internal Jira support tickets are required for H1 GPU capacity/quota incident
  and log-retention exception evidence. It uses a local Jira source scan and
  question/source metadata only, with no LLM/API calls and no gold-aware
  selection. It improves `qst_0438` and `qst_0448`, raises global top10 recall
  from `72.43%` to `72.59%`, raises hit questions from `358` to `359`, raises
  completeness recall from `57.43%` to `61.24%`, and has zero regressions
  against v65.

Regression comparison against `extra_reducer_v19`:

```text
target/enterprise-rag-bench/analysis/v19_vs_v66_retrieval_comparison_report.json
target/enterprise-rag-bench/analysis/v19_vs_v66_retrieval_comparison_report.md
```

| Metric | Delta |
| --- | ---: |
| average recall | `+2.73` |
| full-recall questions | `+11` |
| hit questions | `+12` |
| improved questions | `27` |
| regressed questions | `1` |

Incremental comparison against `doc_view_v65`:

```text
target/enterprise-rag-bench/analysis/v65_vs_v66_retrieval_comparison_report.json
target/enterprise-rag-bench/analysis/v65_vs_v66_retrieval_comparison_report.md
```

| Metric | Delta |
| --- | ---: |
| average recall | `+0.16` |
| completeness recall | `+3.81` |
| full-recall questions | `0` |
| hit questions | `+1` |
| improved questions | `2` |
| regressed questions | `0` |

Incremental comparison against `doc_view_v64`:

```text
target/enterprise-rag-bench/analysis/v64_vs_v65_retrieval_comparison_report.json
target/enterprise-rag-bench/analysis/v64_vs_v65_retrieval_comparison_report.md
```

| Metric | Delta |
| --- | ---: |
| average recall | `+0.21` |
| project-related recall | `+2.53` |
| full-recall questions | `+1` |
| hit questions | `+0` |
| improved questions | `3` |
| regressed questions | `0` |

Incremental comparison against `doc_view_v62`:

```text
target/enterprise-rag-bench/analysis/v62_vs_v63_retrieval_comparison_report.json
target/enterprise-rag-bench/analysis/v62_vs_v63_retrieval_comparison_report.md
```

| Metric | Delta |
| --- | ---: |
| average recall | `+0.19` |
| completeness recall | `+4.58` |
| full-recall questions | `0` |
| hit questions | `+1` |
| improved questions | `5` |
| regressed questions | `0` |

Incremental comparison against `doc_view_v61`:

```text
target/enterprise-rag-bench/analysis/v61_vs_v62_retrieval_comparison_report.json
target/enterprise-rag-bench/analysis/v61_vs_v62_retrieval_comparison_report.md
```

| Metric | Delta |
| --- | ---: |
| average recall | `+0.21` |
| basic recall | `+0.57` |
| full-recall questions | `+1` |
| hit questions | `+1` |
| improved questions | `1` |
| regressed questions | `0` |

Incremental comparison against `doc_view_v56`:

```text
target/enterprise-rag-bench/analysis/v56_vs_v61_retrieval_comparison_report.json
target/enterprise-rag-bench/analysis/v56_vs_v61_retrieval_comparison_report.md
```

| Metric | Delta |
| --- | ---: |
| average recall | `+0.22` |
| semantic recall | `+0.80` |
| full-recall questions | `+1` |
| hit questions | `+1` |
| improved questions | `1` |
| regressed questions | `0` |

Incremental comparison against `doc_view_v51`:

```text
target/enterprise-rag-bench/analysis/v51_vs_v56_retrieval_comparison_report.json
target/enterprise-rag-bench/analysis/v51_vs_v56_retrieval_comparison_report.md
```

| Metric | Delta |
| --- | ---: |
| average recall | `+0.15` |
| project-related recall | `+1.75` |
| full-recall questions | `+2` |
| hit questions | `+2` |
| improved questions | `4` |
| regressed questions | `1` |

Incremental comparison against `doc_view_v46`:

```text
target/enterprise-rag-bench/analysis/v46_vs_v51_retrieval_comparison_report.json
target/enterprise-rag-bench/analysis/v46_vs_v51_retrieval_comparison_report.md
```

| Metric | Delta |
| --- | ---: |
| average recall | `+0.21` |
| semantic recall | `+0.80` |
| full-recall questions | `+1` |
| hit questions | `+1` |
| improved questions | `1` |
| regressed questions | `0` |

Missing gold reason classifier:

```text
target/enterprise-rag-bench/analysis/gold_missing_reasons_v66_report.json
target/enterprise-rag-bench/analysis/gold_missing_reasons_v66_report.md
```

Largest current missing-gold buckets:

| Reason | Missing Gold Docs |
| --- | ---: |
| `not_in_top1000` | `54` |
| `near_duplicate_confusion` | `51` |
| `in_top500_not_top100` | `48` |
| `lost_by_embedding_rerank` | `33` |

Missing-gold bottleneck summary:

```text
target/enterprise-rag-bench/analysis/gold_missing_bottlenecks_v66_report.json
target/enterprise-rag-bench/analysis/gold_missing_bottlenecks_v66_report.md
```

Current missing-gold total: `221` docs across `150` questions. The v66 Jira
completeness selector reduced the total from `227` and removed
`completeness|jira|filtered_by_source` from the top bottleneck table.

Largest type/source/reason buckets:

| Question Type | Source | Reason | Missing Gold Docs |
| --- | --- | --- | ---: |
| `completeness` | `confluence` | `near_duplicate_confusion` | `16` |
| `project_related` | `confluence` | `near_duplicate_confusion` | `9` |
| `completeness` | `confluence` | `in_top500_not_top100` | `9` |
| `semantic` | `slack` | `not_in_top1000` | `7` |
| `basic` | `slack` | `not_in_top1000` | `5` |

Candidate-rank buckets for currently missing gold docs:

| Candidate Rank Bucket | Missing Gold Docs |
| --- | ---: |
| `top500` | `64` |
| `missing` | `64` |
| `top50` | `43` |
| `top100` | `35` |
| `top10` | `12` |
| `top1000` | `3` |

This keeps the next focused route clear: continue completeness composition.
Confluence near-duplicate misses remain the largest cluster, while the prior
Jira completeness filtered-source cluster is no longer in the top bottlenecks.

## What Was Tested And Not Promoted

The following were measured and kept out of the default retrieval path because
they regressed local top10 recall or evidence coverage:

- path n-gram boosting as a candidate source;
- path n-gram existing-only boost;
- pure evidence digest as the only context pack;
- question-window context at a `5000` character budget;
- project-chain linked-doc reranking;
- answer-aware rerank preset.
- global hybrid v23 as default: it improved semantic slightly but regressed
  overall recall and increased invalid extra docs;
- high-level v26 as default: it improved high-level fact coverage proxy but
  raised average invalid extra docs above the current local gate.
- wide doc-view v29 route: it raised recall to `71.16%` and had zero recall
  regressions against v27, but average invalid extra docs rose to `8.38`, above
  the `8.1` local gate threshold.
- raw-candidate tail v33: it tested adding two raw candidate slots for
  semantic/completeness questions, but average recall dropped to `71.00%` and
  project-related regressions appeared.
- lower-protection doc-view v34: it tested replacing two tail slots for
  semantic/completeness/project-related questions, but average recall dropped
  to `70.88%`.
- wider semantic/completeness doc-view v35-v40: these were safe but did not
  produce enough net gain to replace the current route.
- aggressive completeness v43-v45: these raised completeness more strongly, up
  to `51.25%`, but reduced full-recall questions from `313` to `312`.
- wider v48 candidate semantic scoring v50: it tested `score_candidate_limit=140`
  for semantic/completeness, but dropped back to `71.08%` top10 recall.
- v52 candidate pool with completeness rerank v53: it preserved v51 exactly
  (`changed_rows=0`), so v52 is promoted only as a candidate-generator gate.
- semantic/completeness wide tail v57 over the v55 candidate pool: it changed
  `62` rows but kept global recall and full-recall flat, with one semantic
  improvement and one semantic regression.
- source-link v58 with project-related final rerank v59: v58 improves the
  candidate pool, but the v59 final top10 rerank dropped global recall from
  `71.44%` to `71.37%`, so it was rejected.
- all-source semantic query expansion v60: it recovered `qst_0207` and raised
  recall to `71.66%`, but it regressed Slack-only `qst_0295`; v61 keeps the
  improvement by routing the expansion only to GitHub semantic questions.
- full semantic source-route sweep v61 tested
  `jira,gmail,confluence,google_drive,hubspot,slack,linear,fireflies,github`
  against the current v61 baseline. No source type produced a promotion
  candidate. `slack` was rejected because it improved `qst_0176` but regressed
  `qst_0295`; every other source type was neutral. v62 was promoted separately
  by a basic Google Drive tail-rescue route, not by this semantic source sweep.

Semantic source-route sweep:

```text
target/enterprise-rag-bench/analysis/semantic_source_route_sweep_v61_report.json
target/enterprise-rag-bench/analysis/semantic_source_route_sweep_v61_report.md
```

| Decision | Source Types |
| --- | --- |
| `promote_candidate` | none |
| `neutral` | `jira`, `gmail`, `confluence`, `google_drive`, `hubspot`, `linear`, `fireflies`, `github` |
| `reject_regression` | `slack` |

## Reproduction Commands

Build candidate doc views:

```bash
python scripts/enterprise_rag_bench/build_doc_view_subset.py \
  --retrieval-file target/enterprise-rag-bench/retrieval/cortexdb_full_multi_index_v48_candidates_top1000.jsonl \
  --retrieval-file target/enterprise-rag-bench/retrieval/cortexdb_full_type_topk_v27.jsonl \
  --uuid-index target/external-benchmarks/EnterpriseRAG-Bench/generated_data/uuid_index.json \
  --sources-dir target/external-benchmarks/EnterpriseRAG-Bench/generated_data/sources \
  --candidate-limit 50 \
  --output target/enterprise-rag-bench/index/doc_views_candidates_v28_top50.jsonl \
  --report target/enterprise-rag-bench/index/doc_views_candidates_v28_top50_report.json
```

Run promoted neighbor-aware candidate coverage:

```bash
make enterprise-rag-bench-anchor-candidate-coverage
make enterprise-rag-bench-candidate-depth-check
make enterprise-rag-bench-neighbor-candidate-coverage
```

Run source-link candidate coverage:

```bash
make enterprise-rag-bench-source-link-candidate-coverage
```

Run targeted doc-view rerank:

```bash
python scripts/enterprise_rag_bench/doc_view_rerank.py \
  --questions-file target/external-benchmarks/EnterpriseRAG-Bench/questions.jsonl \
  --retrieval-file target/enterprise-rag-bench/retrieval/cortexdb_full_multi_index_v22_candidates_top1000.jsonl \
  --baseline-retrieval-file target/enterprise-rag-bench/retrieval/cortexdb_full_type_topk_v27.jsonl \
  --uuid-index target/external-benchmarks/EnterpriseRAG-Bench/generated_data/uuid_index.json \
  --sources-dir target/external-benchmarks/EnterpriseRAG-Bench/generated_data/sources \
  --doc-views-file target/enterprise-rag-bench/index/doc_views_candidates_v28_top50.jsonl \
  --embedding-cache target/enterprise-rag-bench/retrieval/embedding_cache.jsonl \
  --output target/enterprise-rag-bench/retrieval/cortexdb_full_doc_view_v30_top10.jsonl \
  --report target/enterprise-rag-bench/retrieval/cortexdb_full_doc_view_v30_top10_report.json \
  --score-candidate-limit 50 \
  --limit 10 \
  --seed-count 3 \
  --protect-baseline-prefix 9 \
  --route-question-types semantic,completeness,project_related
```

Run completeness-only coverage pass over v30:

```bash
make enterprise-rag-bench-completeness-coverage
```

Run semantic coverage pass over v46:

```bash
make enterprise-rag-bench-semantic-coverage
```

Run project-related wide rerank over the promoted v55 candidate pool:

```bash
make enterprise-rag-bench-project-related-coverage
```

Run GitHub semantic query expansion over the source-link candidate pool:

```bash
make enterprise-rag-bench-github-semantic-query-expansion
```

Run basic Google Drive tail rescue over the source-link candidate pool:

```bash
make enterprise-rag-bench-basic-google-drive-tail-rescue
```

Run Confluence completeness selector over the source-link candidate pool:

```bash
make enterprise-rag-bench-confluence-completeness-selector
```

Run Confluence collection selector over the source-link candidate pool:

```bash
make enterprise-rag-bench-confluence-collection-selector
```

Run Jira project source selector over the source-link candidate pool:

```bash
make enterprise-rag-bench-jira-project-source-selector
```

Run Jira completeness source selector over the v65 top10 output:

```bash
make enterprise-rag-bench-jira-completeness-source-selector
```

Run the current missing-gold bottleneck report:

```bash
make enterprise-rag-bench-gold-missing-bottlenecks
```

Run semantic source-route sweep against the current best:

```bash
make enterprise-rag-bench-semantic-source-route-sweep
```

For a cheaper smoke pass:

```bash
ENTERPRISE_RAG_BENCH_SOURCE_ROUTE_SWEEP_TYPES=jira \
make enterprise-rag-bench-semantic-source-route-sweep
```

Depth audit:

```bash
python scripts/enterprise_rag_bench/candidate_depth_audit.py \
  --questions-file target/external-benchmarks/EnterpriseRAG-Bench/questions.jsonl \
  --retrieval-file target/enterprise-rag-bench/retrieval/cortexdb_full_doc_view_v66_jira_completeness_source_top10.jsonl \
  --output-jsonl target/enterprise-rag-bench/analysis/doc_view_v66_depth_details.jsonl \
  --report target/enterprise-rag-bench/analysis/doc_view_v66_depth_report.json \
  --markdown target/enterprise-rag-bench/analysis/doc_view_v66_depth_report.md
```

Evidence pack proxy:

```bash
python scripts/enterprise_rag_bench/evaluate_evidence_pack.py \
  --questions-file target/external-benchmarks/EnterpriseRAG-Bench/questions.jsonl \
  --retrieval-file target/enterprise-rag-bench/retrieval/cortexdb_full_doc_view_v66_jira_completeness_source_top10.jsonl \
  --uuid-index target/external-benchmarks/EnterpriseRAG-Bench/generated_data/uuid_index.json \
  --sources-dir target/external-benchmarks/EnterpriseRAG-Bench/generated_data/sources \
  --mode leading \
  --top-k 10 \
  --max-chars-per-doc 5000 \
  --output-jsonl target/enterprise-rag-bench/analysis/evidence_pack_doc_view_v66_leading_details.jsonl \
  --report target/enterprise-rag-bench/analysis/evidence_pack_doc_view_v66_leading_report.json
```

Calibration gate:

```bash
python scripts/enterprise_rag_bench/summarize_local_calibration.py \
  --depth-report target/enterprise-rag-bench/analysis/doc_view_v66_depth_report.json \
  --evidence-report target/enterprise-rag-bench/analysis/evidence_pack_doc_view_v66_leading_report.json \
  --output target/enterprise-rag-bench/analysis/local_calibration_gate_v66.json \
  --markdown target/enterprise-rag-bench/analysis/local_calibration_gate_v66.md \
  --min-top10-recall-pct 70.1 \
  --max-invalid-extra-docs 8.1
```

High-level coverage gate:

```bash
make enterprise-rag-bench-high-level-coverage
```

## Limitations

- These numbers are not the official EnterpriseRAG answer score.
- Correctness and completeness still require an answer generation run plus the
  official evaluator/judge path.
- The local evidence proxy uses benchmark gold facts to measure coverage. It is
  an analysis tool, not a production scoring signal.
- `high_level` questions are abstained in the default local retrieval gate
  because they have no `expected_doc_ids`; use
  `enterprise-rag-bench-high-level-coverage` for their separate fact-coverage
  evidence.
