---
name: Good first issue
about: Small, bounded starter task for new CortexDB contributors
title: "[good-first-issue] "
labels: good first issue
assignees: ""
---

## Goal

One sentence describing the starter task.

## Scope

Expected files or module area:

- `docs/...`
- `crates/...`

## Success Criteria

Commands that should pass:

```bash
make contributor-onboarding-check
cargo fmt --check
```

Add any narrower gate here, for example:

```bash
cargo test -p cortex-aql
make use-case-pack-check
```

## Boundaries

This issue should not:

- add new dependencies;
- change public API schemas without OpenAPI/snapshot updates;
- add secrets or hosted credentials;
- make production claims outside `docs/PUBLIC_CLAIMS_POLICY.md`.
