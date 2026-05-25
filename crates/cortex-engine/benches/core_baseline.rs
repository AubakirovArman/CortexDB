use std::collections::BTreeSet;
use std::time::Instant;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{scope_id, ContextPackOptions, Database, DatabaseOptions};
use cortex_storage::wal::DurabilityMode;

fn main() {
    println!("Starting CortexDB Benchmark Matrix v2...");

    // --- 1K Baseline ---
    let dir_1k = tempfile::tempdir().unwrap();
    let mut db_1k = Database::open(dir_1k.path()).unwrap();

    let put_1k_start = Instant::now();
    for id in 1..=1000 {
        db_1k.put_cell(CellId(id), payload(id)).unwrap();
    }
    let put_1k_time = put_1k_start.elapsed();

    let get_1k_start = Instant::now();
    for id in 1..=1000 {
        assert!(db_1k.get_latest_cell(CellId(id)).is_some());
    }
    let get_1k_time = get_1k_start.elapsed();

    let checkpoint_1k_start = Instant::now();
    db_1k.checkpoint().unwrap();
    let checkpoint_1k_time = checkpoint_1k_start.elapsed();
    drop(db_1k);

    // Replay 1k (WAL is empty here because of checkpoint)
    let replay_1k_start = Instant::now();
    let mut db_1k = Database::open(dir_1k.path()).unwrap();
    let replay_1k_time = replay_1k_start.elapsed();

    let compact_1k_start = Instant::now();
    db_1k.compact().unwrap();
    let compact_1k_time = compact_1k_start.elapsed();

    let aql_1k_start = Instant::now();
    let retrieved_1k = db_1k.retrieve_aql(query(), &view()).unwrap();
    let aql_1k_time = aql_1k_start.elapsed();
    assert!(!retrieved_1k.is_empty());

    let context_1k_start = Instant::now();
    let pack_1k = db_1k
        .context_pack_from_aql(query(), &view(), ContextPackOptions::default())
        .unwrap();
    let context_1k_time = context_1k_start.elapsed();
    assert!(!pack_1k.cells.is_empty());
    drop(db_1k);

    // --- WAL Replay (Without Checkpoint) ---
    let dir_replay_1k = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir_replay_1k.path()).unwrap();
        for id in 1..=1000 {
            db.put_cell(CellId(id), payload(id)).unwrap();
        }
    }
    let replay_no_cp_1k_start = Instant::now();
    let _db = Database::open(dir_replay_1k.path()).unwrap();
    let replay_no_cp_1k_time = replay_no_cp_1k_start.elapsed();

    let dir_replay_10k = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir_replay_10k.path()).unwrap();
        let batch = (1..=10000)
            .map(|id| (CellId(id), payload(id)))
            .collect::<Vec<_>>();
        db.put_cells(batch).unwrap();
    }
    let replay_no_cp_10k_start = Instant::now();
    let _db = Database::open(dir_replay_10k.path()).unwrap();
    let replay_no_cp_10k_time = replay_no_cp_10k_start.elapsed();

    // --- Strict vs Balanced (10K) ---
    let dir_strict = tempfile::tempdir().unwrap();
    let mut db_strict = Database::open(dir_strict.path()).unwrap();
    let put_10k_strict_start = Instant::now();
    for id in 1..=1000 {
        // 1K limit for sequential strict to keep test runtime reasonable
        db_strict.put_cell(CellId(id), payload(id)).unwrap();
    }
    let put_1k_strict_time = put_10k_strict_start.elapsed();

    let dir_balanced = tempfile::tempdir().unwrap();
    let mut db_balanced = Database::open_with_options(
        dir_balanced.path(),
        DatabaseOptions {
            durability_mode: DurabilityMode::Balanced,
            ..DatabaseOptions::default()
        },
    )
    .unwrap();
    let put_10k_balanced_start = Instant::now();
    for id in 1..=10000 {
        db_balanced.put_cell(CellId(id), payload(id)).unwrap();
    }
    let put_10k_balanced_time = put_10k_balanced_start.elapsed();

    // --- Batch Put (1K / 10K) ---
    let dir_batch_1k = tempfile::tempdir().unwrap();
    let mut db_batch_1k = Database::open(dir_batch_1k.path()).unwrap();
    let batch_1k = (1..=1000)
        .map(|id| (CellId(id), payload(id)))
        .collect::<Vec<_>>();
    let batch_1k_start = Instant::now();
    db_batch_1k.put_cells(batch_1k).unwrap();
    let batch_1k_time = batch_1k_start.elapsed();

    let dir_batch_10k = tempfile::tempdir().unwrap();
    let mut db_batch_10k = Database::open(dir_batch_10k.path()).unwrap();
    let batch_10k = (1..=10000)
        .map(|id| (CellId(id), payload(id)))
        .collect::<Vec<_>>();
    let batch_10k_start = Instant::now();
    db_batch_10k.put_cells(batch_10k).unwrap();
    let batch_10k_time = batch_10k_start.elapsed();

    // --- 10K Checkpoint & Compact ---
    let checkpoint_10k_start = Instant::now();
    db_batch_10k.checkpoint().unwrap();
    let checkpoint_10k_time = checkpoint_10k_start.elapsed();

    let compact_10k_start = Instant::now();
    db_batch_10k.compact().unwrap();
    let compact_10k_time = compact_10k_start.elapsed();

    // --- 10K Retrieve & Context Pack ---
    let aql_10k_start = Instant::now();
    let _retrieved_10k = db_batch_10k
        .retrieve_aql(query_large(), &view_large())
        .unwrap();
    let aql_10k_time = aql_10k_start.elapsed();

    let context_10k_start = Instant::now();
    let _pack_10k = db_batch_10k
        .context_pack_from_aql(query_large(), &view_large(), ContextPackOptions::default())
        .unwrap();
    let context_10k_time = context_10k_start.elapsed();

    println!("================================================");
    println!("CORTEXDB BENCHMARK MATRIX V2");
    println!("================================================");
    println!("put_1k_cells:                   {:?}", put_1k_time);
    println!("get_1k_cells:                   {:?}", get_1k_time);
    println!("checkpoint_1k:                  {:?}", checkpoint_1k_time);
    println!("restart_replay_1k:              {:?}", replay_1k_time);
    println!("compact_1k:                     {:?}", compact_1k_time);
    println!("aql_retrieve_1k:                {:?}", aql_1k_time);
    println!("context_pack_1k:                {:?}", context_1k_time);
    println!("------------------------------------------------");
    println!("restart_replay_1k_no_cp:        {:?}", replay_no_cp_1k_time);
    println!(
        "restart_replay_10k_no_cp:       {:?}",
        replay_no_cp_10k_time
    );
    println!("------------------------------------------------");
    println!("put_1k_strict_sequential:       {:?}", put_1k_strict_time);
    println!(
        "put_10k_balanced_sequential:     {:?}",
        put_10k_balanced_time
    );
    println!("------------------------------------------------");
    println!("batch_put_1k_cells:             {:?}", batch_1k_time);
    println!("batch_put_10k_cells:            {:?}", batch_10k_time);
    println!("------------------------------------------------");
    println!("checkpoint_10k:                 {:?}", checkpoint_10k_time);
    println!("compact_10k:                    {:?}", compact_10k_time);
    println!("aql_retrieve_10k:               {:?}", aql_10k_time);
    println!("context_pack_10k:               {:?}", context_10k_time);
    println!("================================================");
}

fn payload(id: u64) -> Vec<u8> {
    format!("scope=bench\nstatus=ready\nsource=bench-{id}\ncell {id} budget ready").into_bytes()
}

fn query() -> &'static str {
    r#"RETRIEVE CONTEXT FOR TASK "bench" IN BRAIN investment_projects WHERE space = bench AND status = "ready" LIMIT 100 CANDIDATES;"#
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

fn query_large() -> &'static str {
    r#"RETRIEVE CONTEXT FOR TASK "bench" IN BRAIN investment_projects WHERE space = bench AND status = "ready" LIMIT 1000 CANDIDATES;"#
}

fn view_large() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("bench".to_owned()),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id("bench")]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 1_000_000,
        default_context_budget_tokens: 500_000,
        max_candidate_limit: 10000,
        default_candidate_limit: 1000,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3_600),
        allow_remember: false,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: true,
        private_scope: None,
    }
}
