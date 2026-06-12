use super::{CellDescriptor, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};

#[test]
fn descriptor_decodes_stable_payload_headers() {
    let payload = b"scope=project:investments\nstatus=verified\ntype=fact\nmemory_type=decision\nttl_seconds=3600\ncreated_unix_seconds=1710000000\nsource_trust_q16=60000\nsource=annual-report\ncitation=p12\ncontent_hash=abc\nparent_chunk_id=doc-1\nvalid_from=2024-01-01\nvalid_to=2024-12-31\n\nbody";
    let descriptor = CellDescriptor::from_payload_lossy(payload);

    assert_eq!(descriptor.scope, "project:investments");
    assert_eq!(descriptor.status, "verified");
    assert_eq!(descriptor.cell_type, KnowledgeCellType::Fact);
    assert_eq!(descriptor.memory_type.as_deref(), Some("decision"));
    assert_eq!(descriptor.ttl_seconds, Some(3600));
    assert_eq!(descriptor.created_unix_seconds, Some(1_710_000_000));
    assert_eq!(descriptor.source_trust_q16, Some(60_000));
    assert_eq!(descriptor.source.as_deref(), Some("annual-report"));
    assert_eq!(descriptor.citation.as_deref(), Some("p12"));
    assert_eq!(descriptor.content_hash.as_deref(), Some("abc"));
    assert_eq!(descriptor.parent_id.as_deref(), Some("doc-1"));
    assert_eq!(descriptor.valid_from.as_deref(), Some("2024-01-01"));
    assert_eq!(descriptor.valid_to.as_deref(), Some("2024-12-31"));
}

#[test]
fn descriptor_matches_encoded_knowledge_cell_metadata() {
    let cell = KnowledgeCell::new(
        KnowledgeCellMetadata {
            scope: "agent:7".to_owned(),
            status: "ready".to_owned(),
            cell_type: KnowledgeCellType::Memory,
            memory_type: Some("preference".to_owned()),
            ttl_seconds: Some(30),
            created_unix_seconds: Some(42),
            source_trust_q16: Some(50_000),
            source: Some("user".to_owned()),
        },
        "remember this",
    );

    let descriptor = CellDescriptor::from_payload_lossy(&cell.encode_payload());
    assert_eq!(descriptor, CellDescriptor::from_metadata(&cell.metadata));
}

#[test]
fn descriptor_decodes_cell_metadata_section() {
    let metadata = KnowledgeCellMetadata {
        scope: "tenant:private".to_owned(),
        status: "ready".to_owned(),
        cell_type: KnowledgeCellType::Fact,
        memory_type: Some("decision".to_owned()),
        ttl_seconds: Some(600),
        created_unix_seconds: Some(1_710_000_001),
        source_trust_q16: Some(61_000),
        source: Some("source-b".to_owned()),
    };

    let descriptor = CellDescriptor::from_metadata_section_lossy(&metadata.encode_wal_section());

    assert_eq!(descriptor, CellDescriptor::from_metadata(&metadata));
}

#[test]
fn descriptor_binary_section_roundtrips_and_skips_unknown_fields() {
    let descriptor = CellDescriptor {
        scope: "project:beta".to_owned(),
        status: "verified".to_owned(),
        cell_type: KnowledgeCellType::Fact,
        memory_type: Some("decision".to_owned()),
        ttl_seconds: Some(3600),
        created_unix_seconds: Some(1_710_000_000),
        source_trust_q16: Some(60_000),
        source: Some("source-a".to_owned()),
        citation: Some("p7".to_owned()),
        content_hash: Some("hash".to_owned()),
        parent_id: Some("parent".to_owned()),
        valid_from: Some("2024-01-01".to_owned()),
        valid_to: Some("2024-12-31".to_owned()),
    };
    let mut encoded = descriptor.encode_section_v1();
    encoded.push(250);
    encoded.extend_from_slice(&3u32.to_le_bytes());
    encoded.extend_from_slice(b"new");

    assert_eq!(
        CellDescriptor::decode_section_v1(&encoded),
        Some(descriptor)
    );
}
