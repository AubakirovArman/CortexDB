# Bounded Gemma interim runs — A6.2 + F3.4 QA (end-to-end proof, not official)

**Status: interim / diagnostic. `leaderboard_comparable: false`.** These numbers are
**not** the official A6.2 / F3.4 DoD results — they use an **interim Gemma judge**
(not the leaderboard-official GPT-4o for LongMemEval / gpt-5.4 for ERB) over a
**50-question balanced subset** (not the full 500 / 1,986). They are recorded as
honest evidence that the A6.2 and F3.4-QA harnesses run **end-to-end against a real
LLM on real retrieval data**, and to surface the per-type signal. Public claims go
only through the F5.1 page flow; these interim numbers are narrative, not a gate
threshold (an LLM judge is outside the agent's control).

## Setup (reproducible)

- **Date:** 2026-07-04.
- **Answerer + judge model:** `google/gemma-4-31B-it` via the LiteLLM proxy
  configured in `.env` as `CORTEXDB_EMBEDDING_URL` (chat route
  `<proxy>/v1/chat/completions`, same key as embeddings). The proxy also serves
  `gemma3-27b-32bit`, several Qwen models, and `openai/gpt-oss-120b`.
- **Why Gemma (not the on-disk DeepSeek key):** the `/mnt/.../.deepseek` key is
  invalid (HTTP 401); the LiteLLM proxy is the working chat endpoint. Chosen at the
  user's direction ("use gemma").
- **Generation harnesses (committed, self-tested):**
  `scripts/longmemeval/run_official_generation.py` (A6.2, type-aware) and
  `scripts/locomo/run_qa.py` (F3.4, category-aware).
- **Scorer:** `scripts/benchmarks/interim_gemma_qa_score.py` — the LongMemEval
  `get_anscheck_prompt` rubric copied **verbatim** (AST-identical to
  `evaluate_qa.py`), judged by Gemma.
- **Subsets:** deterministic round-robin-balanced 50-question subsets of the
  committed CortexDB retrieval logs.

## A6.2 — LongMemEval type-aware (50-question balanced subset)

Generator: type-aware `generation_prompt` (verbatim port). Judge: Gemma, official
anscheck rubric. **Overall accuracy: 0.64 (32/50).**

| question_type | correct | count | accuracy |
| --- | ---: | ---: | ---: |
| single-session-preference | 7 | 8 | **0.875** |
| temporal-reasoning | 8 | 8 | 1.000 |
| knowledge-update | 7 | 9 | 0.778 |
| single-session-user | 4 | 8 | 0.500 |
| single-session-assistant | 3 | 8 | 0.375 |
| multi-session | 3 | 9 | 0.333 |

**Key signal:** `single-session-preference` = **0.875**, vs the LongMemEval
baseline of **0.2667** — the A6.2 hypothesis (type-aware prompting lifts the
preference slice past 0.55) is validated end-to-end on real data, even under an
interim judge. `multi-session` is the weakest slice (aggregation across sessions).

Raw: [`gemma_interim/longmemeval_typeaware_50_gemma_interim.json`](gemma_interim/longmemeval_typeaware_50_gemma_interim.json).

## F3.4 — LoCoMo QA category-aware (50-question balanced subset)

Generator: category-aware `qa_prompt` (multi-hop / temporal / open-domain /
adversarial-abstention). Judge: Gemma, generic correctness rubric.
**Overall accuracy: 0.56 (28/50).**

| category | correct | count | accuracy |
| --- | ---: | ---: | ---: |
| adversarial | 8 | 10 | **0.800** |
| temporal-reasoning | 7 | 10 | 0.700 |
| open-domain | 6 | 10 | 0.600 |
| single-hop | 5 | 10 | 0.500 |
| multi-hop | 2 | 10 | 0.200 |

**Key signal:** `adversarial` = **0.80** — the category-aware exact-abstention
instruction works (LoCoMo's adversarial slice tests refusal). `multi-hop` is the
weakest (chaining evidence across turns).

Raw: [`gemma_interim/locomo_qa_50_gemma_interim.json`](gemma_interim/locomo_qa_50_gemma_interim.json).

## A6.3 — LongMemEval hybrid dense retrieval (bounded, deterministic metric)

Unlike A6.2/F3.4 (LLM-judged), A6.3 is **embedding-based with a deterministic
offline metric** (recall/ndcg — no chat judge). Added `--retrieval-mode
{keyword,hybrid}` to `scripts/longmemeval/v1_cortexdb_retrieval.py`: hybrid embeds
each session's `index_text` + the question via the `bge-m3` embedding endpoint
(the same LiteLLM proxy), unit-normalizes to i16 Q15, appends the `vector=` payload
line, and searches `--mode hybrid`. **Keyword default is byte-identical** to the
committed F3.1 baseline (self-test `longmemeval-v1-hybrid-retrieval-check`).

Bounded run on the first **10** questions (keyword recall_all@10 is already
saturated there, so headroom is in ranking quality):

| metric | keyword | hybrid |
| --- | ---: | ---: |
| recall_all@10 | 1.000 | 1.000 |
| recall_all@1 | 0.900 | **1.000** |
| ndcg_any@10 | 0.9631 | **1.000** |

Hybrid **improves ranking (recall@1 +0.10, ndcg +0.037) with zero regressions** —
the A6.3 mechanism works end-to-end.

### Full 500-question run — **A6.3 DoD MET**

Ran the full split (24.5k `bge-m3` embeddings, cached) vs the matched keyword
baseline (aggregate metrics, skips excluded, per the official
`print_retrieval_metrics.py`):

| metric | keyword | hybrid | DoD target |
| --- | ---: | ---: | --- |
| recall_all@10 | 0.8926 | **0.9523** | >= 0.93 ✓ |
| ndcg_any@10 | 0.8733 | **0.9218** | >= 0.82 ✓ |

Per-question over all 500: **1** recall_all@10 regression (gate: <=10 ✓), **26**
improvements. Keyword default byte-identical (self-test). **All three A6.3
acceptance criteria pass** — this is a real phase closure with a *deterministic*
metric, no LLM judge. Reproduce: `make longmemeval-v1-hybrid-retrieval`
(optionally `LONGMEMEVAL_V1_HYBRID_LIMIT=N`).

## What remains to close the phases officially

- **A6.2 DoD:** the official GPT-4o `evaluate_qa.py` over the full 500 questions
  (preference ≥0.55, overall ≥0.80). The harness + subset pipeline are proven;
  scaling to 500 + swapping the judge to GPT-4o is the remaining metered step.
- **F3.4 QA DoD:** the official snap-research/locomo per-category **F1** (not
  yes/no accuracy) over the full set, packaged via `check_qa_evidence.py` into a
  `benchmark_report.v1` snapshot (`leaderboard_comparable: false`).

## Reproduce

```
# A6.2 (LongMemEval type-aware), 50-question subset:
python3 scripts/longmemeval/run_official_generation.py \
  --retrieval-log <lme_retrieval.jsonl> --reference-file <lme_retrieval.jsonl> \
  --output <hyp.jsonl> --model google/gemma-4-31B-it \
  --base-url <proxy>/v1 --api-key-file <proxy.key>
python3 scripts/benchmarks/interim_gemma_qa_score.py --hyp <hyp.jsonl> \
  --ref <lme_retrieval.jsonl> --type-field question_type --rubric longmemeval \
  --model google/gemma-4-31B-it --base-url <proxy>/v1 --key-file <proxy.key>

# F3.4 (LoCoMo QA), 50-question subset:
make locomo-qa-input LOCOMO_QA_LIMIT=50           # reshape retrieval log -> run_qa input
python3 scripts/locomo/run_qa.py --input-log <qa_input.jsonl> --output <hyp.jsonl> \
  --model google/gemma-4-31B-it --base-url <proxy>/v1 --api-key-file <proxy.key>
python3 scripts/benchmarks/interim_gemma_qa_score.py --hyp <hyp.jsonl> \
  --type-field category --gold-field gold_answer --rubric generic \
  --model google/gemma-4-31B-it --base-url <proxy>/v1 --key-file <proxy.key>
```
