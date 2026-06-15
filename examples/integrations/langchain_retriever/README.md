# LangChain-Style Retriever

This example keeps the LangChain surface small and dependency-free. It exposes a
`CortexRetriever.invoke(query)` method that returns `Document` objects with
`page_content` and `metadata`, matching the shape LangChain retrievers consume.

Run:

```bash
python3 examples/integrations/langchain_retriever/demo.py --self-test
```

Use it as the boundary behind your actual LangChain chain:

```python
retriever = CortexRetriever(db_path, "project:investments")
docs = retriever.invoke("Solar Plant budget")
```

The smoke path uses a mock answerer and the local CortexDB CLI.
