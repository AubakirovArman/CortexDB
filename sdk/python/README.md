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
context = client.context_response("default", 'RETRIEVE CONTEXT FOR TASK "hello" IN BRAIN default LIMIT 10 CANDIDATES;')
verify = client.verify_response("default", 'VERIFY FACT "hello" IN BRAIN default;')
remember = client.remember_response("default", 'REMEMBER "hello" IN SCOPE default AS TYPE decision TTL 3600 SECONDS;')
print(context.token_budget_tokens, verify.status, remember.cell_id)
print(client.ingest_text("default", "hello from sdk"))
```

The package metadata is prepared for PyPI as `cortexdb-client`. Publication is
not automatic; run `../publish/check.sh` before cutting a package release.
Run `make sdk-contract-check` from the repository root to validate this client
against a freshly built local `cortex-server` together with the TypeScript and
Rust SDKs.
