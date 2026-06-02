# Financial Filing Review Pack

Goal: show how CortexDB retrieves filing facts and verifies a numeric revenue
statement with deterministic evidence.

Scope: `sec:filings`

Fixture:

```text
examples/datasets/sec_financial_facts/cells.jsonl
```

Run:

```bash
cargo run -p cortex-cli -- load-fixture target/use-case-packs/financial-filing-review/db examples/datasets/sec_financial_facts
cargo run -p cortex-cli -- search --json target/use-case-packs/financial-filing-review/db sec:filings "Tesla 2024 revenue"
cargo run -p cortex-cli -- context --format json target/use-case-packs/financial-filing-review/db sec:filings \
  'RETRIEVE CONTEXT FOR TASK "Tesla annual revenue" IN BRAIN default REQUIRE citations LIMIT 10 CANDIDATES;'
cargo run -p cortex-cli -- verify --format json target/use-case-packs/financial-filing-review/db sec:filings \
  'VERIFY FACT "Tesla reported full year 2024 total revenues of 96.77B USD" IN BRAIN default;'
```

Boundary: this pack is not audited financial assurance and does not replace
review of official filings.
