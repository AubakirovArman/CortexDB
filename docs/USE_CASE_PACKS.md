# CortexDB Use-case Packs

Status: runnable beta scenarios for legal, financial, and technical use cases.

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
| Legal Policy Review | `project:legal` | `examples/datasets/legal_policies/cells.jsonl` | Retrieve cited policy context and verify policy-update facts without claiming legal advice. |
| Financial Filing Review | `sec:filings` | `examples/datasets/sec_financial_facts/cells.jsonl` | Retrieve filing facts and verify revenue statements with normalized numeric values. |
| Technical Runbook Triage | `docs:technical` | `examples/datasets/technical_docs/cells.jsonl` | Retrieve operational docs for compatibility, beta release, and search explain workflows. |

## Manual Smoke

```bash
cargo run -p cortex-cli -- load-fixture target/use-case-packs/legal-policy-review/db examples/datasets/legal_policies
cargo run -p cortex-cli -- context --format json target/use-case-packs/legal-policy-review/db project:legal \
  'RETRIEVE CONTEXT FOR TASK "affiliate approval policy" IN BRAIN default REQUIRE citations LIMIT 10 CANDIDATES;'

cargo run -p cortex-cli -- load-fixture target/use-case-packs/financial-filing-review/db examples/datasets/sec_financial_facts
cargo run -p cortex-cli -- verify --format json target/use-case-packs/financial-filing-review/db sec:filings \
  'VERIFY FACT "Tesla reported full year 2024 total revenues of 96.77B USD" IN BRAIN default;'

cargo run -p cortex-cli -- load-fixture target/use-case-packs/technical-runbook-triage/db examples/datasets/technical_docs
cargo run -p cortex-cli -- search --json target/use-case-packs/technical-runbook-triage/db docs:technical "compatibility endpoint"
```

## Boundary

These packs prove:

- fixtures can be loaded through the CLI;
- ContextPack generation works over domain-specific scopes;
- VERIFY FACT returns deterministic JSON over legal, financial, and technical
  scenarios;
- search can retrieve the expected scoped examples.

These packs do not prove:

- legal advice, legal-grade verification, or compliance certification;
- audited financial assurance;
- production incident-management readiness;
- private customer-domain quality.

## Files

- Manifest: [`../examples/use_cases/packs.json`](../examples/use_cases/packs.json)
- Legal pack: [`../examples/use_cases/legal_policy_review/README.md`](../examples/use_cases/legal_policy_review/README.md)
- Financial pack: [`../examples/use_cases/financial_filing_review/README.md`](../examples/use_cases/financial_filing_review/README.md)
- Technical pack: [`../examples/use_cases/technical_runbook_triage/README.md`](../examples/use_cases/technical_runbook_triage/README.md)
- Gate script: [`../scripts/use_case_pack_check.py`](../scripts/use_case_pack_check.py)
