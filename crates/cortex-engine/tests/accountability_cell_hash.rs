use cortex_core::{CellDescriptor, CellId, KnowledgeCellType};
use cortex_engine::accountability::{
    canonical_cell_bytes, cell_content_hash, retrieved_cell_content_hash,
    ACCOUNTABILITY_CELL_BYTES_SCHEMA, ACCOUNTABILITY_CELL_HASH_DOMAIN,
};
use cortex_engine::RetrievedCell;

#[test]
fn accountability_cell_content_hash_is_deterministic() {
    let descriptor = sample_descriptor("descriptor-asserted-hash");
    let payload = sample_payload("payload-asserted-hash");

    let first = cell_content_hash(CellId(42), &payload, &descriptor);
    let second = cell_content_hash(CellId(42), &payload, &descriptor);
    let canonical = canonical_cell_bytes(CellId(42), &payload, &descriptor);
    let canonical_text = std::str::from_utf8(&canonical).expect("canonical cell bytes are utf-8");

    assert_eq!(first, second);
    assert_eq!(first.len(), 64);
    assert_ne!(first, "descriptor-asserted-hash");
    assert!(canonical_text.contains(ACCOUNTABILITY_CELL_BYTES_SCHEMA));
    assert!(canonical_text.contains(r#""payload_hex":"#));
    assert!(canonical_text.contains(r#""descriptor":"#));
    assert!(canonical_text.contains(r#""content_hash":"descriptor-asserted-hash""#));
}

#[test]
fn accountability_cell_content_hash_changes_on_one_payload_byte() {
    let descriptor = sample_descriptor("descriptor-asserted-hash");
    let mut payload = sample_payload("payload-asserted-hash");
    let original = cell_content_hash(CellId(42), &payload, &descriptor);

    let last = payload.last_mut().expect("sample payload is nonempty");
    *last ^= 0x01;
    let changed = cell_content_hash(CellId(42), &payload, &descriptor);

    assert_ne!(original, changed);
}

#[test]
fn accountability_cell_content_hash_changes_on_descriptor_metadata_mutation() {
    let payload = sample_payload("payload-asserted-hash");
    let descriptor = sample_descriptor("descriptor-asserted-hash");
    let original = cell_content_hash(CellId(42), &payload, &descriptor);

    let mut changed_descriptor = descriptor.clone();
    changed_descriptor.source_id = Some("source-b".to_owned());
    let changed = cell_content_hash(CellId(42), &payload, &changed_descriptor);

    assert_ne!(original, changed);
}

#[test]
fn accountability_cell_content_hash_is_not_payload_string_content_hash() {
    let payload = sample_payload("payload-asserted-hash");
    let descriptor = sample_descriptor("descriptor-asserted-hash");
    let retrieved = RetrievedCell {
        cell_id: CellId(42),
        payload,
        descriptor,
        captured_access_decision: None,
    };

    let hash = retrieved_cell_content_hash(&retrieved);

    assert_ne!(hash, "payload-asserted-hash");
    assert_ne!(hash, retrieved.descriptor.content_hash.as_deref().unwrap());
    assert_eq!(
        hash,
        cell_content_hash(retrieved.cell_id, &retrieved.payload, &retrieved.descriptor)
    );
}

#[test]
fn accountability_cell_content_hash_uses_domain_separated_blake3() {
    assert_eq!(
        ACCOUNTABILITY_CELL_HASH_DOMAIN,
        "cortexdb.accountability.cell_hash.v1"
    );
    let descriptor = sample_descriptor("descriptor-asserted-hash");
    let payload = sample_payload("payload-asserted-hash");

    let domain_hash = cell_content_hash(CellId(42), &payload, &descriptor);
    let raw_canonical_hash = cortex_crypto::hex_lower(&cortex_crypto::blake3_256(
        &canonical_cell_bytes(CellId(42), &payload, &descriptor),
    ));

    assert_ne!(domain_hash, raw_canonical_hash);
}

fn sample_payload(self_asserted_hash: &str) -> Vec<u8> {
    format!(
        "scope=project:accountability\nstatus=verified\ntype=fact\ncontent_hash={self_asserted_hash}\n\nCortexDB receipt cells anchor raw bytes."
    )
    .into_bytes()
}

fn sample_descriptor(content_hash: &str) -> CellDescriptor {
    CellDescriptor {
        scope: "project:accountability".to_owned(),
        status: "verified".to_owned(),
        cell_type: KnowledgeCellType::Fact,
        memory_type: Some("evidence".to_owned()),
        ttl_seconds: Some(86_400),
        created_unix_seconds: Some(1_710_000_000),
        source_trust_q16: Some(60_000),
        source: Some("roadmap-fixture".to_owned()),
        citation: Some("docs/ACCOUNTABILITY_ROADMAP.md".to_owned()),
        content_hash: Some(content_hash.to_owned()),
        source_id: Some("source-a".to_owned()),
        source_url: Some("file://docs/ACCOUNTABILITY_ROADMAP.md".to_owned()),
        document_id: Some("accountability-roadmap".to_owned()),
        page: Some(3),
        row: Some(7),
        cell_range: Some("A1:B4".to_owned()),
        json_path: Some("$.phases[3].AR-3".to_owned()),
        confidence_q16: Some(65_000),
        parent_id: Some("parent-cell".to_owned()),
        valid_from: Some("2026-06-28".to_owned()),
        valid_to: None,
        session_id: Some("session-accountability".to_owned()),
        session_kind: Some("roadmap-implementation".to_owned()),
    }
}
