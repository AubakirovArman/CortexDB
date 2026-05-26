# cortexdb-client

Stdlib Python client for the Core Alpha CortexDB HTTP API.

```python
from cortexdb_client import CortexDBClient

client = CortexDBClient("http://127.0.0.1:8181")
client.put_cell(1, "scope=default\nstatus=ready\nhello")
print(client.get_cell(1))
print(client.search("default", "hello"))
print(client.ingest_text("default", "hello from sdk"))
```

The package metadata is prepared for PyPI as `cortexdb-client`. Publication is
not automatic; run `../publish/check.sh` before cutting a package release.
