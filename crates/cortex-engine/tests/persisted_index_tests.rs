use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, ScopeId, Q16_ZERO};
use cortex_core::CellId;
use cortex_engine::{scope_id, Database, EngineError};
use cortex_storage::indexes::{BitmapIndex, LexicalIndex};
use cortex_storage::manifest::{ManifestSegment, StorageManifest};
use cortex_storage::segment::{SegmentCell, SegmentWriter};

#[test]
fn two_checkpoint_segments_same_status_are_union_merged() {
    let dir = two_ready_checkpoints();
    let db = Database::open(dir.path()).unwrap();
    let cells = retrieve_ready(&db);
    assert_eq!(cell_ids(cells), vec![CellId(1), CellId(2)]);
}

#[test]
fn two_checkpoint_segments_same_scope_are_union_merged() {
    let dir = two_ready_checkpoints();
    let db = Database::open(dir.path()).unwrap();
    let cells = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "sharedterm" IN BRAIN investment_projects
WHERE space = project:investments LIMIT 10 CANDIDATES;"#,
            &view(scope_id("project:investments")),
        )
        .unwrap();
    assert_eq!(cell_ids(cells), vec![CellId(1), CellId(2)]);
}

#[test]
fn two_checkpoint_segments_same_term_are_union_merged() {
    let dir = two_ready_checkpoints();
    let db = Database::open(dir.path()).unwrap();
    let (_, lexical) = db.persisted_indexes().unwrap();
    assert_eq!(
        lexical.terms.get("sharedterm").cloned().unwrap_or_default(),
        BTreeSet::from([1, 2])
    );
}

#[test]
fn retrieve_aql_after_two_checkpoints_returns_both_cells() {
    let dir = two_ready_checkpoints();
    let db = Database::open(dir.path()).unwrap();
    assert_eq!(cell_ids(retrieve_ready(&db)), vec![CellId(1), CellId(2)]);
}

#[test]
fn candidate_reverse_map_matches_forward_map() {
    let dir = two_ready_checkpoints();
    let db = Database::open(dir.path()).unwrap();
    let index = db.try_aql_index().unwrap();
    for (candidate, cell_id) in &index.candidate_to_cell {
        assert_eq!(index.cell_to_candidate.get(cell_id), Some(candidate));
    }
}

#[test]
fn checkpoint_returns_error_on_candidate_overflow() {
    let dir = tempfile::tempdir().unwrap();
    write_max_candidate_segment(dir.path());
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(
        CellId(2),
        b"scope=project:investments\nstatus=ready\nnew".to_vec(),
    )
    .unwrap();
    assert!(matches!(
        db.checkpoint().unwrap_err(),
        EngineError::CandidateIdOverflow
    ));
}

#[test]
fn second_open_same_path_fails() {
    let dir = tempfile::tempdir().unwrap();
    let db = Database::open(dir.path()).unwrap();
    assert!(matches!(
        Database::open(dir.path()).unwrap_err(),
        EngineError::DatabaseAlreadyOpen(_)
    ));
    drop(db);
    assert!(Database::open(dir.path()).is_ok());
}

#[test]
fn agent_with_scope_a_does_not_retrieve_scope_b() {
    let dir = tempfile::tempdir().unwrap();
    let mut db = Database::open(dir.path()).unwrap();
    db.put_cell(CellId(1), b"scope=scope:a\nstatus=ready\nalpha".to_vec())
        .unwrap();
    db.put_cell(CellId(2), b"scope=scope:b\nstatus=ready\nbeta".to_vec())
        .unwrap();

    let cells = db
        .retrieve_aql(
            r#"RETRIEVE CONTEXT FOR TASK "ready" IN BRAIN investment_projects
WHERE status = "ready" LIMIT 10 CANDIDATES;"#,
            &view(scope_id("scope:a")),
        )
        .unwrap();
    assert_eq!(cell_ids(cells), vec![CellId(1)]);
}

fn two_ready_checkpoints() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(
            CellId(1),
            b"scope=project:investments\nstatus=ready\nsharedterm one".to_vec(),
        )
        .unwrap();
        db.checkpoint().unwrap();
        db.put_cell(
            CellId(2),
            b"scope=project:investments\nstatus=ready\nsharedterm two".to_vec(),
        )
        .unwrap();
        db.checkpoint().unwrap();
    }
    dir
}

fn retrieve_ready(db: &Database) -> Vec<cortex_engine::RetrievedCell> {
    db.retrieve_aql(
        r#"RETRIEVE CONTEXT FOR TASK "sharedterm" IN BRAIN investment_projects
WHERE status = "ready" LIMIT 10 CANDIDATES;"#,
        &view(scope_id("project:investments")),
    )
    .unwrap()
}

fn cell_ids(cells: Vec<cortex_engine::RetrievedCell>) -> Vec<CellId> {
    cells.into_iter().map(|cell| cell.cell_id).collect()
}

fn write_max_candidate_segment(root: &std::path::Path) {
    let segments = root.join("segments");
    std::fs::create_dir_all(&segments).unwrap();
    SegmentWriter::write(
        segments.join("segment-1.acs"),
        &[SegmentCell {
            candidate_id: u32::MAX,
            cell_id: 1,
            created_seq: 1,
            deleted_seq: None,
            payload: b"scope=project:investments\nstatus=ready\nold".to_vec(),
        }],
    )
    .unwrap();
    BitmapIndex::default()
        .write(segments.join("segment-1.acb"))
        .unwrap();
    LexicalIndex::default()
        .write(segments.join("segment-1.aci"))
        .unwrap();
    StorageManifest {
        generation: 1,
        checkpoint_seq: 1,
        live_segments: vec![ManifestSegment {
            id: 1,
            generation: 1,
            checkpoint_seq: 1,
            cell_count: 1,
        }],
        retired_segments: Vec::new(),
        hnsw_profile: None,
    }
    .store(root.join("manifest.acm"))
    .unwrap();
}

fn view(scope: ScopeId) -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: None,
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 1_000,
        default_context_budget_tokens: 400,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3_600),
        allow_remember: false,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}
