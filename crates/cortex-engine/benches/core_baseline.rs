use std::collections::BTreeSet;
use std::time::Instant;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{scope_id, ContextPackOptions, Database};

const CELL_COUNT: u64 = 256;

fn main() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut db = Database::open(dir.path()).expect("open database");

    let put_start = Instant::now();
    for id in 1..=CELL_COUNT {
        db.put_cell(CellId(id), payload(id)).expect("put cell");
    }
    let put_elapsed = put_start.elapsed();

    let get_start = Instant::now();
    for id in 1..=CELL_COUNT {
        assert!(db.get_latest_cell(CellId(id)).is_some());
    }
    let get_elapsed = get_start.elapsed();

    let checkpoint_start = Instant::now();
    db.checkpoint().expect("checkpoint");
    let checkpoint_elapsed = checkpoint_start.elapsed();
    drop(db);

    let replay_start = Instant::now();
    let db = Database::open(dir.path()).expect("reopen database");
    let replay_elapsed = replay_start.elapsed();

    let context_start = Instant::now();
    let pack = db
        .context_pack_from_aql(query(), &view(), ContextPackOptions::default())
        .expect("context pack");
    let context_elapsed = context_start.elapsed();
    assert!(!pack.cells.is_empty());

    println!("cortexdb core baseline");
    println!("put {CELL_COUNT} cells: {:?}", put_elapsed);
    println!("get {CELL_COUNT} cells: {:?}", get_elapsed);
    println!("checkpoint {CELL_COUNT} cells: {:?}", checkpoint_elapsed);
    println!("reopen from checkpoint: {:?}", replay_elapsed);
    println!("context pack: {:?}", context_elapsed);
}

fn payload(id: u64) -> Vec<u8> {
    format!("scope=bench\nstatus=ready\nsource=bench-{id}\ncell {id} budget ready").into_bytes()
}

fn query() -> &'static str {
    r#"RETRIEVE CONTEXT FOR TASK "bench" IN BRAIN investment_projects
WHERE space = bench AND status = "ready" LIMIT 32 CANDIDATES;"#
}

fn view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("bench".to_owned()),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id("bench")]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 4_000,
        default_context_budget_tokens: 1_000,
        max_candidate_limit: 100,
        default_candidate_limit: 32,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3_600),
        allow_remember: false,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: true,
        private_scope: None,
    }
}
