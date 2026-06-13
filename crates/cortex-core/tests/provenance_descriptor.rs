use cortex_core::{CellDescriptor, KnowledgeCellType};

#[test]
fn descriptor_decodes_source_ref_payload_headers() {
    let payload = b"scope=project:investments\nstatus=verified\ntype=fact\nsource_id=ifc:project-1\nsource_url=https://example.test/project-1\ndocument_id=doc-1\npage=7\nrow=8\ncell_range=chunk-9\njson_path=a.b\nconfidence_q16=55000\n\nbody";
    let descriptor = CellDescriptor::from_payload_lossy(payload);

    assert_eq!(descriptor.source_id.as_deref(), Some("ifc:project-1"));
    assert_eq!(
        descriptor.source_url.as_deref(),
        Some("https://example.test/project-1")
    );
    assert_eq!(descriptor.document_id.as_deref(), Some("doc-1"));
    assert_eq!(descriptor.page, Some(7));
    assert_eq!(descriptor.row, Some(8));
    assert_eq!(descriptor.cell_range.as_deref(), Some("chunk-9"));
    assert_eq!(descriptor.json_path.as_deref(), Some("a.b"));
    assert_eq!(descriptor.confidence_q16, Some(55_000));
}

#[test]
fn descriptor_binary_section_roundtrips_source_ref_fields() {
    let descriptor = CellDescriptor {
        scope: "project:beta".to_owned(),
        status: "verified".to_owned(),
        cell_type: KnowledgeCellType::Fact,
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

    assert_eq!(
        CellDescriptor::decode_section_v1(&descriptor.encode_section_v1()),
        Some(descriptor)
    );
}
