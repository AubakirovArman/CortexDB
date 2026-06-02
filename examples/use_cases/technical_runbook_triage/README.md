# Technical Runbook Triage Pack

Goal: show how CortexDB retrieves technical documentation and verifies release
or API runbook facts.

Scope: `docs:technical`

Fixture:

```text
examples/datasets/technical_docs/cells.jsonl
```

Run:

```bash
cargo run -p cortex-cli -- load-fixture target/use-case-packs/technical-runbook-triage/db examples/datasets/technical_docs
cargo run -p cortex-cli -- search --json target/use-case-packs/technical-runbook-triage/db docs:technical "compatibility endpoint"
cargo run -p cortex-cli -- context --format json target/use-case-packs/technical-runbook-triage/db docs:technical \
  'RETRIEVE CONTEXT FOR TASK "compatibility endpoint" IN BRAIN default REQUIRE citations LIMIT 10 CANDIDATES;'
cargo run -p cortex-cli -- verify --format json target/use-case-packs/technical-runbook-triage/db docs:technical \
  'VERIFY FACT "The /v1/compatibility endpoint exposes API version and SDK contract version" IN BRAIN default;'
```

Boundary: this pack is a local documentation triage demo, not a production
incident-management workflow.
