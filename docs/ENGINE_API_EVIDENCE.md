# Engine API Evidence

The embedded API evidence gate is:

```bash
make engine-public-api-freeze-check
make engine-api-check
```

The freeze gate checks the stable facade fixture, compile coverage in
`crates/cortex-engine/tests/public_api.rs`, rustdoc examples, and required API
docs.

`make engine-api-check` extends that with compatibility, error model, feature
flag, module ownership, internal boundary, determinism, and panic-audit checks.
