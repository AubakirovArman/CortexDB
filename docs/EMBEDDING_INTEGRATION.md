# Server-Side Embedding Integration

CortexDB can generate request-time query vectors for semantic context retrieval
and vector search when the server is configured with an HTTP embedding endpoint.

## Configuration

Set these environment variables before starting `cortex-server`:

| Variable | Required | Notes |
| --- | --- | --- |
| `CORTEXDB_EMBEDDING_URL` | yes | OpenAI-compatible HTTP embeddings endpoint. Local `http://127.0.0.1:...` endpoints are supported without extra TLS dependencies. |
| `CORTEXDB_EMBEDDING_MODEL` | no | Included as `model` in the JSON request when set. |
| `CORTEXDB_EMBEDDING_API_KEY` | no | Sent as `Authorization: Bearer ...` when set. |
| `CORTEXDB_EMBEDDING_TIMEOUT_MS` | no | Request connect/read/write timeout. Defaults to `2000`. |

The server sends:

```json
{"input":"semantic query","model":"configured-model"}
```

Accepted provider response shapes:

```json
{"vector":[0,100]}
{"embedding":[0.0,1.0]}
{"data":[{"embedding":[0.0,1.0]}]}
```

Floating point embeddings are clamped to `[-1.0, 1.0]` and scaled into signed
Q15-ish `i16` literals for existing vector indexes.

## Request Behavior

`POST /v1/context` accepts either plain AQL or JSON:

```json
{
  "retrieve_aql": "RETRIEVE CONTEXT FOR TASK \"semantic lookup\" IN BRAIN default USING MODE semantic LIMIT 10 CANDIDATES;",
  "embed_query": true
}
```

When `embed_query=true`, CortexDB embeds `query_text` if provided, otherwise the
AQL task text, and injects `query_vector=...` into the task before compiling.

`POST /v1/search` and `POST /v1/search/explain` accept `embed_query=true` with
`q=...` when `vector=...` is omitted.

## Failure Policy

Semantic/vector requests fail closed when no manual vector is supplied and no
embedding config is available:

```text
semantic requires vector or embedding config
```

Provider errors and timeouts are returned as `400 bad_request`. CortexDB does
not silently downgrade semantic or hybrid requests to lexical search; use
`mode=keyword` explicitly when lexical fallback is desired.
