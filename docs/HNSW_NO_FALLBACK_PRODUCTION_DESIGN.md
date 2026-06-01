# HNSW No-fallback Production Design

Status: future phase 2 operator rollout policy started, not globally production-ready.

## Goal

Define the path for serving selected ANN/HNSW workloads without exact fallback.

## Allowed Workloads

No-fallback HNSW is allowed only for profiles with explicit corpus, metric,
dimension, graph parameters, recall target, latency target, rebuild policy, and
rollback plan.

Critical audit, legal, or safety-sensitive retrieval continues to require exact
fallback until separately proven.

## Recall SLO

Each profile must define minimum recall@k, mean recall, MRR, nDCG, and exact
parity thresholds. The thresholds must be evaluated across repeated runs, not a
single fixture pass.

## Latency SLO

Each profile must define p50, p95, p99, and max latency budgets. Reports must
include machine profile metadata so regressions are comparable.

## Graph Freshness

The graph must track source segment generation, vector count, deleted vector
count, rebuild count, and stale state. Stale or corrupt graphs must be blocked
from no-fallback serving.

## Serving Guardrails

Serving without fallback requires:

1. healthy graph signature;
2. current corpus generation;
3. passing recall history;
4. passing latency history;
5. no corruption flags;
6. explicit rollout flag.

## Runtime Rollout Policy

Local no-fallback serving is blocked by default. A selected profile can pass the
runtime rollout evaluator only when:

1. rollout is explicitly enabled for that profile;
2. the ANN search policy disables exact fallback;
3. SLO enforcement is required by both policy and report;
4. the search path is `hnsw_graph`;
5. no fallback reason or fallback execution is present;
6. `production_safe=true` and no SLO violations are present;
7. graph nodes, eligible candidates, returned results, recall, and upper-layer
   topology satisfy the profile threshold.

This runtime policy is a local guardrail. It does not promote HNSW to a general
fallback-free production engine for unknown corpora.

Operators can request the decision through CLI and HTTP by passing an explicit
rollout flag:

```text
cortexdb search-vector-eval --fallback false --require-slo \
  --no-fallback-rollout --no-fallback-min-recall 1.0 <path> <scope> <vector>

POST /v1/search/ann-evaluate?...&fallback=false&require_slo=true&no_fallback_rollout=true
```

The response includes `no_fallback_decision.allowed` and stable reason strings.
Absence of the rollout flag means CortexDB keeps the normal guarded ANN mode.

## Required Gates

1. `make ann-production-no-fallback-check`
2. `make ann-real-domain-history-check`
3. `make ann-public-corpus-history-check`
4. `make ann-graph-freshness-check`
5. `make performance-trend-check`

## Current Evidence Boundary

The current gates prove local HNSW no-fallback prerequisites only. They do not
remove exact fallback globally and they do not make HNSW safe for unknown
corpora.

| Gate | Evidence |
| --- | --- |
| `make ann-production-no-fallback-check` | synthetic, explicit external fixture, metric matrix, local domain reports with recall, latency, graph shape, `production_safe=true`, and runtime rollout policy tests |
| `make ann-real-domain-history-check` | local domain corpus report plus clean multi-run history fixture with no recall or latency regression |
| `make ann-public-corpus-history-check` | public-corpus harness self-test plus clean history fixture; real external public corpus source is still required before promotion |
| `make ann-graph-freshness-check` | HNSW persistence, maintenance, manifest profile, validation, stale/change, and corrupt graph guard tests |

Reports are written under `target/hnsw-no-fallback/`. They keep
`fallback_free_general_ready=false`; only selected local profiles can be marked
ready by their own evidence.

## Acceptance

1. No-fallback mode is profile-scoped and opt-in.
2. Degraded graphs cannot serve no-fallback results.
3. Recall and latency reports are repeatable.
4. Runtime policy rejects disabled rollout, fallback-enabled search policy,
   missing SLO enforcement, fallback reasons, unsafe reports, weak topology, and
   recall below the rollout threshold.
5. Public docs do not generalize beyond proven profiles.

## Non-goals

1. Removing exact fallback globally.
2. Claiming production HNSW for unknown corpora.
3. Serving stale graphs for critical workloads.
