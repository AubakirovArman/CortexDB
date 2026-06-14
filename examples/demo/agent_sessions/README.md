# Agent Sessions Demo

Agent sessions are explicit short-lived memory cells grouped by `session_id`.
Use them when an agent needs task-local context across turns without promoting
that context into long-term memory.

```rust
use cortex_aql::{AgentId, AgentView, MemoryType};
use cortex_engine::{scope_id, Database};

let mut db = Database::open("./agent-session-db")?;
let view = AgentView {
    agent_id: AgentId(7),
    label: Some("finance-agent".to_owned()),
    readable_brains: Default::default(),
    readable_scopes: [scope_id("agent:finance")].into_iter().collect(),
    writable_scopes: [scope_id("agent:finance")].into_iter().collect(),
    allowed_modes: Default::default(),
    allowed_memory_types: [MemoryType::WorkflowResult, MemoryType::Observation]
        .into_iter()
        .collect(),
    max_context_budget_tokens: 10_000,
    default_context_budget_tokens: 1_000,
    max_candidate_limit: 100,
    default_candidate_limit: 10,
    min_required_confidence_q16: Default::default(),
    max_ttl_seconds: Some(3600),
    allow_remember: true,
    allow_verify_fact: false,
    allow_audit_mode: false,
    require_citations_by_default: false,
    private_scope: Some(scope_id("agent:finance")),
};

let session = db.start_agent_session(
    &view,
    "agent:finance",
    b"Review capex evidence before answering.",
    3600,
    1_760_000_000,
)?;

db.remember_session_memory(
    &session,
    &view,
    b"Temporary note: compare budget, deadline, and source confidence.",
    None,
    1_760_000_060,
)?;

let cells = db.retrieve_session_cells(&session.session_id, &view, 1_760_000_120);
assert_eq!(cells.len(), 2);
# Ok::<(), cortex_engine::EngineError>(())
```

The session contract is descriptor-backed:

- `session_id` and `session_kind` are stored in `CellDescriptor`;
- `AgentView` scope checks use descriptor scope, not spoofable payload headers;
- lazy checkpoint reopen indexes session cells without loading every payload;
- expired session cells are excluded by TTL at retrieval time.

Regression coverage:

```bash
cargo test -p cortex-engine --test agent_session_tests --test agent_session_lazy_tests --all-features
```
