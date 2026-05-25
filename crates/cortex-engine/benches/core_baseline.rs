use std::collections::BTreeSet;
use std::time::Instant;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{scope_id, ContextPackOptions, Database};

const CELL_COUNT: u64 = 1000;

fn main() {
    let dir = tempfile::tempdir().expect("tempdir");

    // 1. put_1k_cells
    let mut db = Database::open(dir.path()).expect("open database");
    let put_start = Instant::now();
    for id in 1..=CELL_COUNT {
        db.put_cell(CellId(id), payload(id)).expect("put cell");
    }
    let put_elapsed = put_start.elapsed();

    // 2. get_1k_cells
    let get_start = Instant::now();
    for id in 1..=CELL_COUNT {
        assert!(db.get_latest_cell(CellId(id)).is_some());
    }
    let get_elapsed = get_start.elapsed();

    // 3. checkpoint_1k
    let checkpoint_start = Instant::now();
    db.checkpoint().expect("checkpoint");
    let checkpoint_elapsed = checkpoint_start.elapsed();
    drop(db);

    // 4. restart_replay_1k
    let replay_start = Instant::now();
    let mut db = Database::open(dir.path()).expect("reopen database");
    let replay_elapsed = replay_start.elapsed();

    // 5. compact_1k
    let compact_start = Instant::now();
    db.compact().expect("compact");
    let compact_elapsed = compact_start.elapsed();

    // 6. aql_retrieve_1k
    let aql_start = Instant::now();
    let retrieved = db.retrieve_aql(query(), &view()).expect("aql retrieve");
    let aql_elapsed = aql_start.elapsed();
    assert!(!retrieved.is_empty());

    // 7. context_pack_1k
    let context_start = Instant::now();
    let pack = db
        .context_pack_from_aql(query(), &view(), ContextPackOptions::default())
        .expect("context pack");
    let context_elapsed = context_start.elapsed();
    assert!(!pack.cells.is_empty());

    println!("================================================");
    println!("CORTEXDB CORE ALPHA BENCHMARK BASELINE (1K CELLS)");
    println!("================================================");
    println!("put_1k_cells:       {:?}", put_elapsed);
    println!("get_1k_cells:       {:?}", get_elapsed);
    println!("checkpoint_1k:      {:?}", checkpoint_elapsed);
    println!("restart_replay_1k:  {:?}", replay_elapsed);
    println!("compact_1k:         {:?}", compact_elapsed);
    println!("aql_retrieve_1k:    {:?}", aql_elapsed);
    println!("context_pack_1k:    {:?}", context_elapsed);
    println!("================================================");
}

fn payload(id: u64) -> Vec<u8> {
    format!("scope=bench\nstatus=ready\nsource=bench-{id}\ncell {id} budget ready").into_bytes()
}

fn query() -> &'static str {
    r#"RETRIEVE CONTEXT FOR TASK "bench" IN BRAIN investment_projects
WHERE space = bench AND status = "ready" LIMIT 100 CANDIDATES;"#
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
        max_context_budget_tokens: 100_000,
        default_context_budget_tokens: 50_000,
        max_candidate_limit: 1000,
        default_candidate_limit: 100,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3_600),
        allow_remember: false,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: true,
        private_scope: None,
    }
}
