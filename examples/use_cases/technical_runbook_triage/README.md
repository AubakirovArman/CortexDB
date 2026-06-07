# Technical Runbook Triage Pack

Goal: show how CortexDB retrieves technical documentation and verifies release
or API runbook facts.

Scope: `docs:technical`

Fixture:

```text
examples/datasets/technical_docs/cells.jsonl
```

Demo:

```bash
./examples/demo/technical_docs/run.sh
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

## Docs Retrieval

The pack retrieves technical docs about `/v1/compatibility`, beta release
gates, search explain fields, and migration matrix behavior:

```bash
cargo run -p cortex-cli -- search --json target/use-case-packs/technical-runbook-triage/db docs:technical \
  "compatibility endpoint SDK contract"
```

## Tool Hints

The fixture includes an explicit tool hint for compatibility failures:
run `cortexdb compatibility --json` and `cortexdb validate` before changing
storage files.

## Version Conflicts

The pack contains a deterministic version conflict example: SDK contract v1.4
with API contract v1.3 is marked incompatible by the migration matrix until the
server is upgraded.

## Source Refs

Every fixture cell includes `source=` metadata, and the ContextPack command uses
`REQUIRE citations` so the selected technical evidence carries source refs.

## Boundary

This pack is a local documentation triage demo. It is not production
incident-management workflow, migration certification, or operational approval.
