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

The v0 report has:

- `fact`
- `status`
- `evidence`

For now the engine returns `Supported` when readable evidence overlaps the fact
terms, otherwise `Insufficient`. The enum reserves `Contradicted` and `Mixed`,
but contradiction detection is not implemented yet.

## Not Yet

- Contradiction extraction.
- Source trust scoring.
- Citation quality scoring.
- Numeric guard checks.
