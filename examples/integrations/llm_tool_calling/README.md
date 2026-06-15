# LLM Tool Calling

This example maps CortexDB into OpenAI and Anthropic style tool definitions:

- `retrieve_context` builds a citation-aware ContextPack.
- `verify_fact` checks a claim before the mock LLM writes the final answer.

Run the local smoke path:

```bash
python3 examples/integrations/llm_tool_calling/demo.py --self-test
```

The example uses the local CortexDB CLI and a mock tool-calling model. Replace
`MockToolCallingModel` with your provider client while keeping the tool handler
boundary unchanged.
