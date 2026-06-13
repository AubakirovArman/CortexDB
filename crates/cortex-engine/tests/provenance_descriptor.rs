use std::collections::BTreeMap;

use cortex_core::{CellDescriptor, CellId, CommitSeq, KnowledgeCellType};
use cortex_engine::{
    descriptor_from_decoded_wal_record, wal_record_from_operation_with_metadata, CellMetadata,
    ContextPack, ContextPackExportFormat, ContextPackOptions, DbOperation, RetrievedCell,
};
use cortex_storage::wal::WalCodec;
use serde_json::Value;

#[test]
fn descriptor_metadata_builds_source_ref_without_payload_headers() {
    let descriptor = CellDescriptor {
        scope: "project:secure".to_owned(),
        source: Some("descriptor-source".to_owned()),
        citation: Some("descriptor-citation".to_owned()),
        source_id: Some("ifc:project-1".to_owned()),
        source_url: Some("https://example.test/project-1".to_owned()),
        document_id: Some("doc-1".to_owned()),
        page: Some(7),
        row: Some(8),
        cell_range: Some("chunk-9".to_owned()),
        json_path: Some("a.b".to_owned()),
        confidence_q16: Some(55_000),
        ..CellDescriptor::default()
    };
    let metadata = CellMetadata::from_payload_with_descriptor(b"body without headers", &descriptor);
    let source_ref = metadata.source_ref.unwrap();

    assert_eq!(metadata.document_id.as_deref(), Some("doc-1"));
    assert_eq!(metadata.citation.as_deref(), Some("descriptor-citation"));
    assert_eq!(source_ref.source_id, "ifc:project-1");
    assert_eq!(
        source_ref.source_url.as_deref(),
        Some("https://example.test/project-1")
    );
    assert_eq!(source_ref.document_id.as_deref(), Some("doc-1"));
    assert_eq!(source_ref.page, Some(7));
    assert_eq!(source_ref.row, Some(8));
    assert_eq!(source_ref.cell_range.as_deref(), Some("chunk-9"));
    assert_eq!(source_ref.json_path.as_deref(), Some("a.b"));
    assert_eq!(source_ref.confidence_q16, 55_000);
}

#[test]
fn context_pack_exports_descriptor_source_ref_without_payload_headers() {
    let descriptor = CellDescriptor {
        scope: "project:secure".to_owned(),
        status: "ready".to_owned(),
        cell_type: KnowledgeCellType::Fact,
        citation: Some("descriptor-citation".to_owned()),
        source_id: Some("descriptor-ref".to_owned()),
        document_id: Some("descriptor-doc".to_owned()),
        page: Some(9),
        cell_range: Some("descriptor-chunk".to_owned()),
        confidence_q16: Some(55_000),
        ..CellDescriptor::default()
    };
    let cell = RetrievedCell {
        cell_id: CellId(42),
        payload: b"secure evidence".to_vec(),
        descriptor,
    };
    let pack = ContextPack::from_retrieved_with_feedback_options_and_view(
        vec![cell],
        1_000,
        true,
        &ContextPackOptions::default(),
        "secure evidence",
        &BTreeMap::new(),
        None,
    );
    let json: Value = serde_json::from_str(&pack.export(ContextPackExportFormat::Json)).unwrap();

    assert_eq!(json["cells"][0]["citation"], "descriptor-citation");
    assert_eq!(
        json["cells"][0]["source_ref"]["source_id"],
        "descriptor-ref"
    );
    assert_eq!(
        json["cells"][0]["source_ref"]["document_id"],
        "descriptor-doc"
    );
    assert_eq!(json["cells"][0]["source_ref"]["page"], 9);
    assert_eq!(
        json["cells"][0]["source_ref"]["cell_range"],
        "descriptor-chunk"
    );
    assert_eq!(json["cells"][0]["source_ref"]["confidence_q16"], 55_000);
}

#[test]
fn wal_metadata_overlay_preserves_payload_provenance_descriptor() {
    let operation = DbOperation::PutCell {
        cell_id: CellId(7),
        payload: b"scope=payload\nstatus=ready\ntype=fact\nsource=payload-source\ncitation=payload-citation\ncontent_hash=payload-hash\nsource_id=ifc:project-1\ndocument_id=doc-1\npage=7\ncell_range=chunk-9\nconfidence_q16=55000\n\nhello"
            .to_vec(),
    };
    let metadata = b"cortexdb.cell_metadata.v1\nscope=metadata\nstatus=verified\ntype=document_block\nsource=metadata-source"
        .to_vec();
    let record = wal_record_from_operation_with_metadata(CommitSeq(3), &operation, metadata);
    let encoded = WalCodec::encode_record(&record).unwrap();
    let decoded = WalCodec::decode_record(&encoded).unwrap();
    let descriptor = descriptor_from_decoded_wal_record(&decoded)
        .unwrap()
        .expect("descriptor section");

    assert_eq!(descriptor.scope, "metadata");
    assert_eq!(descriptor.status, "verified");
    assert_eq!(descriptor.cell_type, KnowledgeCellType::DocumentBlock);
    assert_eq!(descriptor.source.as_deref(), Some("metadata-source"));
    assert_eq!(descriptor.citation.as_deref(), Some("payload-citation"));
    assert_eq!(descriptor.content_hash.as_deref(), Some("payload-hash"));
    assert_eq!(descriptor.source_id.as_deref(), Some("ifc:project-1"));
    assert_eq!(descriptor.document_id.as_deref(), Some("doc-1"));
    assert_eq!(descriptor.page, Some(7));
    assert_eq!(descriptor.cell_range.as_deref(), Some("chunk-9"));
    assert_eq!(descriptor.confidence_q16, Some(55_000));
}
