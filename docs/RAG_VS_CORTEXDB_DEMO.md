# RAG Demo: Classic RAG vs CortexDB

The beta product demo shows why CortexDB is more than a vector snippet store.

## Scenario

Question:

```text
Какой бюджет у Финансового департамента на 2024 год?
```

The fixture contains finance evidence with a budget conflict:

- approved budget evidence;
- conflicting or superseded budget evidence;
- source citations;
- private or unrelated cells that should not drive the answer.

## Classic RAG Path

Classic RAG usually does:

```text
query -> top-k chunks -> prompt
```

This can retrieve useful text, but the application still has to solve:

- duplicate snippets;
- missing citations;
- private-scope filtering;
- token budget control;
- contradictory evidence detection.

## CortexDB Path

The demo uses:

```text
query -> AQL -> ContextPack -> VERIFY FACT -> evidence-aware prompt
```

The release smoke gate proves the CortexDB side:

```bash
make rag-demo-smoke
```

The smoke test starts a local `cortex-server`, loads the demo data, then checks:

- search returns relevant rows;
- AQL returns cells for the finance scope;
- ContextPack returns cited cells under budget;
- prompt assembly includes the expected budget evidence;
- `VERIFY FACT` returns `mixed_evidence`;
- output matches `examples/rag_demo/expected_output.json`.

## Expected Output Contract

The demo contract requires:

```json
{
  "ok": true,
  "verify_verdict": "mixed_evidence",
  "ingested_records": 74
}
```

Minimum result counts are recorded in
`examples/rag_demo/expected_output.json`. Exact prompt length is not fixed, but
the prompt must include enough evidence to pass the configured minimum.

## Boundary

This demo proves a deterministic local product story. It does not prove hosted
LLM quality, production UI readiness, managed cloud behavior, or legal-grade
verification.

