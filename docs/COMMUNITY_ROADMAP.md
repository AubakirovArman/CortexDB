# CortexDB Community Roadmap Board

Status: Epic 140 community roadmap board.

This board is the public-facing planning layer for contributors. It separates
near-term milestones from blockers and experimental tracks so new work does not
accidentally turn future research into a public product claim.

## Roadmap Contract

| Required item | Roadmap location | Rule |
| --- | --- | --- |
| Add milestones | [Milestones](#milestones) | Every milestone must have a target, owner area, and gate. |
| Add beta blockers | [Beta Blockers](#beta-blockers) | Beta blockers must stay open until they have repeatable evidence. |
| Add production blockers | [Production Blockers](#production-blockers) | Production blockers must not be described as shipped features. |
| Add experimental tracks | [Experimental Tracks](#experimental-tracks) | Experimental tracks must carry explicit non-claim language. |

## Milestones

| Milestone | Target | Owner area | Exit gate |
| --- | --- | --- | --- |
| Beta contract freeze | Stable HTTP, CLI, SDK, ContextPack, VERIFY, guarded retrieval, backup, and operations contracts. | API / SDK / Operations | `make beta-release-check` |
| Production v1 local boundary | Local single-node production-like evidence without distributed production claims. | Engine / Storage / Release | `make production-v1-check` |
| ANN guarded promotion | Repeatable recall, latency, fallback, freshness, and history evidence for guarded ANN serving. | Search / ANN | `make ann-production-slo-history-check` and `make ann-real-embedding-release-check` |
| Dashboard product surface | Standalone operator/debug UI with role-aware flows and visual smoke evidence. | Dashboard / Server | `make dashboard-product-check` |
| Community contribution loop | Bounded starter issues, module map, roadmap board, and contributor gates. | Docs / Maintainers | `make contributor-onboarding-check` and `make community-roadmap-check` |

## Beta Blockers

| Blocker | Why it blocks beta | Required evidence |
| --- | --- | --- |
| Repeatable real-domain ANN history | One local real-embedding run is not enough to promote real-domain ANN behavior. | Multiple stable local runs, packaged baselines, and no-regression history. |
| SDK publication discipline | Public clients need version lock-step, deprecation policy, and package registry ownership. | Registry dry-runs or published package receipts plus changelog evidence. |
| Product UI beta readiness | The dashboard must expose operator errors, role flows, and stable recovery paths clearly. | Dashboard product gate plus screenshot/e2e evidence. |
| Consensus beta evidence | Raft-like primitives need sustained failover/rejoin evidence before beta wording. | `consensus-release-lane-check` with N consecutive partition, failover SLO, rejoin, SCALE-1, SCALE-2, and SCALE-3 green runs. |

## Production Blockers

| Blocker | Production risk | Required evidence |
| --- | --- | --- |
| Distributed consensus is not production HA | Local tests do not prove multi-node safety under real operational churn. | Long-running split-brain/rejoin, leader failover, snapshot handoff, and membership lifecycle evidence. |
| Managed cloud is not implemented | Hosted multi-tenant operations, billing, isolation, and SRE workflows are future work. | Cloud tenant lifecycle, backup/restore, upgrade, security, and incident gates. |
| Enterprise RBAC/compliance is future work | Current controls are local/token oriented, not enterprise policy governance. | Policy store, audit review tooling, external identity, and compliance mapping evidence. |
| Legal-grade verification is out of scope | Deterministic VERIFY helps find evidence conflicts but is not legal advice. | Legal dataset, citation policy, reviewer workflow, and explicit domain validation. |

## Experimental Tracks

| Track | Current boundary | Promotion condition |
| --- | --- | --- |
| Production distributed consensus | Release-lane CI evidence through `consensus-release-lane-check`; not production HA. | Sustained multi-process consensus evidence and operator lifecycle docs. |
| Managed cloud | Design/feasibility only. | Real hosted control plane, tenant isolation, backups, upgrades, and incident handling. |
| Enterprise RBAC/compliance | Design boundary only. | Dynamic policy store, external identity integration, audit review tooling, and compliance gates. |
| Full HNSW without fallback | Future non-goal; guarded ANN keeps exact fallback available. | Recall/latency history strong enough to remove fallback for selected collections. |
| Built-in LLM inference | Future runtime design only. | Local model lifecycle, safety limits, provider isolation, and cost controls. |
| External identity | Design/runbook only. | OIDC/SAML mapping, rotation, failure-mode tests, and admin recovery. |
| Legal-grade verification | Explicitly not claimed. | Domain-specific gold datasets, expert review loop, and citation/audit requirements. |

## Contributor Use

- Pick issues from `docs/GOOD_FIRST_ISSUES.md` when you want bounded starter work.
- Use this roadmap when proposing larger issues or design tasks.
- Link the relevant milestone or blocker in GitHub issues.
- Do not promote an experimental track into README, release notes, SDK docs, or
  dashboard copy until its promotion condition has repeatable evidence.

## Evidence Gate

Run:

```bash
make community-roadmap-check
```

The gate validates that this board keeps milestones, beta blockers, production
blockers, experimental tracks, and public-claims boundaries visible.
