# CortexDB v0.2.0-beta.2 Release Evidence

Status: R04 release-readiness evidence recorded.

Recorded at: `2026-06-16T07:29:59Z`

## Freeze Boundary

- Freeze scope: no new feature epics; release-readiness, docs, security, and
  release-gate fixes only.
- Evidence source base commit:
  `cc81b63084676623b49904edc9b121acb40199ec`.
- Evidence run tree: clean worktree at the base commit plus the R04
  release-gate and documentation changes in the commit that adds this file.
- Version: `0.2.0-beta.2` in `Cargo.toml`.

## Tag State

The tag `v0.2.0-beta.2` already existed before R04:

- tag object: `aec484b283f116ba7e9d78d052b6e0e2b649713e`;
- peeled commit: `bbd3b6c35a77a1d9c6d3845e9dd2b2ef91b16dc8`;
- remote: `refs/tags/v0.2.0-beta.2`.

R04 did not force-move this public tag. Moving it would rewrite an existing
published release pointer and requires explicit maintainer approval.

## Commands

Run from a clean checkout/worktree:

| Command | Status |
| --- | --- |
| `cargo fmt --check` | passed |
| `cargo test --workspace --all-features` | passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| `make beta-release-check` | passed |
| `make docs-site-check` | passed |
| `make security-check` | passed |

Additional targeted checks used while fixing the release gates:

| Command | Status |
| --- | --- |
| `python3 scripts/ann/bootstrap_history_fixture.py --self-test` | passed |
| `python3 scripts/ann/history_fixture_check.py` | passed |
| `python3 scripts/check_public_claims.py --self-test` | passed |
| `python3 scripts/check_public_claims.py --report target/public-claims/report.json` | passed |
| `make retrieval-quality-check ANN_REAL_EMBEDDING_RUN_ROOT=target/r04-clean-ann-runs ANN_REAL_EMBEDDING_HISTORY_REPORT=target/r04-clean-ann-runs/history.json` | passed |

## Evidence Artifacts

Generated local artifacts:

```text
target/beta-release/report.json
target/beta-release/evidence.tar.gz
target/docs-site/report.json
target/security/report.json
target/public-claims/report.json
target/retrieval-quality/report.json
target/release-artifact-manifest/report.json
```

Key report facts:

- `target/beta-release/report.json`: `status=passed`, `version=0.2.0-beta.2`,
  `suites=15`, `failed=[]`.
- `target/docs-site/report.json`: `status=passed`.
- `target/security/report.json`: `status=passed`.
- `target/public-claims/report.json`: `status=passed`.
- `target/retrieval-quality/report.json`: `status=passed`,
  `run_count=3`, `latest_run_id=clean-c`, `regression_count=0`.
- `target/release-artifact-manifest/report.json`: `status=passed`.

## Boundary

This evidence proves local single-node developer/API beta readiness gates. It
does not prove production distributed consensus, managed cloud readiness,
enterprise compliance, legal-grade verification, unrestricted HNSW without
fallback, built-in production LLM inference, or production SLA coverage.
