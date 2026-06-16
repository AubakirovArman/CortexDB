# cortexdb-sdk

Stdlib Python client for the Core Alpha CortexDB HTTP API.

```python
from cortexdb_client import ContextPackResponse, CortexDBClient

client = CortexDBClient("http://127.0.0.1:8181").with_retries(3, retry_delay_seconds=0.1).with_timeout(5.0)
with client.with_session() as session:
    tenant_client = session.with_tenant("tenant:alpha")
    session.put_cell(1, "scope=default\nstatus=ready\nhello")
    print(session.get_cell(1))
    print(tenant_client.stats())
    results = session.search_response("default", "hello")
    print(results.search_mode, results.ann_report, results.results)
    ann_eval = session.evaluate_ann_response("default", [1, 2, 3])
    print(ann_eval.available, ann_eval.recall_q16)
    retrieve = session.build_retrieve_context_aql("hello", "default", limit_candidates=10)
    context: ContextPackResponse = session.context_response("default", retrieve)
    verify = session.verify_response("default", session.build_verify_fact_aql("hello", "default"))
    remember = session.remember_response("default", session.build_remember_aql("hello", "default", "decision", ttl_seconds=3600))
    print(context.token_budget_tokens, verify.status, remember.cell_id)
    print(session.ingest_text("default", "hello from sdk"))

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

The package metadata is prepared for PyPI as `cortexdb-sdk`. Publication is
not automatic; run `../publish/check.sh` before cutting a package release.
Run `make sdk-contract-check` from the repository root to validate this client
against a freshly built local `cortex-server` together with the TypeScript and
Rust SDKs.
