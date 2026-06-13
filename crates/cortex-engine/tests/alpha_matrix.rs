use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::{CellId, CommitSeq};
use cortex_engine::query::scope_id;
use cortex_engine::verification::VerificationStatus;
use cortex_engine::{
    ContextPackOptions, Database, DatabaseOptions, PayloadResidency, RecoveryMode,
};

fn test_view(scope: &str, allow_verify: bool) -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("test-agent".to_owned()),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id(scope)]),
        writable_scopes: BTreeSet::from([scope_id(scope)]),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 4_000,
        default_context_budget_tokens: 1_000,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3600),
        allow_remember: true,
        allow_verify_fact: allow_verify,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}

#[test]
fn alpha_matrix_put_patch_tombstone_restart() {
    let dir = tempfile::tempdir().unwrap();
    // 1. Put
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"hello".to_vec()).unwrap();
    }
    {
        let db = Database::open(dir.path()).unwrap();
        assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"hello");
    }

    // 2. Patch
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.patch_cell(CellId(1), b"hello patch".to_vec()).unwrap();
    }
    {
        let db = Database::open(dir.path()).unwrap();
        assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"hello patch");
    }

    // 3. Tombstone
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.tombstone_cell(CellId(1)).unwrap();
    }
    {
        let db = Database::open(dir.path()).unwrap();
        assert!(db.get_latest_cell(CellId(1)).is_none());
    }
}

#[test]
fn alpha_matrix_checkpoint_compact_restart() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"hello".to_vec()).unwrap();
        db.checkpoint().unwrap();
    }
    {
        let db = Database::open(dir.path()).unwrap();
        assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"hello");
    }

    {
        let mut db = Database::open(dir.path()).unwrap();
        db.compact().unwrap();
    }
    {
        let db = Database::open(dir.path()).unwrap();
        assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"hello");
    }
}

#[test]
fn alpha_matrix_checkpoint_compact_with_wal_tail_restart() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"v1".to_vec()).unwrap();
        db.checkpoint().unwrap();
        db.put_cell(CellId(2), b"v2".to_vec()).unwrap();
    }
    {
        let db = Database::open(dir.path()).unwrap();
        assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"v1");
        assert_eq!(db.get_latest_cell(CellId(2)).unwrap(), b"v2");
    }

    {
        let mut db = Database::open(dir.path()).unwrap();
        db.compact().unwrap();
        db.put_cell(CellId(3), b"v3".to_vec()).unwrap();
    }
    {
        let db = Database::open(dir.path()).unwrap();
        assert_eq!(db.get_latest_cell(CellId(1)).unwrap(), b"v1");
        assert_eq!(db.get_latest_cell(CellId(2)).unwrap(), b"v2");
        assert_eq!(db.get_latest_cell(CellId(3)).unwrap(), b"v3");
    }
}

#[test]
fn alpha_matrix_lazy_payload_checkpoint_compact_and_wal_tail_restart() {
    let dir = tempfile::tempdir().unwrap();
    let lazy_options = || DatabaseOptions {
        payload_residency: PayloadResidency::Lazy,
        payload_cache_bytes: 16,
        ..DatabaseOptions::default()
    };

    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"scope=default\nstatus=ready\n\nv1".to_vec())
            .unwrap();
        db.checkpoint().unwrap();
        db.put_cell(CellId(2), b"scope=default\nstatus=ready\n\nv2".to_vec())
            .unwrap();
    }
    {
        let db = Database::open_with_options(dir.path(), lazy_options()).unwrap();
        assert_eq!(
            db.get_latest_cell(CellId(1)).unwrap(),
            b"scope=default\nstatus=ready\n\nv1"
        );
        assert_eq!(
            db.get_latest_cell(CellId(2)).unwrap(),
            b"scope=default\nstatus=ready\n\nv2"
        );
        assert!(db.storage_stats().unwrap().memtable_payload_bytes > 0);
    }

    {
        let mut db = Database::open_with_options(dir.path(), lazy_options()).unwrap();
        db.patch_cell(
            CellId(1),
            b"scope=default\nstatus=ready\n\nv1-patched".to_vec(),
        )
        .unwrap();
        db.tombstone_cell(CellId(2)).unwrap();
        db.checkpoint().unwrap();
    }
    {
        let db = Database::open_with_options(dir.path(), lazy_options()).unwrap();
        assert_eq!(
            db.get_latest_cell(CellId(1)).unwrap(),
            b"scope=default\nstatus=ready\n\nv1-patched"
        );
        assert!(db.get_latest_cell(CellId(2)).is_none());
        assert_eq!(db.storage_stats().unwrap().memtable_payload_bytes, 0);
    }

    {
        let mut db = Database::open_with_options(dir.path(), lazy_options()).unwrap();
        db.compact().unwrap();
        db.put_cell(CellId(3), b"scope=default\nstatus=ready\n\nv3".to_vec())
            .unwrap();
    }
    {
        let db = Database::open_with_options(dir.path(), lazy_options()).unwrap();
        assert_eq!(
            db.get_latest_cell(CellId(1)).unwrap(),
            b"scope=default\nstatus=ready\n\nv1-patched"
        );
        assert!(db.get_latest_cell(CellId(2)).is_none());
        assert_eq!(
            db.get_latest_cell(CellId(3)).unwrap(),
            b"scope=default\nstatus=ready\n\nv3"
        );
    }
}

#[test]
fn alpha_matrix_aql_and_context_pack_before_after_checkpoint() {
    let dir = tempfile::tempdir().unwrap();
    let query = "RETRIEVE CONTEXT FOR TASK \"budget\" IN BRAIN investment_projects WHERE space = project:investments AND status = \"ready\" LIMIT 10 CANDIDATES;";

    // Before checkpoint (in WAL)
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(
            CellId(1),
            b"scope=project:investments\nstatus=ready\nsource=doc-a\nalpha budget".to_vec(),
        )
        .unwrap();

        let cells = db
            .retrieve_aql(query, &test_view("project:investments", false))
            .unwrap();
        assert_eq!(cells.len(), 1);

        let pack = db
            .context_pack_from_aql(
                query,
                &test_view("project:investments", false),
                ContextPackOptions::default(),
            )
            .unwrap();
        assert_eq!(pack.cells.len(), 1);
    }

    // After checkpoint (persisted segment)
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.checkpoint().unwrap();

        let cells = db
            .retrieve_aql(query, &test_view("project:investments", false))
            .unwrap();
        assert_eq!(cells.len(), 1);

        let pack = db
            .context_pack_from_aql(
                query,
                &test_view("project:investments", false),
                ContextPackOptions::default(),
            )
            .unwrap();
        assert_eq!(pack.cells.len(), 1);
    }
}

#[test]
fn alpha_matrix_verify_mixed_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(1),
        b"scope=project:investments\nstatus=ready\nsource=doc-a\nproject=Solar Plant\nmetric=budget\nvalue=1.2\ncurrency=KZT\n\nSolar Plant report highlights. The total approved budget for the Solar Plant project in first quarter is 1.2B KZT.".to_vec(),
    )
    .unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\nsource=doc-b\nproject=Solar Plant\nmetric=budget\nvalue=1400000000\ncurrency=KZT\n\nSolar Plant Q2 update. Following recent expansions, the budget for Solar Plant has been adjusted to 1.4B KZT.".to_vec(),
    )
    .unwrap();

    let report = db
        .verify_fact_aql(
            r#"VERIFY FACT "Solar Plant budget is 1.2B KZT" IN BRAIN investment_projects;"#,
            &test_view("project:investments", true),
        )
        .unwrap();
    assert_eq!(report.status, VerificationStatus::Mixed);
    assert_eq!(report.evidence[0].cell_id, CellId(1));
    assert_eq!(report.contradicting_evidence[0].cell_id, CellId(2));
}

#[test]
fn alpha_matrix_best_effort_recovery_stops_at_corrupt_payload() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"hello".to_vec()).unwrap();
        db.put_cell(CellId(2), b"world".to_vec()).unwrap();
    }

    // Corrupt last bytes of active WAL file
    let wal_path = dir.path().join("db.aclog");
    let mut bytes = std::fs::read(&wal_path).unwrap();
    let len = bytes.len();
    bytes[len - 5..].copy_from_slice(&[0, 0, 0, 0, 0]);
    std::fs::write(&wal_path, bytes).unwrap();

    let db = Database::open_with_options(
        dir.path(),
        DatabaseOptions {
            recovery_mode: RecoveryMode::BestEffort,
            ..DatabaseOptions::default()
        },
    )
    .unwrap();
    // Replayed first record successfully, stopped before corrupt second record/portion
    assert_eq!(db.current_seq(), CommitSeq(1));
}

#[test]
fn alpha_matrix_strict_corrupt_wal_fails() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(CellId(1), b"hello".to_vec()).unwrap();
    }

    let wal_path = dir.path().join("db.aclog");
    let mut bytes = std::fs::read(&wal_path).unwrap();
    let len = bytes.len();
    bytes[len - 5..].copy_from_slice(&[0, 0, 0, 0, 0]);
    std::fs::write(&wal_path, bytes).unwrap();

    let db_err = Database::open_with_options(
        dir.path(),
        DatabaseOptions {
            recovery_mode: RecoveryMode::Strict,
            ..DatabaseOptions::default()
        },
    );
    assert!(db_err.is_err());
}
