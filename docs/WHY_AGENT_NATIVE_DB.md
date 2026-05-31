# Why Agent-Native DB?

In the era of AI and Large Language Models, traditional database designs no longer fit the natural workflows of autonomous agents.

---

## The Paradigm Shift

| Database Type | Primary Output | Typical Query | Missing Context |
| --- | --- | --- | --- |
| **Traditional SQL/NoSQL** | Rows/Tables | `SELECT * FROM sales;` | No LLM awareness, no budget controls, raw structured data. |
| **Vector Databases** | Nearest Chunks | `search(query_vector)` | Broken citations, redundant passages, token budget overflow. |
| **Agent-Native DB (CortexDB)** | **Context Packs** | `RETRIEVE CONTEXT...` | **Strict token budget, citation verification, anomaly reporting, deterministic fact checking.** |

---

## Introducing Context Packs

CortexDB does not just return nearest neighbors. It compiles a **Context Pack**:
1. **Budget-Aware:** Limits candidate cells to fit within the agent's exact token window.
2. **Deterministic Fact Verification:** Runs `VERIFY FACT` with citation/numeric guards to spot hallucinations.
3. **Redundancy-Safe:** Uses fixed-point/lexical cosine and Jaccard deduplication to strip out duplicate semantic content.
4. **Citations First:** Every factual claim is bound to a verifiable provenance marker (`source=file.pdf#page=3`).

This is the foundational memory layer for agent workflows that need bounded,
auditable, source-grounded context. The detailed technology overview is in
[`CONTEXT_PACK_TECHNOLOGY.md`](CONTEXT_PACK_TECHNOLOGY.md).
