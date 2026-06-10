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

| Metric | Value |
| --- | ---: |
| **Avg correctness across the 10 categories** (leaderboard metric) | **44.0%** |
| Per-question correctness | 35.6% |
| Avg completeness | 38.8% |
| Combined correctness×completeness | 29.67 |

Per-category correctness: basic 42, semantic 16, intra_document 30,
project_related 22, constrained 30, conflicting_info 50, completeness 20,
miscellaneous 80, high_level 50, info_not_found 100.

### Important honesty note about the judge

The official evaluator's default judge is **`gpt-5.4`**
(`src/llm/openai_llm.py: LLM_MODEL_NAME`). Our OpenAI key only has access to
**`gpt-5.2`**, so we ran the official evaluator with `LLM_MODEL_NAME=gpt-5.2`.

Everything else is the official procedure unchanged (the official prompt
`ANSWER_WHOLISTIC_EVALUATION_PROMPT`, the 3-run consensus document-correction
flow, the official scoring). We verified the procedure is faithful: on a
50-question subset, our own local gpt-5.2 judge and the official evaluator with
gpt-5.2 produced **identical correctness (46.0% = 46.0%)** — i.e. the only
remaining variable between our number and a fully-official one is **gpt-5.2 vs
gpt-5.4** (adjacent versions of the same model). To finalize, re-run the
evaluator below with `LLM_MODEL_NAME=gpt-5.4`.

---

## 2. System under test

A **no-oracle** pipeline:

1. **CortexDB** (from-scratch Rust agent-native context database) ingests the
   511,958-document corpus and serves lexical (BM25) retrieval over its persisted
   index.
2. **Dense/hybrid candidate generation**: each document is embedded with
   `BAAI/bge-m3`; per question we take a wide lexical candidate pool and the
   dense (cosine) top-k, then fuse them with Reciprocal-Rank Fusion.
3. **Answer generation**: `google/gemma-4-31B-it` (open weights) answers each
   question from the retrieved context only.

The answer model is interchangeable; an ablation (gemma with no retrieval scores
~10% vs ~46% with CortexDB on the same judge) shows the retrieval — not the
answer model — drives the score.

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
- Two OpenAI-compatible HTTP endpoints (both open models, run them however you
  like — e.g. vLLM / TEI):
  - **Embeddings**: `BAAI/bge-m3` → set in a local `.env`:
    `CORTEXDB_EMBEDDING_URL`, `CORTEXDB_EMBEDDING_MODEL=BAAI/bge-m3`,
    `CORTEXDB_EMBEDDING_API_KEY`.
  - **Answer LLM**: `google/gemma-4-31B-it` →
    `VLLM_URL`, `VLLM_MODEL=google/gemma-4-31B-it`, `VLLM_API_KEY`.
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
  --reset-db --official-clean --retrieval-mode cached-lexical

# (2) embed the whole corpus with bge-m3 (resumable; ~12 GB output)
python3 scripts/enterprise_rag_bench/embed_corpus.py \
  --uuid-index $UUID --sources-dir $SRC \
  --output target/enterprise-rag-bench/embeddings/corpus_bge_m3.jsonl \
  --env-file .env --workers 16 --batch-size 32 \
  --log-file target/enterprise-rag-bench/embeddings/embed_corpus.log

# (3) full pipeline: lexical retrieve -> dense+RRF fuse -> gemma answers
#     (this wrapper runs the exact config used for the reported number)
SIZE=500 \
QUESTIONS_FILE=$BENCH/questions.jsonl \
RUN_LABEL=full500-dense-hybrid \
DB=$DB \
DENSE_TOP_K=100 TOP_K=50 DENSE_WEIGHT=1.0 LEX_WEIGHT=1.0 \
TOP_K_CONTEXT=8 \
ANSWER_PROVIDER=gemma JUDGE_PROVIDER=gemma \
ANSWER_WORKERS=6 \
bash scripts/enterprise_rag_bench/run_dense_hybrid_clean.sh
```

The submission answer file (`{question_id, answer, document_ids}`) is:

```
target/enterprise-rag-bench/official-clean/500/full500-dense-hybrid/answer-gemma/answers.jsonl
```

### Exact configuration of the reported run

| Stage | Setting |
| --- | --- |
| corpus | 511,958 docs ingested into CortexDB; 500,694 embedded with bge-m3 (97.9%) |
| lexical candidates | CortexDB BM25, top-50 |
| dense candidates | bge-m3 cosine, top-100 |
| fusion | Reciprocal-Rank Fusion, weights lexical=1.0 / dense=1.0, `rrf_k=10`, final top-50 |
| answer model | `google/gemma-4-31B-it`, temperature 0 |
| answer context | top-8 docs, `question-window-digest-ranked`, 2200 chars/doc, max 420 tokens |
| answer prompt | `official-clean-v1` (in `scripts/enterprise_rag_bench/answer_prompts.py`) |

---

## 6. Score with the official evaluator

Use the benchmark's own evaluator on the answers file. Set `LLM_API_KEY` (the
wrapper reads `LLM_API_KEY`, not `OPENAI_API_KEY`) and the judge model:

```bash
cd target/external-benchmarks/EnterpriseRAG-Bench
ANS=../../enterprise-rag-bench/official-clean/500/full500-dense-hybrid/answer-gemma/answers.jsonl

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

- **Non-determinism**: answer generation (gemma) and the LLM judge are
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
