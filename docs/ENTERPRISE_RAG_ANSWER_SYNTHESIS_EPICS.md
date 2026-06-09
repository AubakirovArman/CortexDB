# EnterpriseRAG Answer Synthesis Epics

Status: active execution plan
Scope: EnterpriseRAG-Bench answer synthesis, evidence extraction, ContextPack cost, abstention, and submission readiness
Baseline: v81 retrieval with `Document Recall = 85.74`, noisy top-10, and model-dependent answer quality

## Goal

Move CortexDB from "retrieves good documents" to "turns retrieved evidence into correct enterprise answers".

Target gate:

```text
Overall Score:          55+
Answer Correctness:     72+
Answer Completeness:    78+
Document Recall:        keep 85+
Invalid Extra Docs:     <= 6.8
Generation tokens:      <= 4.0M / 500 questions
```

## Execution Order

### Epic 1. Evidence Slot Planner

Goal: identify the exact facts each question requires before answer generation.

Status: initial implementation landed.

Implemented:
- `scripts/enterprise_rag_bench/evidence_slot_planner.py` builds deterministic
  evidence-slot plans from question text and `question_type`.
- `scripts/enterprise_rag_bench/evidence_slot_plan_check.py` writes per-question
  JSONL plans and an aggregate report.
- `scripts/enterprise_rag_bench/run_deepseek_answers.py` can inject plans with
  `--include-evidence-plan` and optional `--evidence-plan-file`.
- `make enterprise-rag-bench-evidence-plan-check` validates the balanced-50 plan
  generation path without LLM/API usage.

Tasks:
- Define expected slots per EnterpriseRAG question type.
- Generate `evidence_plan.json` per question.
- Pass required slots into answer prompts.
- Require abstention when required slots cannot be filled.

Acceptance:
- `answer_missing_gold_facts` decreases by at least 20%.
- Completeness improves without lowering document recall.

### Epic 2. Evidence Table Extractor

Goal: extract exact facts from retrieved documents before the LLM writes an answer.

Status: initial implementation landed.

Implemented:
- `scripts/enterprise_rag_bench/evidence_table_extractor.py` extracts candidate
  numeric facts, dates, literals, status/cause/mitigation/owner markers, and
  table rows from retrieved documents.
- `scripts/enterprise_rag_bench/evidence_table_check.py` writes per-question
  evidence tables and an aggregate report.
- `scripts/enterprise_rag_bench/run_deepseek_answers.py` can inject tables with
  `--include-evidence-table` and optional `--evidence-table-file`.
- `make enterprise-rag-bench-evidence-table-check` validates balanced-50 table
  generation without LLM/API usage.

Tasks:
- Extract numeric facts, dates, thresholds, names, statuses, table rows, headers, and source ids.
- Emit compact evidence rows with `doc_id`, `fact_type`, `text`, and optional line/span metadata.
- Feed evidence rows into answer generation.

Acceptance:
- Correctness improves by at least 3 points on the 500-question gate.
- Prompt tokens do not grow by more than 10%.

### Epic 3. Evidence-First ContextPack

Goal: reduce cost and noise by sending evidence first, not long document windows first.

Tasks:
- Add `context_mode = evidence_first`.
- Order prompt context as question, required slots, evidence table, short snippets, and full windows only when needed.
- Keep source ids attached to each evidence item.

Acceptance:
- Generation tokens drop by at least 30%.
- Correctness and completeness do not regress.

### Epic 4. Invalid Extra Docs Reducer v3

Goal: make top-10 documents more evidential and less noisy.

Tasks:
- Penalize documents with no entity overlap, no slot coverage, no source relevance, weak anchors, or near-duplicate coverage.
- Preserve exceptions for conflict and completeness modes.
- Add per-question invalid-extra analysis.

Acceptance:
- `Invalid Extra Docs` drops from 8.23 to <= 6.8.
- `Document Recall` stays >= 84.

### Epic 5. Span-Level Reranking

Goal: choose the right spans inside retrieved documents.

Tasks:
- Split top documents into title/header spans, paragraph windows, table rows, and anchor-neighbor windows.
- Rerank spans against question and evidence slots.
- Feed top spans into ContextPack instead of full windows.

Acceptance:
- Fact token coverage improves.
- Generation tokens decrease.
- Missing-gold-fact failures decrease.

### Epic 6. High-Level Question Mode

Goal: handle high-level questions as source-grounded synthesis, not normal top-k lookup.

Tasks:
- Detect high-level questions.
- Cluster retrieved documents by topic/source.
- Select representative evidence per cluster.
- Build a high-level digest with key themes, sources, conflicts, and missing information.

Acceptance:
- High-level correctness is no longer zero.
- Answers become source-grounded instead of generic.

### Epic 7. Info-Not-Found Abstention Engine

Goal: stop hallucinating when evidence is missing.

Tasks:
- Compute evidence confidence from max evidence score, slot coverage, source match, and exact anchors.
- Return "not enough information" before LLM answer generation when confidence is below threshold.
- Add a dedicated not-found prompt.

Acceptance:
- `info_not_found` correctness improves materially.
- Hallucinated unavailable answers decrease.

### Epic 8. Project-Related Evidence Graph

Goal: collect multi-source project evidence across Jira, GitHub, Slack, Confluence, Gmail, and Drive.

Tasks:
- Detect project/entity seeds.
- Expand from seed docs to linked artifacts and neighboring project evidence.
- Group evidence into a project bundle.

Acceptance:
- Project-related completeness improves.
- Multi-document project facts are less often missed.

### Epic 9. Project Answer Synthesizer

Goal: answer project questions from a structured project table, not raw document text.

Tasks:
- Build project rows with project, owner, status, blocker, deadline, and source.
- Instruct model to answer only from this table.
- Explicitly expose conflicts when table fields disagree.

Acceptance:
- Project-related correctness exceeds 30% as the first target.

### Epic 10. Completeness Coverage Planner

Goal: ensure completeness questions cover every required subpart.

Tasks:
- Decompose question into sub-requirements.
- Match evidence docs/spans to each sub-requirement.
- Sort ContextPack by coverage gain.
- Avoid selecting many documents covering the same subtopic.

Acceptance:
- Answer completeness improves by at least 4 points.
- Completeness-category recall improves by at least 10 points.

### Epic 11. Conflict-Aware Answer Mode

Goal: make conflicting evidence explicit instead of letting the model silently choose one source.

Tasks:
- Detect same entity with different number/date/status.
- Group evidence by conflict dimension.
- Use a prompt that requires "Evidence A says..." and "Evidence B says...".

Acceptance:
- `conflicting_info` correctness improves.
- Invalid extra docs do not degrade the final answer.

### Epic 12. Model-Robust Prompt Suite

Goal: reduce quality swings between Gemma, Gemini, DeepSeek, and future official judges.

Tasks:
- Add prompt profiles for `gemma_strict`, `gemini_strict`, `deepseek_strict`, and `official_eval_ready`.
- Keep invariant instructions: use only evidence, fill slots, cite sources, abstain when missing, avoid speculation.
- Compare prompts on the same 50-question and 500-question gates.

Acceptance:
- Model-to-model score gap narrows.
- DeepSeek answer score no longer collapses under the same retrieval.

### Epic 13. Answer Repair Loop

Goal: fix missing slots or unsupported claims with one bounded repair pass.

Tasks:
- Compare generated answer against evidence slots.
- Regenerate once if required facts are missing.
- Remove unsupported claims or mark them uncertain.

Acceptance:
- Completeness improves.
- Token cost increase stays <= 20%.

### Epic 14. Citation Discipline Enforcer

Goal: ensure factual claims map to retrieved source documents.

Tasks:
- Map factual sentences to source docs.
- Remove unsupported factual sentences.
- Add source id markers where useful.
- Validate cited docs are included in `document_ids`.

Acceptance:
- Invalid or unsupported citations decrease.
- Correctness does not regress.

### Epic 15. Context Cost Optimizer

Goal: cut generation token cost while preserving quality.

Tasks:
- Set per-question token budget targets.
- Drop low-value windows.
- Prefer evidence digests and span-level context.
- Compress repeated metadata.

Acceptance:
- Generation tokens drop to <= 3.5M for 500 questions.
- Overall score does not drop by more than 1 point.

### Epic 16. Answer-Aware Document Pruning

Goal: remove semantically similar but answer-irrelevant documents before generation.

Tasks:
- Prune docs without anchors, slot coverage, source relevance, or factual overlap.
- Preserve conflict-mode exceptions.
- Track which pruned docs would have been invalid extras.

Acceptance:
- `Invalid Extra Docs` drops to <= 6.5.
- Correctness improves or stays stable.

### Epic 17. Official Evaluator Submission Pack

Goal: make results reproducible and submission-ready.

Tasks:
- Create `docs/ENTERPRISE_RAG_BENCH_SUBMISSION.md`.
- Package `answers.jsonl`, `document_ids`, config, prompt version, model versions, CortexDB commit, retrieval mode, top-k, context budget, token accounting, and eval command.

Acceptance:
- Another engineer can reproduce the run from the submission package.

### Epic 18. Official-vs-Local Judge Matrix

Goal: separate local-only scores from official-comparable scores.

Tasks:
- Evaluate the same `answers.jsonl` with Gemma local judge, Gemini local judge, DeepSeek local judge, and official judge where available.
- Record score deltas and category-level disagreements.
- Document judge assumptions.

Acceptance:
- Local score claims are clearly separated from official-comparable score claims.

### Epic 19. Category Breakdown Dashboard

Goal: make regressions visible by category instead of only by total score.

Tasks:
- Report correctness, completeness, recall, invalid extras, token cost, and failure bucket for each EnterpriseRAG category.
- Track categories: basic, semantic, intra-document, project-related, constrained, conflicting-info, completeness, high-level, info-not-found.

Acceptance:
- Every experiment shows per-category deltas and weak spots.

### Epic 20. Regression-Safe Experiment Harness

Goal: prevent new experiments from overwriting the current best pipeline.

Tasks:
- Assign every experiment an id.
- Store retrieval output, answers, judge output, metrics, token cost, and failure buckets.
- Promote only if overall improves, no catastrophic category regression occurs, and token cost stays acceptable.

Acceptance:
- `make enterprise-rag-bench-promote-candidate EXP=vXX` promotes only validated candidates.

## Immediate Sprint

1. Epic 1: Evidence Slot Planner.
2. Epic 2: Evidence Table Extractor.
3. Epic 3: Evidence-First ContextPack.
4. Epic 7: Info-Not-Found Abstention Engine.
5. Epic 6: High-Level Question Mode.

## Current Weak Spots

```text
high_level        needs dedicated synthesis mode
info_not_found    needs abstention before generation
project_related   needs evidence graph plus table-based answer synthesis
semantic          needs better exact evidence extraction
invalid extras    needs answer-aware pruning
token cost        needs evidence-first ContextPack
```

## Non-Goals

- Do not increase top-k as the primary fix.
- Do not chase model changes before evidence assembly improvements.
- Do not claim official leaderboard readiness until the submission pack is reproducible.
- Do not mix local judge scores with official scores in public claims.
