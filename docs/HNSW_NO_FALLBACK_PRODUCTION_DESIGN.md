# HNSW No-fallback Production Design

Status: future design gate, not implemented.

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

## Required Gates

1. `make ann-production-no-fallback-check`
2. `make ann-real-domain-history-check`
3. `make ann-public-corpus-history-check`
4. `make ann-graph-freshness-check`
5. `make performance-trend-check`

## Acceptance

1. No-fallback mode is profile-scoped and opt-in.
2. Degraded graphs cannot serve no-fallback results.
3. Recall and latency reports are repeatable.
4. Public docs do not generalize beyond proven profiles.

## Non-goals

1. Removing exact fallback globally.
2. Claiming production HNSW for unknown corpora.
3. Serving stale graphs for critical workloads.
