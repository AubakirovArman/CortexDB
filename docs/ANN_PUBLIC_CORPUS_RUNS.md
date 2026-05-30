# ANN Public Corpus Runs

This ledger records hosted public-corpus ANN/HNSW runs that are useful as
external evidence beyond the small in-repo fixtures.

These runs are not committed as large corpus files. The durable evidence is the
GitHub Actions run plus its uploaded artifact, which contains the converted
JSONL corpus, `report.json`, `history.json`, and an optional baseline package.

## siftsmall-public-l2-smoke

| Field | Value |
| --- | --- |
| Actions run | <https://github.com/AubakirovArman/CortexDB/actions/runs/26685491501> |
| Commit | `825c7dc1280bb1dc652e25f7b0dda141d0366f29` |
| Source | `ftp://ftp.irisa.fr/local/texmex/corpus/siftsmall.tar.gz` |
| Dataset id | `siftsmall-public` |
| Baseline id | `siftsmall-public-l2-smoke` |
| Format | `fvecs` vectors + `ivecs` ground truth |
| Metric | `l2` |
| Normalization | `none` |
| Scale | `1` |
| Top-k | `10` |
| Query sample | `10` queries |
| HNSW profile | `max_neighbors=16`, `ef_search=256`, `layer_count=4` |
| Recall gates | `min_recall_q16=65535`, `min_mean_recall_q16=65535` |
| Latency gates | `p95<=1500000000ns`, `max<=2500000000ns` |
| Result | `passed=true`, `production_safe=true` |

Observed hosted result:

| Metric | Value |
| --- | --- |
| Vectors | `10000` |
| Queries | `10` |
| Dimension | `128` |
| Graph nodes | `10000` |
| Graph edges | `319728` |
| Upper layers | `3` |
| Upper graph edges | `50200` |
| Min observed recall | `65535` |
| Mean recall | `65535` |
| p50 latency | `132433040ns` |
| p95 latency | `163163776ns` |
| Max latency | `181313166ns` |

Interpretation:

- The public-corpus workflow successfully ran outside the local machine.
- The sampled SIFT/TEXMEX corpus achieved exact top-10 recall for the sampled
  queries under the recorded HNSW profile.
- This is a hosted smoke baseline, not a full production benchmark. The next
  production-tuning step is to run either all queries from the same corpus or a
  larger public/domain corpus and compare later candidates against this style of
  archived report.

## siftsmall-public-full-l2

| Field | Value |
| --- | --- |
| Actions run | <https://github.com/AubakirovArman/CortexDB/actions/runs/26685633630> |
| Commit | `7df5f0a39ce623798e1a0eee585c2bd61dc19eaa` |
| Source | `ftp://ftp.irisa.fr/local/texmex/corpus/siftsmall.tar.gz` |
| Dataset id | `siftsmall-public-full` |
| Baseline id | `siftsmall-public-full-l2` |
| Format | `fvecs` vectors + `ivecs` ground truth |
| Metric | `l2` |
| Normalization | `none` |
| Scale | `1` |
| Top-k | `10` |
| Query sample | all `100` queries |
| HNSW profile | `max_neighbors=16`, `ef_search=256`, `layer_count=4` |
| Recall gates | `min_recall_q16=65535`, `min_mean_recall_q16=65535` |
| Latency gates | `p95<=2500000000ns`, `max<=5000000000ns` |
| Result | `passed=true`, `production_safe=true` |

Observed hosted result:

| Metric | Value |
| --- | --- |
| Vectors | `10000` |
| Queries | `100` |
| Dimension | `128` |
| Graph nodes | `10000` |
| Graph edges | `319728` |
| Upper layers | `3` |
| Upper graph edges | `50200` |
| Min observed recall | `65535` |
| Mean recall | `65535` |
| p50 latency | `234709955ns` |
| p95 latency | `335525811ns` |
| Max latency | `382737532ns` |

Interpretation:

- This is the preferred public `siftsmall` baseline for future comparisons
  because it evaluates the whole query set instead of the 10-query shakedown.
- The graph achieved exact top-10 recall for every query under the recorded
  HNSW profile.
- Latency gates are intentionally loose enough to tolerate hosted-runner
  variance. Future candidate comparisons should use report-to-report regression
  budgets rather than hard-coding a single machine's p95 as a universal SLO.
