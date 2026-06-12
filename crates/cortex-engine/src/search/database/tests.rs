use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::{CellId, KnowledgeCellMetadata, KnowledgeCellType};

use crate::database::Database;
use crate::operation::DbOperation;
use crate::query::scope_id;
use crate::search::SearchLimit;

#[test]
fn search_result_metadata_prefers_descriptor_for_snapshot_and_persisted_paths() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        put_spoofed_payload_with_descriptor(&mut db);
        assert_descriptor_metadata(&db);
        db.checkpoint().unwrap();
    }

    let db = Database::open(dir.path()).unwrap();
    assert_descriptor_metadata(&db);
}

fn put_spoofed_payload_with_descriptor(db: &mut Database) {
    let descriptor_metadata = KnowledgeCellMetadata {
        scope: "project:investments".to_owned(),
        status: "ready".to_owned(),
        cell_type: KnowledgeCellType::Fact,
        created_unix_seconds: Some(1_717_171_717),
        source_trust_q16: Some(54_321),
        source: Some("descriptor-source".to_owned()),
        ..KnowledgeCellMetadata::default()
    };
    let spoofed_payload = concat!(
        "scope=tenant:spoofed\n",
        "status=draft\n",
        "type=memory\n",
        "created_unix_seconds=1\n",
        "source_trust_q16=1\n",
        "source=payload-source\n",
        "\n",
        "budget descriptor evidence"
    )
    .as_bytes()
    .to_vec();

    db.append_then_apply_with_metadata(
        DbOperation::PutCell {
            cell_id: CellId(44),
            payload: spoofed_payload,
        },
        descriptor_metadata.encode_wal_section(),
    )
    .unwrap();
}

fn assert_descriptor_metadata(db: &Database) {
    let results = db
        .search_keyword("budget", &view("project:investments"), SearchLimit(10))
        .unwrap();

    assert_eq!(results.len(), 1);
    let result = &results[0];
    assert_eq!(result.cell_id, CellId(44));
    assert_eq!(result.metadata.scope, "project:investments");
    assert_eq!(result.metadata.status, "ready");
    assert_eq!(result.metadata.cell_type, "fact");
    assert_eq!(result.metadata.created_unix_seconds, Some(1_717_171_717));
    assert_eq!(result.metadata.source_trust_q16, Some(54_321));
    assert_eq!(result.metadata.source.as_deref(), Some("descriptor-source"));
}

fn view(scope: &str) -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: None,
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id(scope)]),
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
