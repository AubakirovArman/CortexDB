# Support Ticket Triage Pack

Goal: show how CortexDB retrieves customer support incidents, records a
workflow memory update, and verifies that the documented resolution matches the
ticket evidence.

Scope: `support:tickets`

Fixture:

```text
examples/datasets/support_tickets/cells.jsonl
```

## Corpus

The pack has two corpus layers:

```text
examples/datasets/support_tickets/cells.jsonl
examples/real_domains/support_tickets/
```

The fixture is a small runnable support scenario. The real-domain folder adds a
validated synthetic corpus with documents, chunks, queries, and ground truth
rows for retrieval quality gates.

## Demo

Run the full local demo:

```bash
./examples/demo/support_tickets/run.sh
```

## Customer Issue Retrieval

```bash
cargo run -p cortex-cli -- load-fixture target/use-case-packs/support-ticket-triage/db examples/datasets/support_tickets
cargo run -p cortex-cli -- search --json target/use-case-packs/support-ticket-triage/db support:tickets \
  "authentication outage signing key drift"
```

## Memory Update

```bash
cargo run -p cortex-cli -- context --format json target/use-case-packs/support-ticket-triage/db support:tickets \
  'RETRIEVE CONTEXT FOR TASK "Find repeated authentication incidents and successful remediation steps" IN BRAIN default REQUIRE citations LIMIT 10 CANDIDATES;'

cargo run -p cortex-cli -- --json remember target/use-case-packs/support-ticket-triage/db support:tickets \
  'REMEMBER "For repeated authentication failures, check token issuer drift before cache invalidation" IN SCOPE support:tickets AS TYPE workflow_result TTL 1209600 SECONDS;'
```

## Resolution Verification

```bash
cargo run -p cortex-cli -- verify --format json target/use-case-packs/support-ticket-triage/db support:tickets \
  'VERIFY FACT "The authentication outage was mitigated by rotating the signing key" IN BRAIN default;'
```

Expected behavior: search retrieves the customer issue, REMEMBER records a
workflow result memory, and VERIFY returns supporting evidence for the signing
key resolution.

Boundary: this pack is a developer scenario. It is not a customer-support SLA,
incident-response certification, or production operations guarantee.
