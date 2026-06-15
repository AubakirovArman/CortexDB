# CortexDB Live Integration Examples

These examples show the common user path:

```text
application or agent -> CortexDB retrieve/verify/remember -> grounded answer
```

They run locally with the CortexDB CLI and a mock LLM, so the smoke check needs
no provider keys and no network calls.

## Examples

- [`llm_tool_calling`](llm_tool_calling/README.md) - OpenAI and Anthropic style
  tool definitions for `retrieve_context` and `verify_fact`.
- [`langchain_retriever`](langchain_retriever/README.md) - a LangChain-style
  retriever adapter that returns `Document` objects from a ContextPack.
- [`memory_chat_agent`](memory_chat_agent/README.md) - a small chat agent that
  writes a TTL memory, retrieves context, and verifies a claim before answering.

## Smoke

```bash
make live-integration-examples-check
```

The target builds `cortexdb`, runs each example with `--self-test`, and writes a
report to `target/live-integration-examples/report.json`.
