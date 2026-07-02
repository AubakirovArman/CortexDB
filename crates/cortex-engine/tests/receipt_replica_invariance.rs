use std::collections::BTreeSet;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_core::CellId;
use cortex_crypto::ReceiptSigningKey;
use cortex_engine::accountability::{
    append_transparency_log_record, read_transparency_log_records,
};
use cortex_engine::canonical::{canonical_context_pack_bytes, canonical_json_bytes};
use cortex_engine::{scope_id, ContextPackOptions, Database, DatabaseOptions, EngineFeatureFlags};

const RECEIPT_SEED: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
const DB_INSTANCE_ID: &str = "dbi_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const AUDIT_CHAIN_HEAD: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const RECEIPT_CREATED_UNIX_SECONDS: u64 = 1_700_000_000;

#[test]
fn replicated_snapshot_context_pack_and_receipt_are_byte_identical() {
    let leader_dir = tempfile::tempdir().unwrap();
    let follower_dir = tempfile::tempdir().unwrap();
    let mut leader = open_replication_db(leader_dir.path());
    let mut follower = open_replication_db(follower_dir.path());

    leader
        .put_cell(
            CellId(1),
            ready_project("source-a", "alpha budget approved for q4"),
        )
        .unwrap();
    leader
        .put_cell(
            CellId(2),
            ready_project("source-b", "alpha budget has signed approval evidence"),
        )
        .unwrap();
    leader
        .put_cell(
            CellId(3),
            b"scope=private\nstatus=ready\nsource=private\nprivate budget".to_vec(),
        )
        .unwrap();

    let snapshot = leader.replication_snapshot_segment().unwrap();
    follower.install_snapshot_segment(snapshot).unwrap();

    let leader_evidence = leader
        .context_pack_with_receipt_evidence_from_aql(query(), &agent_view(), pack_options())
        .unwrap();
    let follower_evidence = follower
        .context_pack_with_receipt_evidence_from_aql(query(), &agent_view(), pack_options())
        .unwrap();

    assert_eq!(
        canonical_context_pack_bytes(&leader_evidence.pack),
        canonical_context_pack_bytes(&follower_evidence.pack)
    );
    assert_eq!(
        leader_evidence.determinism_hash(),
        follower_evidence.determinism_hash()
    );

    let signing_key = ReceiptSigningKey::from_seed_hex("cluster-receipt-key", RECEIPT_SEED)
        .expect("fixture receipt key is valid");
    let leader_receipt = leader_evidence
        .signed_receipt_value(
            None,
            DB_INSTANCE_ID,
            RECEIPT_CREATED_UNIX_SECONDS,
            AUDIT_CHAIN_HEAD,
            &signing_key,
        )
        .unwrap();
    let follower_receipt = follower_evidence
        .signed_receipt_value(
            None,
            DB_INSTANCE_ID,
            RECEIPT_CREATED_UNIX_SECONDS,
            AUDIT_CHAIN_HEAD,
            &signing_key,
        )
        .unwrap();

    assert_eq!(
        canonical_json_bytes(&leader_receipt),
        canonical_json_bytes(&follower_receipt)
    );
    assert_eq!(
        leader_receipt["header"]["pack_root"],
        follower_receipt["header"]["pack_root"]
    );
    assert_eq!(
        leader_receipt["header"]["determinism_hash"],
        follower_receipt["header"]["determinism_hash"]
    );
    assert_eq!(
        leader_receipt["header"]["audit_chain_head"],
        AUDIT_CHAIN_HEAD
    );

    let transparency_path = follower_dir.path().join("transparency.jsonl");
    append_transparency_log_record(&transparency_path, &leader_receipt).unwrap();
    append_transparency_log_record(&transparency_path, &follower_receipt).unwrap();
    let records = read_transparency_log_records(&transparency_path).unwrap();

    assert_eq!(records.len(), 2);
    assert_eq!(records[0].pack_root, records[1].pack_root);
    assert_eq!(records[0].determinism_hash, records[1].determinism_hash);
    assert_eq!(
        records[0].receipt_signature_hex,
        records[1].receipt_signature_hex
    );
}

fn open_replication_db(path: &std::path::Path) -> Database {
    Database::open_with_options(
        path,
        DatabaseOptions {
            feature_flags: EngineFeatureFlags::production_safe()
                .with_experimental_replication(true),
            ..DatabaseOptions::default()
        },
    )
    .unwrap()
}

fn query() -> &'static str {
    r#"RETRIEVE CONTEXT FOR TASK "alpha budget approval" IN BRAIN investment_projects
WHERE status = "ready" LIMIT 10 CANDIDATES;"#
}

fn ready_project(source: &str, body: &str) -> Vec<u8> {
    format!("scope=project:investments\nstatus=ready\nsource={source}\n{body}").into_bytes()
}

fn pack_options() -> ContextPackOptions {
    ContextPackOptions {
        token_budget_tokens: 512,
        require_citations: false,
        ..ContextPackOptions::default()
    }
}

fn agent_view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("replica-invariance-test".to_owned()),
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id("project:investments")]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 1_000,
        default_context_budget_tokens: 512,
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
