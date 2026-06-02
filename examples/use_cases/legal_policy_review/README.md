# Legal Policy Review Pack

Goal: show how CortexDB retrieves cited policy context and verifies a policy
update without claiming legal advice.

Scope: `project:legal`

Fixture:

```text
examples/datasets/legal_policies/cells.jsonl
```

Run:

```bash
cargo run -p cortex-cli -- load-fixture target/use-case-packs/legal-policy-review/db examples/datasets/legal_policies
cargo run -p cortex-cli -- search --json target/use-case-packs/legal-policy-review/db project:legal "affiliate contract approval"
cargo run -p cortex-cli -- context --format json target/use-case-packs/legal-policy-review/db project:legal \
  'RETRIEVE CONTEXT FOR TASK "affiliate approval policy" IN BRAIN default REQUIRE citations LIMIT 10 CANDIDATES;'
cargo run -p cortex-cli -- verify --format json target/use-case-packs/legal-policy-review/db project:legal \
  'VERIFY FACT "All affiliate contracts must be approved by legal department before signature" IN BRAIN default;'
```

Boundary: this pack is a developer scenario. It is not legal advice, not legal
review, and not legal-grade verification.
