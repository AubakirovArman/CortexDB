# Getting Started

This path takes a fresh checkout to a first ContextPack in under five minutes.
It uses only local files and does not require the HTTP server.

## 1. Build The CLI

```bash
cargo build -p cortex-cli
```

## 2. Load The Investment Projects Fixture

```bash
rm -rf target/getting-started-demo
cargo run -q -p cortex-cli -- load-fixture target/getting-started-demo examples/datasets/investment_projects
```

## 3. Inspect The Database

```bash
cargo run -q -p cortex-cli -- stats target/getting-started-demo
```

## 4. Search As The Finance Agent

```bash
cargo run -q -p cortex-cli -- search --json target/getting-started-demo project:investments "Solar Plant budget"
```

The finance scope can see the investment project cells, including the Q1 and Q2
budget evidence.

## 5. Build A ContextPack

```bash
cargo run -q -p cortex-cli -- context --format json target/getting-started-demo project:investments \
  'RETRIEVE CONTEXT FOR TASK "Solar Plant budget evidence" IN BRAIN default REQUIRE citations LIMIT 10 CANDIDATES;'
```

The response is a `context_pack.v1` object with selected cells, citations,
token accounting, answerability, and conflict visibility.

## 6. Verify A Numeric Claim

```bash
cargo run -q -p cortex-cli -- verify --format json target/getting-started-demo project:investments \
  'VERIFY FACT "Solar Plant budget is 1.2B KZT" IN BRAIN default;'
```

The expected verdict is `mixed_evidence`: Q1 supports `1.2B KZT`, while Q2
contradicts it with `1.4B KZT`.

## 7. Search As A Different Agent

```bash
cargo run -q -p cortex-cli -- search --json target/getting-started-demo agent:hr "Solar Plant budget"
```

The HR scope returns an empty result set. This is the core safety behavior:
retrieval is bounded by the AgentView scope before context is built.

## 8. Run The Flagship Demo

```bash
make demo
```

The demo shows the same product story in one script: scoped retrieval,
ContextPack output, deterministic verification, and storage validation.

## Check This Guide

```bash
make getting-started-check
```
