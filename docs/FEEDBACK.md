# Context Feedback v0

Feedback can now be stored as first-class durable cells:

```rust
db.record_context_feedback(agent_id, ContextFeedback {
    source_cell_id,
    useful: true,
    note: Some("good context".to_owned()),
})?;
```

The write path is the normal durable database path:

```text
record_context_feedback
-> KnowledgeCellType::Feedback
-> WAL append
-> MemTable update
-> restart replay
```

The current payload bridge writes:

```text
scope=agent:7
status=ready
type=feedback
source=cell:42

source_cell_id=42
useful=true
note=good context
```

Feedback is queryable with ordinary AQL filters, for example:

```text
WHERE scope = agent:7 AND type = "feedback"
```

## Not Yet

- Feedback weighting in ContextPack selection.
- Aggregated usefulness statistics.
- AgentView persistence for feedback scopes.
