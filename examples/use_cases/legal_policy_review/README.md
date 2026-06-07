# Legal Policy Review Pack

Goal: show how CortexDB retrieves cited policy context, verifies policy
updates, and surfaces contradiction evidence without claiming legal advice.

Scope: `project:legal`

Fixture:

```text
examples/datasets/legal_policies/cells.jsonl
```

## Corpus

The pack has two corpus layers:

```text
examples/datasets/legal_policies/cells.jsonl
examples/real_domains/legal_policies/
```

The fixture is a small runnable policy scenario. The real-domain folder adds a
validated synthetic retrieval corpus with documents, chunks, queries, and
ground truth rows.

## Demo

Run the full local demo:

```bash
./examples/demo/legal_policies/run.sh
```

## Search Demo

```bash
cargo run -p cortex-cli -- load-fixture target/use-case-packs/legal-policy-review/db examples/datasets/legal_policies
cargo run -p cortex-cli -- search --json target/use-case-packs/legal-policy-review/db project:legal "affiliate contract approval"
```

## ContextPack Demo

```bash
cargo run -p cortex-cli -- context --format json target/use-case-packs/legal-policy-review/db project:legal \
  'RETRIEVE CONTEXT FOR TASK "affiliate approval policy" IN BRAIN default REQUIRE citations LIMIT 10 CANDIDATES;'
```

The ContextPack command requires citations and should surface `POL-01` policy
evidence with source references.

## VERIFY Examples

Supported policy:

```bash
cargo run -p cortex-cli -- verify --format json target/use-case-packs/legal-policy-review/db project:legal \
  'VERIFY FACT "All affiliate contracts must be approved by legal department before signature" IN BRAIN default;'
```

Contradiction demo:

```bash
cargo run -p cortex-cli -- verify --format json target/use-case-packs/legal-policy-review/db project:legal \
  'VERIFY FACT "Low-risk affiliate contracts below 50000 USD could be approved by procurement without legal department approval before signature" IN BRAIN default;'
```

Expected behavior: the contradiction report includes `mixed_evidence` or
contradicting evidence when the current POL-01 approval rule and legacy
exception are both visible.

## Citation Demo

The fixture includes `source=` and `citation=` metadata so citation-required
ContextPack and VERIFY flows can show which policy file supports each answer.
The explicit citation fixture is:

```text
policy_v2_appendix.pdf#page=3
```

Boundary: this pack is a developer scenario. It is not legal advice, not legal
review, and not legal-grade verification.
