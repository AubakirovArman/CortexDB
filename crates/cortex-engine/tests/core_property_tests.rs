use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::panic;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::{CellId, CommitSeq};
use cortex_engine::{scope_id, Database, DatabaseOptions, RecoveryMode, SearchLimit};
use cortex_storage::wal::{WalCodec, WalReader};

#[test]
fn deterministic_operation_sequences_match_model_across_restart() {
    for seed in property_seeds() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::open(dir.path()).unwrap();
        let mut model = BTreeMap::new();
        let mut rng = Lcg::new(seed);

        for step in 0..120 {
            apply_model_operation(&mut db, &mut model, &mut rng, step);
            if step % 17 == 0 {
                assert_model_matches_database(&db, &model, 24);
            }
            if step % 23 == 0 {
                db.checkpoint().unwrap();
                assert_model_matches_database(&db, &model, 24);
            }
            if step % 41 == 0 {
                db.compact().unwrap();
                assert_model_matches_database(&db, &model, 24);
            }
        }

        drop(db);
        let reopened = Database::open(dir.path()).unwrap();
        assert_model_matches_database(&reopened, &model, 24);
    }
}

fn property_seeds() -> Vec<u64> {
    let mut seeds = (1..=8).collect::<Vec<_>>();
    if let Ok(raw_seed) = env::var("CORTEXDB_CORE_PROPERTY_RANDOM_SEED") {
        let mut rng = Lcg::new(raw_seed.parse::<u64>().unwrap_or(1));
        seeds.extend((0..8).map(|_| rng.next_u64().max(1)));
    }
    seeds
}

#[test]
fn wal_complete_prefixes_replay_exact_committed_model_in_strict_mode() {
    let source = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(source.path()).unwrap();
        for id in 1..=6 {
            db.put_cell(CellId(id), model_payload(id as usize, id as usize))
                .unwrap();
        }
    }

    let wal_path = source.path().join("db.aclog");
    let bytes = fs::read(&wal_path).unwrap();
    let scan = WalReader::scan_path(&wal_path).unwrap();
    let mut offsets = vec![WalCodec::file_header_len()];
    let mut offset = WalCodec::file_header_len();
    for record in &scan.records {
        offset += record.bytes_consumed;
        offsets.push(offset);
    }

    for (committed, offset) in offsets.into_iter().enumerate() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("db.aclog"), &bytes[..offset]).unwrap();
        let db = Database::open_with_options(
            dir.path(),
            DatabaseOptions {
                recovery_mode: RecoveryMode::Strict,
                ..DatabaseOptions::default()
            },
        )
        .unwrap();

        assert_eq!(db.current_seq(), CommitSeq(committed as u64));
        for id in 1..=6 {
            let expected = if id <= committed as u64 {
                Some(model_payload(id as usize, id as usize))
            } else {
                None
            };
            assert_eq!(db.get_latest_cell(CellId(id)), expected);
        }
    }
}

#[test]
fn wal_corruption_variants_do_not_panic_and_best_effort_opens_safe_prefix() {
    let source = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(source.path()).unwrap();
        for id in 1..=5 {
            db.put_cell(CellId(id), model_payload(id as usize, id as usize))
                .unwrap();
        }
    }

    let original = fs::read(source.path().join("db.aclog")).unwrap();
    let positions = corruption_positions(original.len());
    for position in positions {
        let mut corrupted = original.clone();
        corrupted[position] ^= 0xff;

        let strict_dir = tempfile::tempdir().unwrap();
        fs::write(strict_dir.path().join("db.aclog"), &corrupted).unwrap();
        let strict_result = panic::catch_unwind(|| {
            Database::open_with_options(
                strict_dir.path(),
                DatabaseOptions {
                    recovery_mode: RecoveryMode::Strict,
                    ..DatabaseOptions::default()
                },
            )
        });
        assert!(strict_result.is_ok());

        let best_effort_dir = tempfile::tempdir().unwrap();
        fs::write(best_effort_dir.path().join("db.aclog"), corrupted).unwrap();
        let best_effort_result = panic::catch_unwind(|| {
            Database::open_with_options(
                best_effort_dir.path(),
                DatabaseOptions {
                    recovery_mode: RecoveryMode::BestEffort,
                    ..DatabaseOptions::default()
                },
            )
        });
        assert!(best_effort_result.is_ok());
        if let Ok(Ok(db)) = best_effort_result {
            assert!(db.current_seq().0 <= 5);
        }
    }
}

#[test]
fn persisted_keyword_index_matches_fresh_rebuild_after_mutation_sequence() {
    let model = seeded_search_model();
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        apply_search_model_mutations(&mut db);
        db.checkpoint().unwrap();
    }
    let persisted = Database::open(dir.path()).unwrap();
    let persisted_ids = keyword_ids(&persisted, "alpha");

    let rebuild_dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(rebuild_dir.path()).unwrap();
        for (cell_id, payload) in &model {
            db.put_cell(*cell_id, payload.clone()).unwrap();
        }
        db.checkpoint().unwrap();
    }
    let rebuilt = Database::open(rebuild_dir.path()).unwrap();
    assert_eq!(persisted_ids, keyword_ids(&rebuilt, "alpha"));
}

fn apply_model_operation(
    db: &mut Database,
    model: &mut BTreeMap<CellId, Vec<u8>>,
    rng: &mut Lcg,
    step: usize,
) {
    let cell_id = CellId(1 + rng.next_range(24));
    match rng.next_range(3) {
        0 => {
            let payload = model_payload(cell_id.0 as usize, step);
            db.put_cell(cell_id, payload.clone()).unwrap();
            model.insert(cell_id, payload);
        }
        1 => {
            let payload = model_payload(cell_id.0 as usize, step + 10_000);
            let result = db.patch_cell(cell_id, payload.clone());
            if let std::collections::btree_map::Entry::Occupied(mut entry) = model.entry(cell_id) {
                result.unwrap();
                entry.insert(payload);
            } else {
                assert!(result.is_err());
            }
        }
        _ => {
            let result = db.tombstone_cell(cell_id);
            if model.remove(&cell_id).is_some() {
                result.unwrap();
            } else {
                assert!(result.is_err());
            }
        }
    }
}

fn assert_model_matches_database(db: &Database, model: &BTreeMap<CellId, Vec<u8>>, max_id: u64) {
    for id in 1..=max_id {
        let cell_id = CellId(id);
        assert_eq!(db.get_latest_cell(cell_id), model.get(&cell_id).cloned());
    }
}

fn model_payload(cell: usize, version: usize) -> Vec<u8> {
    format!(
        "scope=model\nstatus=ready\ntype=fact\nsource=property-{cell}\n\nalpha property cell {cell} version {version}"
    )
    .into_bytes()
}

fn corruption_positions(len: usize) -> Vec<usize> {
    let header = WalCodec::file_header_len();
    let mut positions = BTreeSet::new();
    for divisor in [2, 3, 5, 7, 11, 13] {
        let position = header + (len.saturating_sub(header + 1) / divisor);
        if position < len {
            positions.insert(position);
        }
    }
    positions.into_iter().collect()
}

fn seeded_search_model() -> BTreeMap<CellId, Vec<u8>> {
    BTreeMap::from([
        (
            CellId(1),
            b"scope=model\nstatus=ready\n\nalpha retained first".to_vec(),
        ),
        (
            CellId(3),
            b"scope=model\nstatus=ready\n\nalpha retained third".to_vec(),
        ),
        (
            CellId(4),
            b"scope=model\nstatus=ready\n\nbeta retained fourth".to_vec(),
        ),
    ])
}

fn apply_search_model_mutations(db: &mut Database) {
    db.put_cell(
        CellId(1),
        b"scope=model\nstatus=ready\n\nalpha stale first".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=model\nstatus=ready\n\nalpha deleted second".to_vec(),
    )
    .unwrap();
    db.checkpoint().unwrap();
    db.patch_cell(
        CellId(1),
        b"scope=model\nstatus=ready\n\nalpha retained first".to_vec(),
    )
    .unwrap();
    db.tombstone_cell(CellId(2)).unwrap();
    db.put_cell(
        CellId(3),
        b"scope=model\nstatus=ready\n\nalpha retained third".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(4),
        b"scope=model\nstatus=ready\n\nbeta retained fourth".to_vec(),
    )
    .unwrap();
}

fn keyword_ids(db: &Database, query: &str) -> Vec<CellId> {
    db.search_keyword(query, &model_view(), SearchLimit(20))
        .unwrap()
        .into_iter()
        .map(|result| result.cell_id)
        .collect()
}

fn model_view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("property-test".to_owned()),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id("model")]),
        writable_scopes: BTreeSet::from([scope_id("model")]),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([
            MemoryType::Decision,
            MemoryType::Preference,
            MemoryType::WorkflowResult,
            MemoryType::ErrorLog,
            MemoryType::Observation,
        ]),
        max_context_budget_tokens: 4_000,
        default_context_budget_tokens: 1_000,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: None,
        allow_remember: true,
        allow_verify_fact: true,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}

struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_range(&mut self, upper: u64) -> u64 {
        self.next_u64() % upper
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        self.state >> 32
    }
}
