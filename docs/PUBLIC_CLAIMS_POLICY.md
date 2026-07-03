# Public Claims Policy

This policy keeps CortexDB public-facing documentation aligned with the actual
single-node agent-native database beta state.

## Allowed Claims

- CortexDB is a single-node agent-native database beta (`v0.2.0-beta.2`).
- CortexDB may describe `v0.2.0-beta.2` as a local single-node developer/API
  beta when that statement keeps the non-production boundary explicit.
- The single-node durable core has repeatable test and release evidence.
- HTTP, CLI, SDK, AQL, ContextPack, backup/restore, and ANN gates exist for the
  documented beta contract.
- ANN/HNSW, consensus, dashboard, and SDK publication can be described as
  guarded, experimental, blocked, or future product layers when that status is
  explicit.

## Disallowed Claims

Do not describe CortexDB as:

- production-ready;
- enterprise-ready;
- fully production-grade;
- a production distributed consensus database;
- an unrestricted production ANN/HNSW search engine;
- an SLA-backed or benchmark-certified high-performance database.

## Required Qualifiers

Public docs that describe product status must include the relevant qualifier:

- `single-node agent-native database beta` for the current release status;
- `v0.2.0-beta.2` for the current workspace version;
- `not recommended for production workloads` or equivalent for README-level
  positioning;
- `not a production SLA` for API performance or server behavior;
- `experimental`, `guarded`, `future`, or `blocked` for ANN/HNSW, consensus,
  product UI, and SDK publication lifecycle claims.

The release gate is `make public-claims-check`. It writes
`target/public-claims/report.json` and is paired with the release-facing freeze
checklist in [`PUBLIC_CLAIMS_FREEZE.md`](archive/PUBLIC_CLAIMS_FREEZE.md).

## Judge of Record (F2.0)

An LLM judge is part of the measuring instrument for answer-quality benchmarks
(EnterpriseRAG-Bench), so **which judge scored a number governs what may be
claimed about it.** This is a fixed policy, not a per-run choice:

- **Official / leaderboard judge of record: `gpt-5.4` (provider `openai`) only.**
  A benchmark number may be presented in an official header, called
  "leaderboard", or marked `leaderboard_official` / `leaderboard_comparable` only
  if `gpt-5.4` scored it.
- **Interim in-house numbers: judged by `gemini-3.5-flash`.** These are valid for
  internal comparison (they are comparable to the committed in-house ERB baseline
  of `47.74` combined) but are **never** leaderboard-official or
  leaderboard-comparable, and must carry the interim / non-official qualifier.
- **Machine rule.** A registry snapshot (or any published claim) whose judge is
  not the official judge of record **cannot** carry
  `leaderboard_official: true`. This is enforced by
  `scripts/benchmarks/registry_summarize.py` (a `leaderboard_official` entry
  requires `judge.official`) and by `scripts/benchmarks/judge_of_record_check.py`
  (`make judge-of-record-check`), which also asserts this policy + the evidence
  page agree with the committed registry.

This resolves the apparent tension with
[`RESULTS.md`](RESULTS.md)'s "Gemini judge of record": that line describes the
**interim in-house** record (the `gemini-3.5-flash`-judged `47.74` line), which is
correct and non-leaderboard; the **official** judge of record for any
leaderboard-comparable claim is `gpt-5.4`. Cross-run comparisons additionally
refuse across judges (`erb-compare-runs-check`, F2.2).
