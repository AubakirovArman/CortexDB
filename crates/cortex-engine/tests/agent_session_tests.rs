use std::fs::File;
use std::io::Write;

use cortex_aql::{AgentId, AgentView, BindError, MemoryType, PolicyError, ScopeId};
use cortex_core::{CellDescriptor, CellId, CommitSeq, KnowledgeCellType};
use cortex_engine::{
    encode_cell_core, scope_id, Database, DatabaseOptions, EngineError, PayloadResidency,
};
use cortex_storage::wal::{SectionTag, WalCodec, WalRecord, WalRecordType, WalSection};
use tempfile::tempdir;

fn view(scope: &str) -> AgentView {
    AgentView {
        agent_id: AgentId(7),
        label: None,
        readable_brains: Default::default(),
        readable_scopes: [scope_id(scope)].into_iter().collect(),
        writable_scopes: [scope_id(scope)].into_iter().collect(),
        allowed_modes: Default::default(),
        allowed_memory_types: [
            MemoryType::WorkflowResult,
            MemoryType::Observation,
            MemoryType::Decision,
        ]
        .into_iter()
        .collect(),
        max_context_budget_tokens: 10_000,
        default_context_budget_tokens: 1_000,
        max_candidate_limit: 100,
        default_candidate_limit: 10,
        min_required_confidence_q16: Default::default(),
        max_ttl_seconds: Some(3_600),
        allow_remember: true,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}

#[test]
fn agent_session_records_context_and_temporary_memory() {
    let dir = tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let view = view("agent:finance");

    let session = db
        .start_agent_session(&view, "agent:finance", b"review capex first", 60, 1_000)
        .unwrap();
    let memory = db
        .remember_session_memory(&session, &view, b"temporary note", None, 1_010)
        .unwrap();

    assert_eq!(memory.session_id, session.session_id);
    assert_eq!(memory.ttl_seconds, 50);

    let cells = db.retrieve_session_cells(&session.session_id, &view, 1_020);
    assert_eq!(cells.len(), 2);
    assert!(cells[0]
        .payload
        .windows("session_kind=context".len())
        .any(|w| w == b"session_kind=context"));
    assert!(cells[1]
        .payload
        .windows("temporary note".len())
        .any(|w| w == b"temporary note"));
}

#[test]
fn session_cell_ids_preserve_max_documented_agent_slot() {
    let dir = tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let low = view("agent:finance");
    let mut high = view("agent:finance");
    high.agent_id = AgentId(0x0fff_ffff);

    let low_session = db
        .start_agent_session(&low, "agent:finance", b"low", 60, 1_000)
        .unwrap();
    let high_session = db
        .start_agent_session(&high, "agent:finance", b"high", 60, 1_001)
        .unwrap();

    assert_eq!(
        encoded_agent_slot(low_session.context_cell_id),
        low.agent_id.0
    );
    assert_eq!(
        encoded_agent_slot(high_session.context_cell_id),
        high.agent_id.0
    );
}

#[test]
fn session_cell_ids_reject_agent_slot_overflow() {
    let dir = tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let mut view = view("agent:finance");
    view.agent_id = AgentId(0x1000_0007);

    let error = db
        .start_agent_session(&view, "agent:finance", b"overflow", 60, 1_000)
        .unwrap_err();
    assert!(matches!(error, EngineError::StorageInvariant(_)));
}

#[test]
fn session_retrieval_filters_by_session_and_ttl() {
    let dir = tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let view = view("agent:finance");

    let first = db
        .start_agent_session(&view, "agent:finance", b"first", 30, 1_000)
        .unwrap();
    let second = db
        .start_agent_session(&view, "agent:finance", b"second", 30, 1_001)
        .unwrap();
    db.remember_session_memory(&first, &view, b"first note", Some(20), 1_005)
        .unwrap();
    db.remember_session_memory(&second, &view, b"second note", Some(20), 1_006)
        .unwrap();

    let first_cells = db.retrieve_session_cells(&first.session_id, &view, 1_010);
    assert_eq!(first_cells.len(), 2);
    assert!(first_cells.iter().all(|cell| {
        !cell
            .payload
            .windows("second note".len())
            .any(|w| w == b"second note")
    }));

    assert!(db
        .retrieve_session_cells(&first.session_id, &view, 1_031)
        .is_empty());
}

#[test]
fn session_memory_survives_restart() {
    let dir = tempdir().unwrap();
    let session_id = {
        let mut db = Database::open(dir.path()).unwrap();
        let view = view("agent:finance");
        let session = db
            .start_agent_session(&view, "agent:finance", b"restart context", 60, 1_000)
            .unwrap();
        db.remember_session_memory(&session, &view, b"restart note", Some(30), 1_010)
            .unwrap();
        session.session_id
    };

    let db = Database::open(dir.path()).unwrap();
    let cells = db.retrieve_session_cells(&session_id, &view("agent:finance"), 1_020);
    assert_eq!(cells.len(), 2);
    assert!(cells.iter().any(|cell| {
        cell.payload
            .windows("restart note".len())
            .any(|w| w == b"restart note")
    }));
}

#[test]
fn session_memory_survives_lazy_checkpoint_reopen() {
    let dir = tempdir().unwrap();
    let session_id = {
        let mut db = Database::open(dir.path()).unwrap();
        let view = view("agent:finance");
        let session = db
            .start_agent_session(&view, "agent:finance", b"lazy context", 60, 1_000)
            .unwrap();
        db.remember_session_memory(&session, &view, b"lazy restart note", Some(30), 1_010)
            .unwrap();
        db.checkpoint().unwrap();
        session.session_id
    };

    let db = Database::open_with_options(
        dir.path(),
        DatabaseOptions {
            payload_residency: PayloadResidency::Lazy,
            ..DatabaseOptions::default()
        },
    )
    .unwrap();
    assert_eq!(db.storage_stats().unwrap().memtable_payload_bytes, 0);

    let cells = db.retrieve_session_cells(&session_id, &view("agent:finance"), 1_020);
    assert_eq!(cells.len(), 2);
    assert!(cells.iter().any(|cell| {
        cell.payload
            .windows("lazy restart note".len())
            .any(|window| window == b"lazy restart note")
    }));
}

#[test]
fn session_index_tracks_patch_tombstone_checkpoint_and_reopen() {
    let dir = tempdir().unwrap();
    let (session_id, memory_id) = {
        let mut db = Database::open(dir.path()).unwrap();
        let view = view("agent:finance");
        let session = db
            .start_agent_session(&view, "agent:finance", b"context", 60, 1_000)
            .unwrap();
        let memory = db
            .remember_session_memory(&session, &view, b"stale note", Some(40), 1_010)
            .unwrap();
        let patched_payload = format!(
            "scope=agent:finance\nstatus=ready\ntype=memory\nmemory_type=observation\nttl_seconds=40\ncreated_unix_seconds=1010\nsource=agent:7\nsession_id={}\nsession_kind=temporary_memory\n\npatched note",
            session.session_id
        )
        .into_bytes();
        db.patch_cell(memory.cell_id, patched_payload).unwrap();
        db.tombstone_cell(session.context_cell_id).unwrap();

        let live_cells = db.retrieve_session_cells(&session.session_id, &view, 1_020);
        assert_eq!(live_cells.len(), 1);
        assert_eq!(live_cells[0].cell_id, memory.cell_id);
        assert!(live_cells[0]
            .payload
            .windows("patched note".len())
            .any(|window| window == b"patched note"));

        db.checkpoint().unwrap();
        (session.session_id, memory.cell_id)
    };

    let db = Database::open(dir.path()).unwrap();
    let live_cells = db.retrieve_session_cells(&session_id, &view("agent:finance"), 1_020);
    assert_eq!(live_cells.len(), 1);
    assert_eq!(live_cells[0].cell_id, memory_id);
    assert!(live_cells[0]
        .payload
        .windows("patched note".len())
        .any(|window| window == b"patched note"));
}

#[test]
fn session_policy_denies_unwritable_scope() {
    let dir = tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let view = view("agent:finance");

    let error = db
        .start_agent_session(&view, "agent:private", b"blocked", 60, 1_000)
        .unwrap_err();
    assert!(matches!(
        error,
        EngineError::AqlBind(BindError::PolicyDenied(PolicyError::ScopeNotWritable))
    ));
}

#[test]
fn session_memory_cannot_outlive_session() {
    let dir = tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let view = view("agent:finance");
    let session = db
        .start_agent_session(&view, "agent:finance", b"context", 60, 1_000)
        .unwrap();

    let error = db
        .remember_session_memory(&session, &view, b"too long", Some(61), 1_000)
        .unwrap_err();
    assert!(matches!(
        error,
        EngineError::AqlBind(BindError::PolicyDenied(PolicyError::TtlTooLong))
    ));
}

#[test]
fn session_memory_after_expiry_is_rejected() {
    let dir = tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let view = view("agent:finance");
    let session = db
        .start_agent_session(&view, "agent:finance", b"context", 60, 1_000)
        .unwrap();

    let error = db
        .remember_session_memory(&session, &view, b"late", None, 1_060)
        .unwrap_err();
    assert!(matches!(error, EngineError::AgentSessionExpired(_)));
}

#[test]
fn unreadable_scope_cannot_retrieve_session_cells() {
    let dir = tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    let readable = view("agent:finance");
    let session = db
        .start_agent_session(&readable, "agent:finance", b"context", 60, 1_000)
        .unwrap();
    let mut blocked = view("agent:other");
    blocked.writable_scopes.insert(ScopeId(0));

    assert!(db
        .retrieve_session_cells(&session.session_id, &blocked, 1_010)
        .is_empty());
}

#[test]
fn session_retrieval_authorizes_with_descriptor_scope_over_payload_scope() {
    let dir = tempdir().unwrap();
    let wal_path = dir.path().join("db.aclog");
    let descriptor = CellDescriptor {
        scope: "agent:finance".to_owned(),
        status: "ready".to_owned(),
        cell_type: KnowledgeCellType::Memory,
        memory_type: Some("observation".to_owned()),
        ttl_seconds: Some(60),
        created_unix_seconds: Some(1_000),
        ..CellDescriptor::default()
    };
    let payload = b"scope=agent:private\nstatus=ready\ntype=memory\nmemory_type=observation\nttl_seconds=60\ncreated_unix_seconds=1000\nsession_id=session-typed\nsession_kind=context\n\nsession body"
        .to_vec();
    let record = WalRecord::new(
        WalRecordType::PutCellBatch,
        vec![
            WalSection::new(
                SectionTag::CellCore,
                encode_cell_core(CellId(9_001), CommitSeq(1)),
            ),
            WalSection::new(SectionTag::PayloadInline, payload),
            WalSection::new(SectionTag::CellDescriptor, descriptor.encode_section_v1()),
        ],
    );
    let mut file = File::create(&wal_path).unwrap();
    file.write_all(&WalCodec::file_header()).unwrap();
    let encoded = WalCodec::encode_record_at(&record, WalCodec::file_header_len() as u64).unwrap();
    file.write_all(&encoded).unwrap();

    let db = Database::open(dir.path()).unwrap();

    let descriptor_scope_cells =
        db.retrieve_session_cells("session-typed", &view("agent:finance"), 1_010);
    assert_eq!(descriptor_scope_cells.len(), 1);
    assert!(db
        .retrieve_session_cells("session-typed", &view("agent:private"), 1_010)
        .is_empty());
}

fn encoded_agent_slot(cell_id: CellId) -> u64 {
    (cell_id.0 >> 32) & 0x0fff_ffff
}
