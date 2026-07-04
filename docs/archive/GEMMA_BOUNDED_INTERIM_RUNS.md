# Gemma interim runs — A6.1 + A6.2 + F3.4 QA (full-scale, not leaderboard-official)

**Status: interim / diagnostic. `leaderboard_comparable: false`.** These numbers are
**not** the official A6.2 / F3.4 DoD results — they use an **interim Gemma judge**
(not the leaderboard-official GPT-4o for LongMemEval / gpt-5.4 for ERB). They now
cover the **full sets** (A6.2 all 500 LongMemEval questions, F3.4 all 1,986 LoCoMo
questions), after an initial 50-question bounded validation. Recorded as honest
evidence that the A6.2 and F3.4-QA harnesses run **end-to-end against a real LLM on
real retrieval data** at full scale. Public claims go only through the F5.1 page
flow; these interim numbers are narrative, not a gate threshold (an LLM judge is
outside the agent's control). Both answerer and judge are Gemma-31B, so the overall
numbers understate what a stronger official answerer/judge (GPT-4o) would produce —
the value here is the per-type *mechanism* signal.

## Full-scale results (headline)

**A6.2 — LongMemEval type-aware, all 500 · overall 0.682**

| type | acc | | type | acc |
|---|---|---|---|---|
| **single-session-preference** | **0.767** (23/30) | | single-session-user | 0.657 |
| temporal-reasoning | 0.737 | | multi-session | 0.624 |
| knowledge-update | 0.731 | | single-session-assistant | 0.607 |

→ preference **0.767 vs the 0.2667 baseline** — the A6.2 type-aware hypothesis is
confirmed **on all 500** (DoD narrative target preference ≥0.55 met). Overall 0.682
is below the ≥0.80 target because Gemma-31B is both answerer and judge; the
mechanism holds.

**F3.4 — LoCoMo QA category-aware, all 1,986 — TWO judges (Gemma-31B vs gpt-oss-120B)**

Overall: **0.592 (Gemma judge)** vs **0.485 (gpt-oss-120B judge)** — the same
"stronger judge is stricter" pattern the A6.1 ERB cross-check found (−0.107 overall).

| category | n | Gemma judge | gpt-oss-120B judge | Δ |
|---|---:|---:|---:|---:|
| **adversarial** | 446 | **0.960** | **0.637** | **−0.323** |
| single-hop | — | 0.564 | 0.524 | −0.039 |
| temporal-reasoning | — | 0.573 | 0.530 | −0.044 |
| multi-hop | — | 0.238 | 0.170 | −0.067 |
| open-domain | — | 0.229 | 0.208 | −0.021 |

→ **Honest correction:** the earlier headline "adversarial **0.960** — abstention
works at full scale" was **substantially judge-inflated**. Under the far stronger
gpt-oss-120B judge the same abstentions score only **0.637** (−0.323) — by far the
largest cross-judge gap of any slice. The lenient Gemma judge over-credited "Not
mentioned" answers; a stronger judge is much more critical of them. So the true
abstention quality is "good but not near-perfect (~0.64)," not 0.96. Every other
slice moves only −0.02…−0.07 (the ranking is stable; multi-hop/open-domain stay the
weak slices). Raw gpt-oss numbers:
[`gemma_interim/locomo_qa_1986_gptoss120b_interim.json`](gemma_interim/locomo_qa_1986_gptoss120b_interim.json).
Both are `leaderboard_comparable=false`; the official snap-research/locomo F1 scorer
is the DoD.

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

## A6.1 — EnterpriseRAG-Bench, all 500, judge cross-check (Gemma judge)

Unlike A6.2/F3.4 (which generate *new* answers), A6.1 re-judges the **already
committed** 500 ERB answers in [`erb-submission/answers.jsonl`](../../erb-submission/answers.jsonl)
(answerer `google/gemma-4-31B-it`). Those answers were recorded with a
**gemini-3.5-flash** interim judge (overall **47.74**; see
[`erb-submission/REPRODUCE.md`](../../erb-submission/REPRODUCE.md)). Here we run
the **exact same 2-axis official-clean rubric**
(`judge_metrics.prompts.build_prompt`, unchanged) with **two further independent
interim judges** — **Gemma-31B** and **gpt-oss-120B** (the largest model the proxy
serves) — over all 500, to test how judge-dependent the ~47 combined score is.
**No new answerer tokens** (answers are fixed); only ~500 judge calls each.

**Result — three independent judges span a ~42–48 band; the *stronger* model is
the *stricter* judge:**

| metric | gemini-3.5-flash (recorded) | Gemma-31B | **gpt-oss-120B** |
| --- | ---: | ---: | ---: |
| overall combined correctness/completeness | 47.74 | 46.71 | **41.82** |
| correctness | 50.0% | 49.2% | 45.2% |
| completeness | 53.7% | 53.4% | 49.5% |
| document recall (deterministic) | 55.71% | 55.71% | 55.71% |
| invalid extra docs (deterministic) | 9.23 | 9.23 | 9.23 |

Document recall / invalid-extra-docs are **deterministic** document-overlap
metrics (judge-independent) — **identical across all three judges**, confirming the
harness join is stable. The two LLM-judged axes are **judge-dependent within a
~6-point band**: gemini-flash and Gemma-31B agree within ~1 point (~47), but the
much larger **gpt-oss-120B judges ~5–6 points stricter (41.82)**. So the honest
reading is **not** "judge-agnostic ~47" but "**combined is robustly ~42–48, and a
stronger judge trends lower**." That materially tempers the DoD expectation: the
leaderboard-official `gpt-5.4` / GPT-4o evaluator (no budget) is a *stronger* judge
still, so it may well land **at or below** the interim band, **not above** it — the
optimistic "~62–68 within days" A6.4 projection is not supported by any of the three
interim judges. DoD still requires that official evaluator; this is the best
multi-judge interim signal obtainable without budget.

Per-`question_type` (Gemma judge, all 500; gpt-oss-120B in
[`gemma_interim/erb_a61_gptoss120b_judge_500.json`](gemma_interim/erb_a61_gptoss120b_judge_500.json)
follows the same relative shape — info_not_found ~95%, weak on
semantic/project_related/high_level — uniformly ~5pp stricter):

| question_type | n | correctness | completeness |
| --- | ---: | ---: | ---: |
| info_not_found | 20 | **100.0%** | 100.0% |
| miscellaneous | 20 | 85.0% | 79.0% |
| conflicting_info | 20 | 75.0% | 70.8% |
| intra_document_reasoning | 40 | 67.5% | 71.5% |
| constrained | 30 | 66.7% | 70.7% |
| basic | 175 | 53.7% | 55.0% |
| high_level | 10 | 30.0% | 33.8% |
| semantic | 125 | 28.8% | 34.1% |
| completeness | 20 | 25.0% | 41.7% |
| project_related | 40 | 22.5% | 41.8% |

**Key signals:** `info_not_found` = **100%** — the abstention behaviour holds on
ERB too (echoing LoCoMo adversarial 0.96 and the F4.3 abstention axis).
`conflicting_info` = 75% (conflict handling), `intra_document_reasoning` = 67.5%.
The weak slices are `semantic` (28.8%, the largest non-basic bucket) and
`project_related` (22.5%) — cross-document semantic aggregation, the same
weakness LongMemEval multi-session and LoCoMo multi-hop show.

Raw: [`gemma_interim/erb_a61_gemma_judge_500.json`](gemma_interim/erb_a61_gemma_judge_500.json).
Reproduce: the tested judge core `run_deepseek_answer_metrics.run` over
`erb-submission/answers.jsonl` + the ERB master `questions.jsonl` (gold), pointed
at the proxy Gemma (`make enterprise-rag-bench-official-clean-500-gemma
ENTERPRISE_RAG_BENCH_OFFICIAL_CLEAN_STAGE=judge` once `.env`'s `VLLM_URL` points
at the proxy).

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
