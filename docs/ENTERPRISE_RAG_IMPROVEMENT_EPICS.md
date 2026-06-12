# EnterpriseRAG-Bench Improvement Epics

This plan tracks the next no-oracle improvements after the full 500-question
EnterpriseRAG-Bench submission package.

## Baseline

Current submission artifact:

```text
erb-submission/answers.jsonl
```

Official-clean gpt-5.2 judge result:

```text
Overall              43.27
Correctness          49.20
Completeness         54.24
Document Recall      85.79
Invalid Extra Docs    8.21
```

Gemini self-judge upper-bound result:

```text
Overall              48.75
Correctness          53.60
Completeness         59.24
Document Recall      85.74
Invalid Extra Docs    8.22
```

## Fairness Rule

All improvements must remain official-clean. During inference the pipeline may
read only:

```text
question_id
question
retrieved document_ids
corpus document text and corpus metadata
```

The pipeline must not use these benchmark fields for routing, retrieval,
packing, prompting, or abstention:

```text
question_type
source_types
expected_doc_ids
gold_answer
answer_facts
```

Those fields are allowed only for post-run analysis and judging.

## Epics

### Epic 01 - Submission Docs Cleanup

Goal: make the submission package internally consistent and easy to reproduce.

Tasks:

- Remove stale `gemma` wording from current submission reproduction docs.
- Keep `gemini-3.5-flash` as the reported answer model.
- Keep `gpt-5.2` as the local strict judge and `gpt-5.4` as the final official
  judge target.
- Verify `erb-submission/README.md`, `erb-submission/REPRODUCE.md`, and
  `docs/ENTERPRISE_RAG_BENCH_SUBMISSION.md` agree.

Success criteria:

```text
grep finds no stale answer-model claims in the current submission docs.
```

### Epic 02 - High-Level Clean Mode

Goal: make overview questions answer from corpus evidence without using
`question_type`.

Status: done for the high-level subset gate. The clean retrieval path now
detects overview intent from the question text and can boost ordinary corpus
documents by path/title-style
metadata such as product overview, sales enablement, security, serving runtime,
pricing, and company handbook overview paths. The answer prompt also avoids
treating overview questions as unavailable when retrieved evidence exists. A
metadata-only shortcut handles overview-only clean runs without loading the full
checkpoint lexical index.

Tasks:

- Infer overview intent from the question text.
- Retrieve company, business, platform, organization, revenue, and mission
  overview documents from the corpus.
- Avoid abstaining when overview evidence is present.
- Keep info-not-found abstention based on evidence confidence, not labels.

Current evidence:

```text
cargo test -p cortex-engine --bin enterprise_rag_bench_retrieval overview
cargo test -p cortex-engine --bin enterprise_rag_bench_retrieval
target/enterprise-rag-bench/official-clean/high-level-epic02/retrieval.cached.clean.jsonl
```

High-level smoke evidence: the 10 official high-level questions produced 10
document IDs each with `oracle_fields=0`; total runtime was 17.5 seconds on the
existing full-corpus DB. The returned documents come from normal
`generated_data/sources` paths such as product overview, sales enablement,
security, serving runtime, pricing, and company handbook overview pages.

High-level answer/judge evidence:

```text
answer model: gemini-3.5-flash
judge model:  gemini-3.5-flash
questions:    10 high_level rows
overall:      30.0
correctness:  30.0
completeness: 41.0
answer tokens: prompt=57,470 completion=1,559 total=59,029
judge tokens:  prompt=4,761 completion=583 total=5,344
```

First-50 regression evidence:

```text
run:          target/enterprise-rag-bench/official-clean/50/epic02-regression-50/
answer model: gemini-3.5-flash
judge model:  gemini-3.5-flash
questions:    qst_0001..qst_0050
overall:      50.7
correctness:  52.0
completeness: 54.02
doc recall:   64.0
invalid docs: 9.36
answer tokens: prompt=291,233 completion=4,698 total=295,931
judge tokens:  prompt=24,021 completion=3,039 total=27,060
official-clean gate: passed
```

Open risk: this validates the high-level subset, not the full 50/500 score.
Failures remain on mission statement, company thesis, revenue streams,
add-ons, and organization questions, mostly because the current generic
overview retrieval still misses some exact summary/business facts. The first
50-question regression split does not include the high-level question IDs
(`qst_0471..qst_0480`), so it is a regression check rather than proof of the
high-level improvement.

Target:

```text
high_level Overall: 0.0 -> 30.0+
```

### Epic 03 - Project Answer Synthesizer

Goal: turn high-recall project-related retrieval into correct answers.

Status: partial, with the project subset target met by an oracle-free text
intent budget gate. The official-clean answer path can now inject a
deterministic evidence table without reading `question_type`, `source_types`,
expected docs, gold answers, or answer facts. The extractor tags project-style
facts such as `cause`, `fix`, `owner`, `status`, `date`, `risk`, and `source`,
and the official-clean prompt treats the table as a navigation aid rather than
an oracle. The benchmark wrapper exposes this through `--include-evidence-table`.
The answer runner also has `--enable-text-intent-budget`, which infers
`complex_project` from the visible question text and raises context/output
budget for complex project, incident, rollout, migration, and remediation
questions. The first measurement showed that the table should stay opt-in for
now: expanding answer budget/context helped the project subset more than
forcing the table into the prompt.

Tasks:

- Build a generic evidence table from retrieved documents:
  `cause`, `fix`, `owner`, `status`, `date`, `risk`, `source`.
- Synthesize project answers from that table instead of raw context windows.
- Prefer explicit source-grounded facts over nearby but unsupported details.
- Add regression checks on project-related questions without reading
  `question_type` during inference.

Current evidence:

```text
subset:      12 project_related rows from weak_semantic_project_completeness_50
retrieval:   existing official-clean 500 retrieval, 10 docs/question
judge:       gemini-3.5-flash
baseline:    Overall 2.92,  Correctness 8.33,  Completeness 30.25
table 520:   Overall 3.75,  Correctness 8.33,  Completeness 35.42
table 900:   Overall 27.50, Correctness 41.67, Completeness 45.25
no table 900: Overall 33.75, Correctness 50.00, Completeness 48.17
intent budget: Overall 25.83, Correctness 41.67, Completeness 47.75
recall:      84.56 across the subset
invalid docs: 5.33 across the subset
```

Artifacts:

```text
target/enterprise-rag-bench/official-clean/project-epic03/questions.clean.jsonl
target/enterprise-rag-bench/official-clean/project-epic03/questions.gold.jsonl
target/enterprise-rag-bench/official-clean/project-epic03/retrieval.clean.jsonl
target/enterprise-rag-bench/official-clean/project-epic03/answer-gemini-evidence-table-tok900/
target/enterprise-rag-bench/official-clean/project-epic03/answer-gemini-no-table-tok900/
target/enterprise-rag-bench/official-clean/project-epic03/answer-gemini-intent-budget/
```

Mixed weak-50 regression:

```text
subset:      50 rows from weak_semantic_project_completeness_50
mix:         30 semantic, 12 project_related, 8 completeness
retrieval:   same official-clean retrieval for baseline and candidate
judge:       gemini-3.5-flash

baseline submission answers:
overall:      30.00
correctness:  34.00
completeness: 40.06
doc recall:   67.14
invalid docs: 7.60

intent-budget candidate:
overall:      34.80
correctness:  40.00
completeness: 44.76
doc recall:   67.14
invalid docs: 7.60
answer tokens: prompt=345,934 completion=12,012 total=357,946
judge tokens:  total=53,293
intent split:  complex_project=18, default=32

category delta:
project_related: 2.92 -> 25.83
semantic:        39.67 -> 39.83
completeness:    34.38 -> 29.38
```

Decision: keep `--enable-text-intent-budget` as a candidate mode, not the
global default yet. It improves the mixed weak-50 score and strongly improves
project-related answers, but it regresses the completeness slice. The next
epic should reduce unsupported specifics and protect completeness before any
full-500 promotion.

Target:

```text
project_related Overall: 5.94 -> 20.0+
```

### Epic 04 - Anti-Hallucination Answer Mode

Goal: convert false-but-high-completeness answers into correct answers.

Status: partial. An oracle-free unsupported-claim guard now exists and is wired
through the answer runner, official-clean answer wrapper, and full benchmark
runner. It can run in `off`, `report`, or `suppress` mode and only checks exact
markers visible in generated answers, such as numbers, dates, IDs, versions,
paths, and units. The first suppress-mode measurement shows this should not be
promoted as a hard post-process yet: it removes too much useful completeness.
Use `report` mode for diagnostics and continue with prompt/repair-level
grounding instead of global sentence deletion.

Tasks:

- Suppress unsupported numbers, percentages, TTLs, dates, owners, and product
  names.
- Require every specific claim to appear in the evidence table or selected
  spans.
- Prefer a shorter answer over a plausible but unsupported answer.
- Add a post-answer unsupported-claim check before writing `answers.jsonl`.

Current evidence:

```text
guard implementation:
scripts/enterprise_rag_bench/answer_guard.py

flags:
--unsupported-claim-guard off|report|suppress

unit tests:
python3 -m unittest scripts/enterprise_rag_bench/test_answer_guard.py

mixed weak-50 suppress test:
source answers: intent-budget candidate from Epic 03
changed answers: 23 / 50
flagged statements: 39
removed statements: 39

intent-budget baseline:
overall:      34.80
correctness:  40.00
completeness: 44.76

suppress guard:
overall:      28.70
correctness:  36.00
completeness: 38.90

question deltas:
improved:  2
regressed: 12
same:      36
```

Decision: do not promote `suppress` mode. Keep the guard as an audit/reporting
tool and use its flagged markers to build safer answer prompts or a targeted
repair pass. A hard sentence remover is too blunt for project and completeness
questions because exact markers can be absent from the compact context even
when the original selected document contains the supporting fact.

Target:

```text
Correctness: 49.2 -> 55.0+
Invalid contradictions reduced in basic, semantic, and constrained questions.
```

### Epic 05 - Completeness Planner

Goal: answer all required sub-parts when evidence is available.

Tasks:

- Split the question into generic sub-requirements from text.
- Map each sub-requirement to evidence spans.
- Use checklist/table output when the question asks for multiple items,
  exceptions, steps, owners, or comparisons.
- Re-run one repair pass only when a sub-requirement has evidence but is missing
  from the answer.

Target:

```text
completeness Overall: 23.75 -> 40.0+
Answer Completeness: 54.24 -> 62.0+
```

### Epic 06 - Semantic Retrieval v2

Goal: improve semantic document discovery without oracle metadata.

Tasks:

- Expand queries using only question text.
- Add title/path/entity/source-metadata views to dense and lexical candidate
  generation.
- Improve dense rerank/fusion for paraphrased single-document questions.
- Track recall by category only after the run for diagnostics.

Target:

```text
semantic recall: 75.2 -> 85.0+
semantic Overall: 37.44 -> 45.0+
```

### Epic 07 - Invalid Extra Docs Reducer

Goal: reduce noisy context while preserving recall.

Tasks:

- Penalize candidates with no anchor overlap or evidence-slot coverage.
- Suppress near-duplicate documents in top-k.
- Keep diversity for project, conflict, and completeness questions using only
  text-inferred intent and corpus metadata.
- Compare recall and invalid-extra-doc deltas before promotion.

Target:

```text
Invalid Extra Docs: 8.21 -> <= 6.5
Document Recall: keep >= 85.0
```

## Execution Order

```text
1. Epic 01 - Submission Docs Cleanup
2. Epic 02 - High-Level Clean Mode
3. Epic 03 - Project Answer Synthesizer
4. Epic 04 - Anti-Hallucination Answer Mode
5. Epic 05 - Completeness Planner
6. Epic 06 - Semantic Retrieval v2
7. Epic 07 - Invalid Extra Docs Reducer
```

## Promotion Rule

Promote a new run only if it passes:

```text
official_clean_gate: passed
oracle_usage_audit: passed
full 500-question score produced
no catastrophic regression in info_not_found
document recall does not fall below 85.0 unless Overall improves materially
```
