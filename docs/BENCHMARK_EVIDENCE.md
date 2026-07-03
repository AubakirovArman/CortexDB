# Benchmark Evidence — Judge of Record (F2.0)

This document is the fixed decision on the **judge of record** for CortexDB's
answer-quality benchmarks, plus the evidence trail that makes the decision
machine-checkable. It is the companion to
[`PUBLIC_CLAIMS_POLICY.md`](PUBLIC_CLAIMS_POLICY.md) (§ Judge of Record) and the
committed registry under `fixtures/benchmarks/registry/`.

## Decision

An LLM judge is part of the measuring instrument for EnterpriseRAG-Bench, so a
delta between a run judged by one model and a run judged by another measures the
**judges**, not the systems. The decision is therefore fixed, not per-run:

| Role | Judge | May be leaderboard-comparable? |
| --- | --- | --- |
| **Official / leaderboard judge of record** | `gpt-5.4` (`openai`) | **Yes** — only `gpt-5.4`-judged numbers may carry an official/leaderboard header or `leaderboard_official` / `leaderboard_comparable`. |
| **Interim in-house judge** | `gemini-3.5-flash` (`google`) | **No** — valid for internal comparison (comparable to the in-house `47.74` combined baseline), but always carries the interim / non-official qualifier. |

## Evidence trail (all committed, all machine-checked)

- **Registry** — `fixtures/benchmarks/registry/enterprise_rag_bench_official_500.json`
  records the interim ERB run: `judge = gemini-3.5-flash`, `judge.official =
  false`, `leaderboard_official = false`, `status = "interim_non_official_judge"`,
  `combined = 47.74`. So the interim number is committed *as* non-official.
- **Machine rule** — `scripts/benchmarks/registry_summarize.py`
  (`make benchmark-registry-check`) fails if any entry sets
  `leaderboard_official: true` without `judge.official: true`. So a
  `gemini`-judged number can never be published as leaderboard-official.
- **Re-judge readiness** — `fixtures/benchmarks/erb/official_rejudge_target.v1.json`
  + `scripts/enterprise_rag_bench/check_official_rejudge.py`
  (`make erb-official-rejudge-ready-check`, F2.1) pin `gpt-5.4` as the judge of
  record and gate the metered re-judge on its key; until then the official header
  stays blocked, not fabricated.
- **Cross-run guard** — `scripts/enterprise_rag_bench/compare_official_runs.py`
  (`make erb-compare-runs-check`, F2.2) refuses to emit a delta between two runs
  scored by different judges.
- **Consistency gate** — `scripts/benchmarks/judge_of_record_check.py`
  (`make judge-of-record-check`, this phase) asserts this doc + the policy declare
  `gpt-5.4` as the official judge and `gemini-3.5-flash` as interim, and that no
  committed registry entry violates the machine rule.

## Relation to `RESULTS.md`

[`RESULTS.md`](RESULTS.md) describes a "Gemini judge of record" — that is the
**interim in-house** record (the `gemini-3.5-flash`-judged `47.74` line), which is
correct and explicitly non-leaderboard. It does **not** conflict with this
decision: the **official** judge of record for any leaderboard-comparable claim is
`gpt-5.4`, and no interim number is ever presented as leaderboard-official.
