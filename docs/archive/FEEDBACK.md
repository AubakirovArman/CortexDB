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

The engine also exposes aggregate scores:

```rust
let scores = db.feedback_scores();
let stats = db.feedback_stats();
let decayed_scores = db.feedback_scores_at(now_unix_seconds);
let report = db.feedback_score_report_at(now_unix_seconds);
```

`ContextPack` uses these scores as a deterministic pre-pack ordering signal:
positive feedback moves a cell earlier while preserving original order for ties.
`feedback_stats` reports total useful/not-useful votes and per-source-cell
breakdowns. `feedback_scores_at` applies a deterministic 30-day linear decay to
feedback ranking contribution, so older useful/not-useful votes gradually stop
moving ContextPack candidates. `feedback_score_report_at` exposes the raw vote
count, decayed ranking score, and decay window for each source cell.

ContextPack explain output includes a `feedback_bonus` score component whenever
feedback affects a selected cell:

```text
score_components:
  - base_bm25
  - source_trust_bonus
  - redundancy_penalty
  - feedback_bonus
```

## Not Yet

- AgentView persistence for feedback scopes.
- ML/RL ranking from feedback. Current feedback learning is deterministic,
  fixed-policy scoring.
