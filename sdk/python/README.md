# cortexdb-client

Minimal stdlib Python client for the current CortexDB HTTP API.

```python
from cortexdb_client import CortexDBClient

client = CortexDBClient("http://127.0.0.1:8181")
client.put_cell(1, "scope=default\nstatus=ready\nhello")
print(client.get_cell(1))
print(client.search("default", "hello"))
```
