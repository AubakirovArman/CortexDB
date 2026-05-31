# Migration Compatibility Fixtures

`compatibility_matrix_v1.json` is the machine-readable Core Alpha migration
fixture. It records:

- current storage format markers;
- read-only legacy lexical markers (`ACI0`, `ACI1`);
- storage/API/SDK compatibility boundaries;
- offline upgrade and restore-only downgrade policy;
- required proof files and release gates.

Run:

```bash
make migration-compatibility-check
```

This does not claim online rolling upgrades. It proves that the current release
has an auditable compatibility matrix, old-format read coverage, and explicit
upgrade/downgrade notes before binary release artifacts are published.
