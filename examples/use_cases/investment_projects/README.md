# Investment Projects Pack

Goal: show how CortexDB retrieves project evidence, builds a cited
ContextPack, and verifies a concrete infrastructure fact for Kazakhstan and
Central Asia investment-project analysis.

Scope: `project:investments`

Fixture:

```text
examples/datasets/investment_projects/cells.jsonl
```

## Demo

The full local demo is:

```bash
./examples/demo/investment_projects/run.sh
```

It loads investment-project cells, flushes storage, runs scoped search,
generates a ContextPack, verifies a numeric budget claim, validates storage,
and removes its temporary database.

## Queries

The pack has three runnable query paths:

```bash
cargo run -p cortex-cli -- load-fixture target/use-case-packs/investment-projects/db examples/datasets/investment_projects

cargo run -p cortex-cli -- search --json target/use-case-packs/investment-projects/db project:investments \
  "Kazakhstan renewable energy battery"

cargo run -p cortex-cli -- context --format json target/use-case-packs/investment-projects/db project:investments \
  'RETRIEVE CONTEXT FOR TASK "Mirny wind farm battery evidence" IN BRAIN default REQUIRE citations LIMIT 10 CANDIDATES;'

cargo run -p cortex-cli -- verify --format json target/use-case-packs/investment-projects/db project:investments \
  'VERIFY FACT "Mirny wind farm includes a 600 MWh battery system" IN BRAIN default;'
```

Additional AQL examples live in:

```text
examples/aql/investment_projects/
```

The real-domain retrieval query set lives in:

```text
examples/real_domains/investment_projects/queries/queries.jsonl
examples/real_domains/investment_projects/queries/ground_truth.jsonl
```

## ContextPack Example

The ContextPack example asks for Mirny battery evidence and requires
citations. The expected domain evidence includes:

```text
Mirny Wind Farm
TotalEnergies
600 MWh battery system
```

## VERIFY Example

The VERIFY example checks:

```text
Mirny wind farm includes a 600 MWh battery system
```

Expected behavior: the report returns supporting evidence from the
`project:investments` scope rather than private or unrelated cells.

## Benchmark

The benchmark evidence for this pack is documented in:

```text
examples/use_cases/investment_projects/benchmark_report.md
```

Boundary: this pack is a developer scenario and benchmark fixture. It is not
investment advice, financial assurance, or a certification of source accuracy.
