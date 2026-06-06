# AQL Query Examples Pack

These examples are checked by `crates/cortex-aql/tests/aql_examples_pack.rs`.
Each `.aql` file contains exactly one AQL v0.4 statement and should be usable as
a CLI or HTTP query body after loading matching domain data.

Domains:

- `investment_projects`: project finance, budgets, risk, and source-trust checks.
- `legal_policies`: policy lookup and human-review memory.
- `support_tickets`: incident triage and support workflow memory.
- `technical_docs`: API, SDK, storage, and runbook retrieval.

Run the parser gate:

```bash
cargo test -p cortex-aql --test aql_examples_pack
```
