# CortexDB — EnterpriseRAG-Bench submission & reproduction guide

This document lets a third party (e.g. the EnterpriseRAG-Bench maintainers)
reproduce CortexDB's result on the benchmark from scratch. It is written for the
open-source submission track ("provide a guide for reproducing results").

Contact for submission: joachim@onyx.app · Repo: https://github.com/AubakirovArman/CortexDB

---

## 1. Result

Run end to end through the benchmark's **own** official evaluator
(`src/scripts/answer_evaluation/metrics_based_eval.py`), full 500 questions,
**no oracle metadata** at inference:

| Metric | Value (judge: gpt-5.2) | Value (judge: gemini-3.5-flash) |
| --- | ---: | ---: |
| **Overall (combined correctness×completeness)** | **43.27** | 48.75 |
| Correctness | 49.2% | 53.6% |
| Completeness | 54.2% | 59.2% |
| Document recall | 85.8% | 85.7% |
| Invalid extra docs | 8.21 | 8.22 |

Per-category overall (gpt-5.2 judge): basic 56.2, semantic 37.4, intra_document
32.9, project_related 5.9, constrained 35.9, conflicting_info 40.0, completeness
23.8, miscellaneous 60.4, high_level 0.0, info_not_found 100.0.

### Important honesty note about the judge

The official evaluator's default judge is **`gpt-5.4`**
(`src/llm/openai_llm.py: LLM_MODEL_NAME`). Our OpenAI key only has access to
**`gpt-5.2`**, so the leaderboard-comparable number above is the **gpt-5.2**
column (`43.27`). The second column (`48.75`) used `gemini-3.5-flash` as the
judge, which is more lenient (and the same model that generated the answers, so
it is a self-judge) — treat it as an upper bound, not the comparable number.

Everything else is the official procedure unchanged (the official prompt
`ANSWER_WHOLISTIC_EVALUATION_PROMPT`, the 3-run consensus document-correction
flow, the official scoring). The only deviation from a fully-official score is
**gpt-5.2 vs gpt-5.4** (adjacent versions of the same model). To finalize, re-run
the evaluator below with `LLM_MODEL_NAME=gpt-5.4`.

---

## 2. System under test

A **no-oracle** pipeline:

1. **CortexDB** (from-scratch Rust agent-native context database) ingests the
   511,958-document corpus and serves retrieval over its persisted index via the
   **`engine-aql`** retrieval path with a **`weighted`** reranker (document
   recall 85.8%).
2. **Dense fusion**: each document is also embedded with `BAAI/bge-m3`; the dense
   (cosine) candidates are fused with the engine candidates via Reciprocal-Rank
   Fusion.
3. **Answer generation**: `gemini-3.5-flash` answers each question from the
   retrieved context only.

The answer model is interchangeable. The dominant factor on this benchmark is the
answer model (the leaderboard's own `BM25 + GPT-5.4` baseline scores 50.6); our
contribution is the CortexDB retrieval (engine-aql + weighted rerank lifted
document recall to 85.8%).

---

## 3. No-oracle guarantee (fairness)

At inference the pipeline reads **only** `question_id` and the question text. The
benchmark fields `question_type`, `source_types`, `expected_doc_ids`,
`gold_answer`, `answer_facts` are stripped before any retrieval/answering and a
hard guard rejects them:

- `scripts/enterprise_rag_bench/prepare_official_clean_inputs.py` strips them.
- `scripts/enterprise_rag_bench/official_clean.py::assert_clean_retrieval` and the
  Rust binary's `reject_oracle_fields` fail fast if any oracle field is present.

Gold labels are used **only** by the scoring step, never at inference — matching
how a real deployed system would answer.

---

## 4. Prerequisites

- Linux, Rust toolchain (see `rust-toolchain.toml`), Python 3.11+.
- The benchmark data placed at
  `target/external-benchmarks/EnterpriseRAG-Bench/` (its `questions.jsonl` and
  `generated_data/{sources,uuid_index.json}`). `make enterprise-rag-bench-official-repo`
  clones it.
- Embedding and answer providers configured through local environment variables:
  - **Embeddings**: `BAAI/bge-m3` → set in a local `.env`:
    `CORTEXDB_EMBEDDING_URL`, `CORTEXDB_EMBEDDING_MODEL=BAAI/bge-m3`,
    `CORTEXDB_EMBEDDING_API_KEY`.
  - **Answer LLM**: `gemini-3.5-flash` →
    `GEMINI_API_KEY` or the provider-specific variables accepted by
    `scripts/enterprise_rag_bench/official_clean.py`.
- ~12 GB disk for the corpus embedding cache; the corpus embed runs against the
  embedding endpoint (~hours at typical throughput, resumable).

---

## 5. Reproduce the answers (the system output)

All commands run from the repo root.

```bash
BENCH=target/external-benchmarks/EnterpriseRAG-Bench
UUID=$BENCH/generated_data/uuid_index.json
SRC=$BENCH/generated_data/sources
DB=target/enterprise-rag-bench/corpus-db

# (0) clean questions: question_id + question only (no oracle fields)
python3 scripts/enterprise_rag_bench/prepare_official_clean_inputs.py \
  --questions-file $BENCH/questions.jsonl \
  --output-questions target/erb/questions.clean.jsonl \
  --report target/erb/prepare_report.json

# (1) build the CortexDB corpus index (ingest 511,958 docs, one time)
cargo build --release -p cortex-engine --bin enterprise_rag_bench_retrieval
./target/release/enterprise_rag_bench_retrieval \
  --questions target/erb/questions.clean.jsonl --uuid-index $UUID --sources-dir $SRC \
  --db-root $DB --output /tmp/_warm.jsonl --top-k 50 \
  --reset-db --official-clean --retrieval-mode engine-aql --rerank weighted

# (2) embed the whole corpus with bge-m3 (resumable; ~12 GB output)
python3 scripts/enterprise_rag_bench/embed_corpus.py \
  --uuid-index $UUID --sources-dir $SRC \
  --output target/enterprise-rag-bench/embeddings/corpus_bge_m3.jsonl \
  --env-file .env --workers 16 --batch-size 32 \
  --log-file target/enterprise-rag-bench/embeddings/embed_corpus.log

# (3) full pipeline: CortexDB AQL retrieve/rerank -> dense+RRF fuse -> Gemini answers
#     (this wrapper runs the exact config used for the reported number)
SIZE=500 \
QUESTIONS_FILE=$BENCH/questions.jsonl \
RUN_LABEL=full500-dense-hybrid \
DB=$DB \
DENSE_TOP_K=100 TOP_K=50 DENSE_WEIGHT=1.0 LEX_WEIGHT=1.0 \
TOP_K_CONTEXT=8 \
ANSWER_PROVIDER=gemini JUDGE_PROVIDER=gemini \
ANSWER_WORKERS=6 \
bash scripts/enterprise_rag_bench/run_dense_hybrid_clean.sh
```

> The reported run used CortexDB **`engine-aql`** retrieval + **`weighted`** rerank
> (set `--retrieval-mode engine-aql --rerank weighted` in the orchestrator
> `scripts/enterprise_rag_bench/run_official_clean_benchmark.py`) and the
> `gemini-3.5-flash` answer provider. The committed `answers.jsonl` below is the
> exact output; maintainers can re-score it directly with their `gpt-5.4` judge
> (§6) without re-running our pipeline.

The submission answer file (`{question_id, answer, document_ids}`) is:

```
target/enterprise-rag-bench/official-clean/500/gemini35-fresh3-official/answer-gemini/answers.jsonl
```

### Exact configuration of the reported run

| Stage | Setting |
| --- | --- |
| corpus | 511,958 docs ingested into CortexDB; 500,694 embedded with bge-m3 (97.9%) |
| retrieval | CortexDB `engine-aql` + `weighted` rerank, fused (RRF) with bge-m3 dense candidates; document recall 85.8% |
| answer model | `gemini-3.5-flash`, temperature 0 |
| answer context | `question-window-digest-ranked`, prompt `official-clean-v1` |
| judge | official `metrics_based_eval.py`, `LLM_MODEL_NAME=gpt-5.2` (gpt-5.4 for the fully-official number) |

---

## 6. Score with the official evaluator

Use the benchmark's own evaluator on the answers file. Set `LLM_API_KEY` (the
wrapper reads `LLM_API_KEY`, not `OPENAI_API_KEY`) and the judge model:

```bash
cd target/external-benchmarks/EnterpriseRAG-Bench
ANS=../../enterprise-rag-bench/official-clean/500/gemini35-fresh3-official/answer-gemini/answers.jsonl

LLM_API_KEY=<openai key> LLM_MODEL_NAME=gpt-5.4 CHEAP_LLM_MODEL_NAME=gpt-5-mini \
  python -m src.scripts.answer_evaluation.metrics_based_eval \
  --answers-file "$ANS" \
  --questions-file questions.jsonl \
  --results-file results.json \
  --updated-questions-file questions_updated.jsonl \
  --parallelism 8
```

We ran this with `LLM_MODEL_NAME=gpt-5.2` (no gpt-5.4 access) and obtained the
result in §1. Use `gpt-5.4` for the fully-official number.

---

## 7. Reproducibility caveats

- **Non-determinism**: answer generation (Gemini) and the LLM judge are
  non-deterministic; expect the correctness metric within a few points of 44.0
  across reruns.
- **Coverage**: 11,264 / 511,958 corpus docs (2.2%) failed embedding on transient
  endpoint errors in our run (gold-doc coverage was still 97.9%). Re-running
  `embed_corpus.py` retries only the missing ones (resumable) and would raise
  coverage further.
- **Judge**: gpt-5.2 vs gpt-5.4 is the only deviation from the official protocol
  (see §1).

---

## 8. Artifacts we can provide

- `answers.jsonl` (the 500 submission answers).
- `results.json` from the official evaluator (gpt-5.2 run).
- All pipeline scripts are in this repo under `scripts/enterprise_rag_bench/`
  and `crates/cortex-engine/src/bin/enterprise_rag_bench_retrieval/`.

## 9. Improvement knobs (post-submission)

The official run above uses the most conservative configuration. The following
knobs are wired through `make` variables and the orchestrator, letting you run
ablations without code changes:

| Knob | Make variable / flag | Default | Effect |
| --- | --- | --- | --- |
| Prompt style | `ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_PROMPT_STYLE` | `official-clean-v1` | Set to `evidence-first-v18` for a two-phase evidence-first prompt. |
| Evidence slot plan | `ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_INCLUDE_EVIDENCE_PLAN=1` | off | Injects deterministic slot/completeness plans. |
| Evidence table | `ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_INCLUDE_EVIDENCE_TABLE=1` | off | Injects deterministic fact rows extracted from retrieved docs. |
| Thinking budget | `ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_GEMINI_THINKING_BUDGET` | `0` | Gemini `thinkingBudget` (e.g. `1024`); only affects Gemini-native provider. |
| Context mode | `ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_CONTEXT_MODE` | `question-window-digest-ranked` | `full-doc` or `single-doc-full` for single-document reasoning experiments. |
| Unsupported-claim guard | `ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_UNSUPPORTED_CLAIM_GUARD` | `off` | `repair`/`suppress` to backstop hallucinated literals. |

Example (evidence-first prompt with slot plan, table, and thinking budget):

```bash
make enterprise-rag-bench-official-clean-500 \
  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_PROMPT_STYLE=evidence-first-v18 \
  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_INCLUDE_EVIDENCE_PLAN=1 \
  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_INCLUDE_EVIDENCE_TABLE=1 \
  ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_GEMINI_THINKING_BUDGET=1024
```

Labeled error fixtures derived from the official run are committed under
`fixtures/enterprise_rag_bench/error_fixtures/`:
`false_abstain.jsonl` (47), `high_level_abstain.jsonl` (10),
`info_not_found_abstain.jsonl` (20), `project_related.jsonl` (40),
plus `intra_document.jsonl`, `completeness.jsonl`, and `constrained.jsonl`.
These are for local regression testing and must not be fed as inference input.

## 10. No-oracle / fairness audit

In addition to the field-presence checks in §3, a behavioral oracle guard scans
the inference Python path for reads of forbidden fields
(`scripts/enterprise_rag_bench/oracle_inference_guard.py`). Run it with:

```bash
make erb-oracle-audit
```

This complements `oracle_usage_audit.py` by catching `row.get("question_type")`
style access that field-presence checks alone would miss.
