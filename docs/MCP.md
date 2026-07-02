# CortexDB MCP Adapter

`cortex-mcp` exposes CortexDB to MCP-compatible agents over stdio. It is a
thin adapter over the Rust SDK and does not bypass CortexDB server auth,
tenant routing, AgentView policy, or request limits.

## Tools

The adapter exposes the governed agent surface plus permission-scoped search:

| Tool | Purpose |
|---|---|
| `retrieve_context` | Build and execute `RETRIEVE CONTEXT` and return a ContextPack, prompt, or markdown. |
| `verify_fact` | Build and execute deterministic `VERIFY FACT`. |
| `remember` | Build and execute policy-checked `REMEMBER`. |
| `search` | Permission-scoped search returning ranked cells (not a governed ContextPack). Defaults to keyword; `mode=semantic\|hybrid\|auto` searches by meaning. |

Raw `put` is not an MCP tool. Prefer `retrieve_context` for grounded agent
context: unlike `search`, it returns a permission-filtered, token-budgeted,
cited ContextPack. `search` is offered for exploration and cell lookup; it is
still AgentView-scoped through the server and cannot read outside the caller's
readable scopes.

`search` takes an optional `mode`:

| `mode` | Behavior |
|---|---|
| `keyword` (default) | Lexical BM25 search. |
| `semantic` | Embeds the query text server-side and runs vector search. |
| `hybrid` | Embeds the query and blends lexical + vector. |
| `auto` | Embeds the query and lets the server pick the strategy. |

The non-keyword modes require an embedding endpoint configured on the server
(`CORTEXDB_EMBEDDING_*`); with none configured the call fails closed rather than
silently returning keyword results. The agent never supplies a vector — the
server embeds the query at request time.

## AgentView Mapping

MCP config maps to CortexDB access like this:

| MCP setting | CortexDB effect |
|---|---|
| `CORTEXDB_MCP_BASE_URL` | HTTP endpoint for the CortexDB server. |
| `CORTEXDB_MCP_AUTH_TOKEN` | Bearer token. The server maps this token to `AuthRole` and optional `AgentView`/agent id. |
| `CORTEXDB_MCP_TENANT` | Tenant query parameter for per-tenant database routing. |
| `CORTEXDB_MCP_SCOPE` | Default scope passed to SDK calls when a tool call omits `scope`. |
| `CORTEXDB_MCP_BRAIN` | Default AQL brain when a tool call omits `brain`. |

If auth is enabled, configure the server token policy first. For example,
`data:agent-token:7` maps the token to data role with agent id `7`; the MCP
adapter only sends the Bearer token.

## Quickstart

Start a local CortexDB server:

```bash
cargo run -p cortex-server -- ./demo-db --listen 127.0.0.1:8181
```

Run a stdio smoke check:

```bash
printf '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}\n' \
  | cargo run -p cortex-mcp --
```

The response should contain `retrieve_context`, `verify_fact`, `remember`, and
`search`.

## Claude Code / IDE Config

Use the example config at
[`examples/mcp/claude-code.json`](../examples/mcp/claude-code.json), or add the
same server entry to your MCP-capable IDE:

```json
{
  "mcpServers": {
    "cortexdb": {
      "command": "cargo",
      "args": ["run", "-p", "cortex-mcp", "--"],
      "env": {
        "CORTEXDB_MCP_BASE_URL": "http://127.0.0.1:8181",
        "CORTEXDB_MCP_AUTH_TOKEN": "replace-with-token",
        "CORTEXDB_MCP_TENANT": "default",
        "CORTEXDB_MCP_SCOPE": "project:investments",
        "CORTEXDB_MCP_BRAIN": "default"
      }
    }
  }
}
```

For an installed binary, replace the command with `cortex-mcp` and remove the
Cargo arguments.

## Example Tool Calls

Retrieve context:

```json
{
  "task": "Which evidence supports the project budget?",
  "scope": "project:investments",
  "brain": "default",
  "mode": "balanced",
  "require_citations": true,
  "format": "markdown"
}
```

Verify a fact:

```json
{
  "fact": "Solar Plant budget is 1.2B KZT",
  "scope": "project:investments",
  "brain": "default"
}
```

Remember an observation:

```json
{
  "content": "For budget reviews, compare source trust before final answer generation",
  "scope": "project:investments",
  "memory_type": "workflow_result",
  "ttl_seconds": 604800
}
```

## Boundaries

- The adapter is synchronous and local-process oriented.
- Tool responses are text MCP content blocks.
- Permission checks remain in the CortexDB server.
- Use server audit logs to inspect who called the underlying HTTP endpoints.
