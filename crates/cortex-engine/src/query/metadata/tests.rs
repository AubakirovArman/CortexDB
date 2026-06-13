use cortex_aql::MemoryType;

use super::CellMetadata;

#[test]
fn weighted_lexical_terms_include_document_views() {
    let metadata = CellMetadata::from_payload(
            b"scope=docs\nstatus=ready\ntitle=Payments Migration\npath=confluence/payments/runbook\ndocument_id=doc-payments\nchunk_id=chunk-7\nparent_id=chunk-parent\nchunk_role=child\nsection=Rollout Plan\nproject=Apollo\nentity=Payments API\nsector=platform\nsource=confluence\n\nbody mentions payments once",
        );
    let terms = metadata.weighted_lexical_terms();

    assert_eq!(metadata.parent_id.as_deref(), Some("chunk-parent"));
    assert_eq!(metadata.chunk_role.as_deref(), Some("child"));
    assert!(terms.get("migration").copied().unwrap_or(0) >= 8);
    assert!(terms.get("runbook").copied().unwrap_or(0) >= 5);
    assert!(terms.get("apollo").copied().unwrap_or(0) >= 4);
    assert!(terms.get("chunk").copied().unwrap_or(0) >= 2);
}

#[test]
fn embedding_lines_do_not_pollute_body_text() {
    let metadata = CellMetadata::from_payload(
            b"scope=docs\nstatus=ready\nembedding_model=bge-m3\nembedding_text_hash=abc\nvector=1,2,3\ntitle_vector=3,2,1\n\nAlpha body",
        );

    assert_eq!(metadata.body_text, "Alpha body");
    assert!(!metadata.terms.contains(&"vector".to_owned()));
    assert!(!metadata.terms.contains(&"bge".to_owned()));
    assert!(!metadata.terms.contains(&"title".to_owned()));
}

#[test]
fn descriptor_metadata_preserves_legacy_content_hash_when_descriptor_omits_it() {
    let payload = b"scope=payload\nstatus=ready\ncontent_hash=legacy-hash\n\nbody";
    let descriptor = cortex_core::CellDescriptor {
        scope: "descriptor".to_owned(),
        content_hash: None,
        ..cortex_core::CellDescriptor::default()
    };

    let metadata = CellMetadata::from_payload_with_descriptor(payload, &descriptor);

    assert_eq!(metadata.scope, "descriptor");
    assert_eq!(metadata.content_hash.as_deref(), Some("legacy-hash"));

    let descriptor = cortex_core::CellDescriptor {
        scope: "descriptor".to_owned(),
        content_hash: Some("descriptor-hash".to_owned()),
        ..cortex_core::CellDescriptor::default()
    };
    let metadata = CellMetadata::from_payload_with_descriptor(payload, &descriptor);

    assert_eq!(metadata.content_hash.as_deref(), Some("descriptor-hash"));
}

#[test]
fn descriptor_metadata_profile_avoids_legacy_payload_metadata_parser() {
    CellMetadata::reset_legacy_payload_metadata_parse_profile();
    let payload = b"scope=payload\nstatus=blocked\ntype=memory\nmemory_type=preference\nttl_seconds=1\ncreated_unix_seconds=2\nsource_trust_q16=1\nsource=payload-source\ncitation=payload-citation\nparent_id=payload-parent\nvalid_from=1900-01-01\nvalid_to=1900-01-02\ntitle=Useful title\nproject=Apollo\ncontent_hash=legacy-hash\nsource_id=payload-ref\nconfidence_q16=123\n\nbody";
    let descriptor = cortex_core::CellDescriptor {
        scope: "descriptor".to_owned(),
        status: "ready".to_owned(),
        cell_type: cortex_core::KnowledgeCellType::Fact,
        memory_type: Some("decision".to_owned()),
        ttl_seconds: Some(3600),
        created_unix_seconds: Some(123_456),
        source_trust_q16: Some(50_000),
        source: Some("descriptor-source".to_owned()),
        citation: Some("descriptor-citation".to_owned()),
        content_hash: Some("descriptor-hash".to_owned()),
        parent_id: Some("descriptor-parent".to_owned()),
        valid_from: Some("2026-01-01".to_owned()),
        valid_to: Some("2026-12-31".to_owned()),
        ..cortex_core::CellDescriptor::default()
    };
    let metadata = CellMetadata::from_payload_with_descriptor(payload, &descriptor);

    assert_eq!(CellMetadata::legacy_payload_metadata_parse_profile(), 0);
    assert_eq!(metadata.scope, "descriptor");
    assert_eq!(metadata.status, "ready");
    assert_eq!(metadata.cell_type, "fact");
    assert_eq!(metadata.memory_type, Some(MemoryType::Decision));
    assert_eq!(metadata.ttl_seconds, Some(3600));
    assert_eq!(metadata.created_unix_seconds, Some(123_456));
    assert_eq!(metadata.source_trust_q16, Some(50_000));
    assert_eq!(metadata.source.as_deref(), Some("descriptor-source"));
    assert_eq!(metadata.citation.as_deref(), Some("descriptor-citation"));
    assert_eq!(metadata.content_hash.as_deref(), Some("descriptor-hash"));
    assert_eq!(metadata.parent_id.as_deref(), Some("descriptor-parent"));
    assert_eq!(metadata.valid_from.as_deref(), Some("2026-01-01"));
    assert_eq!(metadata.valid_to.as_deref(), Some("2026-12-31"));
    assert_eq!(metadata.title.as_deref(), Some("Useful title"));
    assert_eq!(metadata.project.as_deref(), Some("Apollo"));
}

#[test]
fn weighted_lexical_terms_include_table_views() {
    let metadata = CellMetadata::from_payload(
            b"scope=docs\nstatus=ready\ntype=table\nsource=csv\ntable_id=budget.csv\ntable_headers=project|budget|owner\nrow_label=Apollo\ncell_range=row-7\n\nproject: Apollo\nbudget: 12000",
        );
    let terms = metadata.weighted_lexical_terms();

    assert_eq!(metadata.cell_type, "table");
    assert_eq!(metadata.table_id.as_deref(), Some("budget.csv"));
    assert_eq!(metadata.row_label.as_deref(), Some("Apollo"));
    assert!(terms.get("budget").copied().unwrap_or(0) >= 6);
    assert!(terms.get("apollo").copied().unwrap_or(0) >= 6);
    assert!(terms.get("row").copied().unwrap_or(0) >= 6);
}

#[test]
fn weighted_lexical_terms_include_enrichment_views() {
    let metadata = CellMetadata::from_payload(
            b"scope=docs\nstatus=ready\nproject=Apollo\nowner=Alice Lee\nstatus_tag=blocked\nevent_date=2026-05-14\ntopic=Migration Runbook\n\nbody",
        );
    let terms = metadata.weighted_lexical_terms();

    assert_eq!(metadata.project.as_deref(), Some("Apollo"));
    assert_eq!(metadata.owner.as_deref(), Some("Alice Lee"));
    assert_eq!(metadata.status_tag.as_deref(), Some("blocked"));
    assert_eq!(metadata.event_date.as_deref(), Some("2026-05-14"));
    assert_eq!(metadata.topic.as_deref(), Some("Migration Runbook"));
    assert!(terms.get("alice").copied().unwrap_or(0) >= 4);
    assert!(terms.get("blocked").copied().unwrap_or(0) >= 4);
    assert!(terms.get("migration").copied().unwrap_or(0) >= 4);
}
