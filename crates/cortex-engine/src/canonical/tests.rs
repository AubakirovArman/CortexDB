//! Canonical-serialization unit tests (moved from canonical.rs; behavior unchanged).

use cortex_aql::Q16_ONE;
use cortex_core::CellId;
use serde_json::{json, Map, Value};

use super::*;
use crate::context::{
    ContextAccessDecisionOutcome, ContextPackAnomalyCode, SourceFreshnessCategory,
};
use crate::query::CellMetadata;
use crate::source_trust::SourceTrustCategory;
use crate::verification::{
    Magnitude, VerificationGuardCode, VerificationMatchKind, VerificationNumericConflictKind,
    VerificationStatus,
};

// C3-5: the set of fields that enter each canonical (receipt-signed)
// serialization is bound to its schema version by an external fixture. This
// red-test fails if a canonical field is added/removed/renamed without going
// through the additive-minor procedure (bump the schema version, add a new
// fixture entry, re-baseline goldens) — so an un-versioned change can never
// C4-2 (foundation): the canonical JSON layer is language-independent. This
// asserts the SAME committed digests that scripts/canonical_jcs_cross_language_check.py
// (a pure-Python re-implementation) asserts — both languages agreeing on the
// sha256 of the canonical bytes proves the bytes are identical, so the
// receipt's canonicalization can be reproduced from the spec in any language.
#[test]
fn jcs_cross_language_vectors_match() {
    let vectors: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../fixtures/canonical/jcs_conformance_vectors.v1.json"
    ))
    .expect("jcs conformance vectors parse");
    assert!(!vectors.is_empty());
    for (index, entry) in vectors.iter().enumerate() {
        let bytes = canonical_json_bytes(&entry["value"]);
        let digest = cortex_crypto::hex_lower(&cortex_crypto::sha256(&bytes));
        let expected = entry["canonical_sha256"].as_str().expect("digest string");
        assert_eq!(
            digest, expected,
            "vector {index}: Rust canonical digest differs from the committed \
                 (Python-derived) digest — canonicalization is not cross-language stable"
        );
    }
}

// silently alter what the accountability receipt signs. See
// docs/RECEIPT_SCHEMA_VERSIONING.md.
#[test]
fn canonical_field_sets_are_bound_to_schema_versions() {
    let binding: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/canonical/schema_field_binding_v1.json"
    ))
    .expect("field-binding fixture parses");
    let schemas = binding
        .get("schemas")
        .and_then(Value::as_object)
        .expect("fixture has a schemas map");

    let expect = |schema: &str, code_fields: &[&str]| {
        let recorded: Vec<String> = schemas
            .get(schema)
            .and_then(Value::as_array)
            .unwrap_or_else(|| panic!("fixture missing schema {schema}"))
            .iter()
            .map(|value| value.as_str().unwrap().to_owned())
            .collect();
        let mut sorted = code_fields
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        sorted.sort();
        assert_eq!(
            recorded, sorted,
            "canonical field set for {schema} changed without a schema-version bump; \
                 follow docs/RECEIPT_SCHEMA_VERSIONING.md"
        );
    };

    expect("context_pack.canonical.v1", CONTEXT_PACK_HASHED_FIELDS);
    expect(
        "verification_report.canonical.v1",
        VERIFICATION_REPORT_HASHED_FIELDS,
    );
    // Exactly the known canonical schemas — a new receipt-signed schema must
    // arrive with its own fixture entry, not silently.
    let mut keys: Vec<&str> = schemas.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "context_pack.canonical.v1",
            "verification_report.canonical.v1"
        ]
    );
}

#[test]
fn canonical_json_bytes_sort_object_keys_recursively() {
    let left = nested_object([
        ("b", json!(2)),
        ("a", nested_object([("d", json!(4)), ("c", json!(3))])),
    ]);
    let right = nested_object([
        ("a", nested_object([("c", json!(3)), ("d", json!(4))])),
        ("b", json!(2)),
    ]);

    assert_eq!(canonical_json_bytes(&left), br#"{"a":{"c":3,"d":4},"b":2}"#);
    assert_eq!(canonical_json_bytes(&left), canonical_json_bytes(&right));
}

#[test]
fn context_pack_canonical_bytes_are_stable_and_clock_free() {
    let pack = sample_pack();

    let first = canonical_context_pack_bytes(&pack);
    let second = canonical_context_pack_bytes(&pack);
    let text = std::str::from_utf8(&first).unwrap();

    assert_eq!(first, second);
    assert!(text.contains(r#""schema_version":"context_pack.canonical.v1""#));
    assert!(text.contains(r#""payload_hex":"73636f70653d70726f6a6563743a696e766573746d656e74730a0a536f6c617220706c616e7420627564676574""#));
    for field in EXCLUDED_TELEMETRY_FIELDS {
        assert!(!text.contains(field));
    }
}

#[test]
fn verification_report_canonical_bytes_are_stable_and_clock_free() {
    let report = sample_report();

    let first = canonical_verification_report_bytes(&report);
    let second = canonical_verification_report_bytes(&report);
    let text = std::str::from_utf8(&first).unwrap();

    assert_eq!(first, second);
    assert!(text.contains(r#""schema_version":"verification_report.canonical.v1""#));
    assert!(text.contains(r#""status":"mixed_evidence""#));
    assert!(text.contains(r#""magnitude":"billion""#));
    for field in EXCLUDED_TELEMETRY_FIELDS {
        assert!(!text.contains(field));
    }
}

#[test]
fn verification_report_canonical_conflict_kind_is_hashed() {
    let mut report = sample_report();

    let numeric = canonical_verification_report_bytes(&report);
    report.numeric_conflicts[0].kind = VerificationNumericConflictKind::Citation;
    let citation = canonical_verification_report_bytes(&report);
    let text = std::str::from_utf8(&citation).unwrap();

    assert_ne!(numeric, citation);
    assert!(text.contains(r#""kind":"citation""#));
    for field in EXCLUDED_TELEMETRY_FIELDS {
        assert!(!text.contains(field));
    }
}

#[test]
fn canonical_field_allowlists_are_explicit() {
    assert_eq!(
        CONTEXT_PACK_HASHED_FIELDS,
        &[
            "schema_version",
            "token_budget_tokens",
            "estimated_tokens",
            "truncated",
            "citations_required",
            "answerability_q16",
            "conflict_visibility_q16",
            "visible_conflict_count",
            "cells",
            "anomalies",
            "grounding_report",
        ]
    );
    assert_eq!(
        VERIFICATION_REPORT_HASHED_FIELDS,
        &[
            "schema_version",
            "fact",
            "status",
            "confidence_q16",
            "evidence",
            "contradicting_evidence",
            "guards",
            "numeric_conflicts",
        ]
    );
    assert!(CONTEXT_PACK_EXPORTED_ONLY_FIELDS.is_empty());
    assert!(VERIFICATION_REPORT_EXPORTED_ONLY_FIELDS.is_empty());
    assert!(EXCLUDED_TELEMETRY_FIELDS.contains(&"elapsed_nanos"));
    assert!(EXCLUDED_TELEMETRY_FIELDS.contains(&"total_elapsed_nanos"));
}

fn nested_object<const N: usize>(entries: [(&str, Value); N]) -> Value {
    let mut map = Map::new();
    for (key, value) in entries {
        map.insert(key.to_owned(), value);
    }
    Value::Object(map)
}

fn sample_pack() -> ContextPack {
    ContextPack {
        cells: vec![ContextPackCell {
            cell_id: CellId(10),
            payload: b"scope=project:investments\n\nSolar plant budget".to_vec(),
            metadata: sample_metadata(),
            estimated_tokens: 7,
            citation: Some("doc-a".to_owned()),
            provenance: Some(ContextSpanProvenance {
                source_cell_id: CellId(9),
                source_byte_start: 0,
                source_byte_end: 18,
                source_line_start: 1,
                source_line_end: 1,
                source_ref: Some(sample_source_ref()),
            }),
            explain: Some(ContextExplain {
                score: 42,
                matched_terms: vec!["solar".to_owned(), "budget".to_owned()],
                why_selected: "matched query terms".to_owned(),
                score_components: vec![ContextScoreComponent {
                    name: "bm25".to_owned(),
                    value: 42,
                    contribution: 42,
                    reason: "canonical lexical score".to_owned(),
                }],
                base_bm25: 42,
                source_trust_q16: 50_000,
                source_trust_category: SourceTrustCategory::High,
                source_trust_bonus: 50_000,
                source_freshness_q16: 65_535,
                source_freshness_category: SourceFreshnessCategory::Current,
                source_freshness_bonus: 32_767,
                redundancy_penalty: 0,
            }),
            access_decision: Some(ContextAccessDecision {
                cell_id: CellId(10),
                decision: ContextAccessDecisionOutcome::Allowed,
                policy: "agent_view_scope".to_owned(),
                policy_version: Some("agent_view_readable_scope.v1".to_owned()),
                reason: "scope allowed".to_owned(),
                scope: "project:investments".to_owned(),
                scope_id: 1,
                agent_id: Some(7),
                agent_view_digest: Some(
                    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_owned(),
                ),
            }),
        }],
        token_budget_tokens: 100,
        estimated_tokens: 7,
        truncated: false,
        citations_required: true,
        answerability_q16: Q16_ONE,
        conflict_visibility_q16: 0,
        visible_conflict_count: 0,
        anomalies: vec![ContextPackAnomaly {
            cell_id: Some(CellId(10)),
            code: ContextPackAnomalyCode::MissingCitation,
            message: "citation required".to_owned(),
            why_excluded: None,
        }],
        grounding_report: None,
    }
}

fn sample_metadata() -> CellMetadata {
    CellMetadata {
        scope: "project:investments".to_owned(),
        status: "ready".to_owned(),
        cell_type: "fact".to_owned(),
        memory_type: None,
        ttl_seconds: None,
        created_unix_seconds: None,
        source_trust_q16: Some(50_000),
        source_trust_class: None,
        source: Some("doc-a".to_owned()),
        citation: Some("doc-a".to_owned()),
        title: None,
        content_hash: None,
        source_hash: None,
        document_id: Some("doc-a".to_owned()),
        chunk_id: None,
        parent_id: None,
        chunk_role: None,
        path: None,
        section: None,
        project: Some("Solar Plant".to_owned()),
        entity: None,
        sector: None,
        owner: None,
        status_tag: None,
        event_date: None,
        topic: None,
        as_of: None,
        valid_from: None,
        valid_to: None,
        supersedes: None,
        superseded_by: None,
        compression_kind: None,
        compression_source_cells: Vec::new(),
        compression_answerability_q16: None,
        compression_worker: None,
        table_id: None,
        table_headers: None,
        row_label: None,
        body_text: "Solar plant budget".to_owned(),
        terms: vec!["solar".to_owned(), "budget".to_owned()],
        source_ref: Some(sample_source_ref()),
    }
}

fn sample_source_ref() -> SourceRef {
    SourceRef {
        source_id: "source-a".to_owned(),
        source_url: Some("https://example.test/a".to_owned()),
        document_id: Some("doc-a".to_owned()),
        page: Some(1),
        row: None,
        cell_range: None,
        json_path: Some("$.budget".to_owned()),
        confidence_q16: 60_000,
    }
}

fn sample_report() -> VerificationReport {
    VerificationReport {
        fact: "Solar Plant budget is 1.2B KZT".to_owned(),
        status: VerificationStatus::Mixed,
        confidence_q16: Q16_ONE,
        evidence: vec![sample_evidence(
            CellId(10),
            VerificationMatchKind::ExactText,
        )],
        contradicting_evidence: vec![sample_evidence(
            CellId(20),
            VerificationMatchKind::NumericContradiction,
        )],
        guards: vec![VerificationGuard {
            cell_id: Some(CellId(20)),
            code: VerificationGuardCode::NumericMismatch,
            message: "numeric value differs".to_owned(),
        }],
        numeric_conflicts: vec![VerificationNumericConflict {
            cell_id: CellId(20),
            kind: VerificationNumericConflictKind::Numeric,
            metric: "budget".to_owned(),
            left: "1.2B KZT".to_owned(),
            right: "1.4B KZT".to_owned(),
            fact_value: sample_numeric_value("1.2B KZT", 1_200_000_000),
            evidence_value: sample_numeric_value("1.4B KZT", 1_400_000_000),
        }],
    }
}

fn sample_evidence(cell_id: CellId, match_kind: VerificationMatchKind) -> VerificationEvidence {
    VerificationEvidence {
        cell_id,
        matched_terms: 4,
        match_score_q16: Q16_ONE,
        match_kind,
        source_trust_q16: 50_000,
        source_trust_category: SourceTrustCategory::High,
        citation: Some("doc-a".to_owned()),
    }
}

fn sample_numeric_value(raw: &str, scaled_value: u64) -> NumericValue {
    NumericValue {
        raw: raw.to_owned(),
        scaled_value,
        currency: Some("KZT".to_owned()),
        unit: None,
        magnitude: Some(Magnitude::Billion),
    }
}
