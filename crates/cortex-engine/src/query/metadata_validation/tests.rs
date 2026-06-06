use super::*;
use crate::query::metadata::CellMetadata;

#[test]
fn decode_payload_valid() {
    let m =
        CellMetadata::decode_payload(b"scope=project:test\nstatus=ready\n\nhello world").unwrap();
    assert_eq!(m.scope, "project:test");
    assert_eq!(m.status, "ready");
    assert_eq!(m.body_text, "hello world");
}

#[test]
fn decode_payload_missing_scope_fails() {
    assert!(CellMetadata::decode_payload(b"status=ready\n\nhello").is_err());
}

#[test]
fn decode_payload_invalid_type_fails() {
    assert!(CellMetadata::decode_payload(
        b"scope=project:test\nstatus=ready\ntype=unknown_type\n\nhello"
    )
    .is_err());
}

#[test]
fn valid_metadata_passes() {
    let m = CellMetadata::from_payload(b"scope=project:test\nstatus=ready\n\nhello world");
    assert!(m.validate().is_ok());
}

#[test]
fn empty_scope_fails() {
    let m = CellMetadata::from_payload(b"scope=\nstatus=ready\n\nhello");
    assert_eq!(m.validate(), Err(MetadataValidationError::EmptyScope));
}

#[test]
fn path_traversal_scope_fails() {
    let m = CellMetadata::from_payload(b"scope=../etc\nstatus=ready\n\nhello");
    assert_eq!(
        m.validate(),
        Err(MetadataValidationError::InvalidScopeCharacters(
            "../etc".to_owned()
        ))
    );
}

#[test]
fn slash_in_scope_fails() {
    let m = CellMetadata::from_payload(b"scope=a/b\nstatus=ready\n\nhello");
    assert_eq!(
        m.validate(),
        Err(MetadataValidationError::InvalidScopeCharacters(
            "a/b".to_owned()
        ))
    );
}

#[test]
fn empty_status_fails() {
    let m = CellMetadata::from_payload(b"scope=project:test\nstatus=\n\nhello");
    assert_eq!(m.validate(), Err(MetadataValidationError::EmptyStatus));
}

#[test]
fn zero_ttl_fails() {
    let m = CellMetadata::from_payload(b"scope=project:test\nstatus=ready\nttl_seconds=0\n\nhello");
    assert_eq!(
        m.validate(),
        Err(MetadataValidationError::InvalidTtlSeconds(0))
    );
}

#[test]
fn out_of_range_source_trust_is_gracefully_dropped() {
    let m = CellMetadata::from_payload(
        b"scope=project:test\nstatus=ready\nsource_trust_q16=99999\n\nhello",
    );
    assert_eq!(m.source_trust_q16, None);
}

#[test]
fn sanitize_fixes_invalid_scope() {
    let m = CellMetadata::from_payload(b"scope=../etc\nstatus=ready\n\nhello");
    let fixed = m.sanitized();
    assert_eq!(fixed.scope, "default");
    assert_eq!(fixed.status, "ready");
}

#[test]
fn sanitize_fixes_invalid_status() {
    let m = CellMetadata::from_payload(b"scope=project:test\nstatus=\n\nhello");
    let fixed = m.sanitized();
    assert_eq!(fixed.status, "ready");
}

#[test]
fn sanitize_keeps_valid_source_trust() {
    let m = CellMetadata::from_payload(
        b"scope=project:test\nstatus=ready\nsource_trust_q16=60000\n\nhello",
    );
    let fixed = m.sanitized();
    assert_eq!(fixed.source_trust_q16, Some(60000));
}

#[test]
fn source_trust_class_calibrates_source_ref_confidence() {
    let m = CellMetadata::from_payload(
        b"scope=project:test\nstatus=ready\nsource=doc\nsource_trust_class=internal\n\nhello",
    );
    let source_ref = m.source_ref.unwrap();
    assert_eq!(m.source_trust_class.unwrap().as_str(), "internal");
    assert_eq!(
        source_ref.confidence_q16,
        crate::source_trust::INTERNAL_SOURCE_TRUST_Q16
    );
}

#[test]
fn strict_decode_reads_source_trust_class() {
    let m = CellMetadata::decode_payload(
        b"scope=project:test\nstatus=ready\nsource=doc\nsource_trust_class=inferred\n\nhello",
    )
    .unwrap();
    assert_eq!(m.source_trust_class.unwrap().as_str(), "inferred");
}

#[test]
fn sanitize_clears_zero_ttl() {
    let m = CellMetadata::from_payload(b"scope=project:test\nstatus=ready\nttl_seconds=0\n\nhello");
    let fixed = m.sanitized();
    assert_eq!(fixed.ttl_seconds, None);
}
