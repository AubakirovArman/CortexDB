# CortexDB Use-case Packs

Status: runnable beta scenarios for investment-project, legal, financial,
support, and technical use cases.

Use-case packs are small local scenarios that show how CortexDB moves from raw
cells to search, ContextPack generation, and deterministic verification. They
are not benchmarks and they are not domain certifications.

Run the pack gate:

```bash
make use-case-pack-check
```

The gate validates the pack manifest, fixture files, scenario docs, and then
runs CLI smoke flows for each pack.

## Included Packs

| Pack | Scope | Fixture | Purpose |
| --- | --- | --- | --- |
| Investment Projects | `project:investments` | `examples/datasets/investment_projects/cells.jsonl` | Retrieve project evidence, build cited ContextPacks, verify budget/battery facts, and connect to the local real-domain benchmark. |
| Legal Policy Review | `project:legal` | `examples/datasets/legal_policies/cells.jsonl` | Retrieve cited policy context and verify policy-update facts without claiming legal advice. |
| Financial Filing Review | `sec:filings` | `examples/datasets/sec_financial_facts/cells.jsonl` | Retrieve filing facts and verify revenue statements with normalized numeric values. |
| Support Ticket Triage | `support:tickets` | `examples/datasets/support_tickets/cells.jsonl` | Retrieve customer issues, remember workflow results, and verify documented support resolutions. |
| Technical Runbook Triage | `docs:technical` | `examples/datasets/technical_docs/cells.jsonl` | Retrieve operational docs, surface tool hints, verify version conflicts, and preserve source refs for compatibility runbooks. |

## Manual Smoke

```bash
cargo run -p cortex-cli -- load-fixture target/use-case-packs/investment-projects/db examples/datasets/investment_projects
cargo run -p cortex-cli -- context --format json target/use-case-packs/investment-projects/db project:investments \
  'RETRIEVE CONTEXT FOR TASK "Mirny wind farm battery evidence" IN BRAIN default REQUIRE citations LIMIT 10 CANDIDATES;'
cargo run -p cortex-cli -- verify --format json target/use-case-packs/investment-projects/db project:investments \
  'VERIFY FACT "Mirny wind farm includes a 600 MWh battery system" IN BRAIN default;'
```

```bash
cargo run -p cortex-cli -- load-fixture target/use-case-packs/legal-policy-review/db examples/datasets/legal_policies
cargo run -p cortex-cli -- context --format json target/use-case-packs/legal-policy-review/db project:legal \
  'RETRIEVE CONTEXT FOR TASK "affiliate approval policy" IN BRAIN default REQUIRE citations LIMIT 10 CANDIDATES;'

cargo run -p cortex-cli -- load-fixture target/use-case-packs/financial-filing-review/db examples/datasets/sec_financial_facts
cargo run -p cortex-cli -- verify --format json target/use-case-packs/financial-filing-review/db sec:filings \
  'VERIFY FACT "Tesla reported full year 2024 total revenues of 96.77B USD" IN BRAIN default;'

cargo run -p cortex-cli -- load-fixture target/use-case-packs/support-ticket-triage/db examples/datasets/support_tickets
cargo run -p cortex-cli -- --json remember target/use-case-packs/support-ticket-triage/db support:tickets \
  'REMEMBER "For repeated authentication failures, check token issuer drift before cache invalidation" IN SCOPE support:tickets AS TYPE workflow_result TTL 1209600 SECONDS;'
cargo run -p cortex-cli -- verify --format json target/use-case-packs/support-ticket-triage/db support:tickets \
  'VERIFY FACT "The authentication outage was mitigated by rotating the signing key" IN BRAIN default;'

cargo run -p cortex-cli -- load-fixture target/use-case-packs/technical-runbook-triage/db examples/datasets/technical_docs
cargo run -p cortex-cli -- search --json target/use-case-packs/technical-runbook-triage/db docs:technical "compatibility endpoint"
cargo run -p cortex-cli -- context --format json target/use-case-packs/technical-runbook-triage/db docs:technical \
  'RETRIEVE CONTEXT FOR TASK "Find tool hints for compatibility diagnostics" IN BRAIN default REQUIRE citations LIMIT 10 CANDIDATES;'
cargo run -p cortex-cli -- verify --format json target/use-case-packs/technical-runbook-triage/db docs:technical \
  'VERIFY FACT "SDK contract v1.4 is incompatible with API contract v1.3" IN BRAIN default;'
```

## Boundary

These packs prove:

- fixtures can be loaded through the CLI;
- ContextPack generation works over domain-specific scopes;
- VERIFY FACT returns deterministic JSON over legal, financial, and technical
  scenarios;
- search can retrieve the expected scoped examples.
- the investment-project pack links demo queries to the local real-domain
  benchmark report and `production_safe=true` embedding evidence.
- the support-ticket pack records a workflow memory update and verifies a
  documented resolution from ticket evidence.
- the technical-docs pack retrieves docs, exposes tool hints, verifies version
  conflicts, and carries source refs for compatibility evidence.

These packs do not prove:

- legal advice, legal-grade verification, or compliance certification;
- audited financial assurance;
- investment advice, project diligence, or source-freshness certification;
- customer-support SLA compliance or incident-response certification;
- production incident-management readiness;
- migration certification or operational approval from technical docs examples;
- private customer-domain quality.

## Files

- Manifest: [`../examples/use_cases/packs.json`](../../examples/use_cases/packs.json)
- Investment pack: [`../examples/use_cases/investment_projects/README.md`](../../examples/use_cases/investment_projects/README.md)
- Legal pack: [`../examples/use_cases/legal_policy_review/README.md`](../../examples/use_cases/legal_policy_review/README.md)
- Financial pack: [`../examples/use_cases/financial_filing_review/README.md`](../../examples/use_cases/financial_filing_review/README.md)
- Support pack: [`../examples/use_cases/support_ticket_triage/README.md`](../../examples/use_cases/support_ticket_triage/README.md)
- Technical pack: [`../examples/use_cases/technical_runbook_triage/README.md`](../../examples/use_cases/technical_runbook_triage/README.md)
- Investment benchmark report: [`../examples/use_cases/investment_projects/benchmark_report.md`](../../examples/use_cases/investment_projects/benchmark_report.md)
- Gate script: [`../scripts/use_case_pack_check.py`](../../scripts/use_case_pack_check.py)
