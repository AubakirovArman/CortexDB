# cortexdb-client

Stdlib Python client for the Core Alpha CortexDB HTTP API.

```python
from cortexdb_client import CortexDBClient

client = CortexDBClient("http://127.0.0.1:8181")
tenant_client = client.with_tenant("tenant:alpha")
client.put_cell(1, "scope=default\nstatus=ready\nhello")
print(client.get_cell(1))
print(tenant_client.stats())
results = client.search_response("default", "hello")
print(results.search_mode, results.ann_report, results.results)
ann_eval = client.evaluate_ann_response("default", [1, 2, 3])
print(ann_eval.available, ann_eval.recall_q16)
retrieve = client.build_retrieve_context_aql("hello", "default", limit_candidates=10)
context = client.context_response("default", retrieve)
verify = client.verify_response("default", client.build_verify_fact_aql("hello", "default"))
remember = client.remember_response("default", client.build_remember_aql("hello", "default", "decision", ttl_seconds=3600))
print(context.token_budget_tokens, verify.status, remember.cell_id)
print(client.ingest_text("default", "hello from sdk"))

grounded = client.answer_with_grounded_context(
    "default",
    "default",
    "What does the context say about hello?",
    lambda pack: pack.cells[0].payload_text if pack.cells else "Not enough information.",
    mode="audit",
    limit_candidates=10,
)
print(grounded.citations, grounded.grounding.answer_supported)
```

The package metadata is prepared for PyPI as `cortexdb-client`. Publication is
not automatic; run `../publish/check.sh` before cutting a package release.
Run `make sdk-contract-check` from the repository root to validate this client
against a freshly built local `cortex-server` together with the TypeScript and
Rust SDKs.
