# VERIFY FACT v0

`Database::verify_fact_aql` executes the existing AQL `VERIFY FACT` statement:

```text
VERIFY FACT "ABC budget approved" IN BRAIN investment_projects;
```

The path is policy checked:

```text
AQL VERIFY FACT
-> parser
-> binder and AgentView policy
-> visible snapshot scan
-> readable-scope filter
-> term-overlap evidence report
```

The report path is restart-safe because it reads the same MemTable snapshot
rebuilt from checkpoint plus WAL replay.

The v0 report has:

- `fact`
- `status`
- `evidence`
- `contradicting_evidence`

The engine returns `Supported` when readable evidence overlaps the fact terms,
`Contradicted` when readable cells declare a contradiction, `Mixed` when both
signals are present, and `Insufficient` when neither signal is visible.

Evidence includes `source_trust_q16`. If a payload has `source_trust_q16=<u16>`,
that value is used as an integer trust signal. Evidence with the same term match
count is ordered by higher trust first. Missing trust defaults to `32768`.

Contradiction v0 uses an explicit payload line:

```text
contradicts=ABC budget approved
```

The marker line is not counted as supporting evidence. It only contributes to
`contradicting_evidence` when it matches all normalized fact terms.

The same structured markers are queryable through:

```rust
let conflicts = db.conflict_index(&view);
let conflicts = db.conflicts_for_fact("ABC budget approved", &view);
```

Smoke surfaces:

```text
cortexdb verify <path> <scope> '<VERIFY FACT ...;>'
POST /v1/verify?scope=<scope>
```

## Not Yet

- Citation quality scoring.
- Numeric guard checks.
- Natural-language contradiction extraction.
