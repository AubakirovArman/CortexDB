use serde_json::{json, Value};

use crate::context::{
    AnswerGroundingReport, AnswerGroundingSpan, ContextAccessDecision, ContextExplain, ContextPack,
    ContextPackAnomaly, ContextPackCell, ContextScoreComponent, ContextSpanProvenance,
};
use crate::query::metadata::SourceRef;
use crate::verification::{
    Magnitude, NumericValue, VerificationEvidence, VerificationGuard, VerificationNumericConflict,
    VerificationReport,
};

pub const CONTEXT_PACK_HASHED_FIELDS: &[&str] = &[
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
];

pub const VERIFICATION_REPORT_HASHED_FIELDS: &[&str] = &[
    "schema_version",
    "fact",
    "status",
    "confidence_q16",
    "evidence",
    "contradicting_evidence",
    "guards",
    "numeric_conflicts",
];

pub const CONTEXT_PACK_EXPORTED_ONLY_FIELDS: &[&str] = &[];

pub const VERIFICATION_REPORT_EXPORTED_ONLY_FIELDS: &[&str] = &[];

pub const EXCLUDED_TELEMETRY_FIELDS: &[&str] = &[
    "elapsed_nanos",
    "total_elapsed_nanos",
    "Instant",
    "SystemTime",
];

pub fn canonical_context_pack_bytes(pack: &ContextPack) -> Vec<u8> {
    canonical_json_bytes(&context_pack_value(pack))
}

pub fn canonical_verification_report_bytes(report: &VerificationReport) -> Vec<u8> {
    canonical_json_bytes(&verification_report_value(report))
}

pub fn canonical_json_bytes(value: &Value) -> Vec<u8> {
    let mut out = String::new();
    write_canonical_value(value, &mut out);
    out.into_bytes()
}

fn write_canonical_value(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
        Value::Number(number) => out.push_str(&number.to_string()),
        Value::String(value) => write_json_string(value, out),
        Value::Array(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_canonical_value(item, out);
            }
            out.push(']');
        }
        Value::Object(object) => {
            let mut keys = object.keys().collect::<Vec<_>>();
            keys.sort();
            out.push('{');
            for (index, key) in keys.iter().enumerate() {
                if index > 0 {
                    out.push(',');
                }
                write_json_string(key, out);
                out.push(':');
                if let Some(item) = object.get(*key) {
                    write_canonical_value(item, out);
                } else {
                    out.push_str("null");
                }
            }
            out.push('}');
        }
    }
}

fn write_json_string(value: &str, out: &mut String) {
    match serde_json::to_string(value) {
        Ok(encoded) => out.push_str(&encoded),
        Err(_) => out.push_str("\"\""),
    }
}

fn context_pack_value(pack: &ContextPack) -> Value {
    json!({
        "schema_version": "context_pack.canonical.v1",
        "token_budget_tokens": pack.token_budget_tokens,
        "estimated_tokens": pack.estimated_tokens,
        "truncated": pack.truncated,
        "citations_required": pack.citations_required,
        "answerability_q16": pack.answerability_q16,
        "conflict_visibility_q16": pack.conflict_visibility_q16,
        "visible_conflict_count": pack.visible_conflict_count,
        "cells": pack.cells.iter().map(context_pack_cell_value).collect::<Vec<_>>(),
        "anomalies": pack.anomalies.iter().map(context_pack_anomaly_value).collect::<Vec<_>>(),
        "grounding_report": pack.grounding_report.as_ref().map(grounding_report_value),
    })
}

fn context_pack_cell_value(cell: &ContextPackCell) -> Value {
    json!({
        "cell_id": cell.cell_id.0,
        "payload_hex": hex_bytes(&cell.payload),
        "estimated_tokens": cell.estimated_tokens,
        "citation": cell.citation,
        "source_ref": cell.metadata.source_ref.as_ref().map(source_ref_value),
        "provenance": cell.provenance.as_ref().map(provenance_value),
        "explain": cell.explain.as_ref().map(explain_value),
        "access_decision": cell.access_decision.as_ref().map(access_decision_value),
    })
}

fn source_ref_value(source_ref: &SourceRef) -> Value {
    json!({
        "source_id": source_ref.source_id,
        "source_url": source_ref.source_url,
        "document_id": source_ref.document_id,
        "page": source_ref.page,
        "row": source_ref.row,
        "cell_range": source_ref.cell_range,
        "json_path": source_ref.json_path,
        "confidence_q16": source_ref.confidence_q16,
    })
}

fn provenance_value(provenance: &ContextSpanProvenance) -> Value {
    json!({
        "source_cell_id": provenance.source_cell_id.0,
        "source_byte_start": provenance.source_byte_start,
        "source_byte_end": provenance.source_byte_end,
        "source_line_start": provenance.source_line_start,
        "source_line_end": provenance.source_line_end,
        "source_ref": provenance.source_ref.as_ref().map(source_ref_value),
    })
}

fn access_decision_value(decision: &ContextAccessDecision) -> Value {
    json!({
        "cell_id": decision.cell_id.0,
        "decision": decision.decision.as_str(),
        "policy": decision.policy,
        "policy_version": decision.policy_version,
        "reason": decision.reason,
        "scope": decision.scope,
        "scope_id": decision.scope_id,
        "agent_id": decision.agent_id,
        "agent_view_digest": decision.agent_view_digest,
    })
}

fn explain_value(explain: &ContextExplain) -> Value {
    json!({
        "score": explain.score,
        "matched_terms": explain.matched_terms,
        "why_selected": explain.why_selected,
        "score_components": explain.score_components.iter().map(score_component_value).collect::<Vec<_>>(),
        "base_bm25": explain.base_bm25,
        "source_trust_q16": explain.source_trust_q16,
        "source_trust_category": explain.source_trust_category.as_str(),
        "source_trust_bonus": explain.source_trust_bonus,
        "source_freshness_q16": explain.source_freshness_q16,
        "source_freshness_category": explain.source_freshness_category.as_str(),
        "source_freshness_bonus": explain.source_freshness_bonus,
        "redundancy_penalty": explain.redundancy_penalty,
    })
}

fn score_component_value(component: &ContextScoreComponent) -> Value {
    json!({
        "name": component.name,
        "value": component.value,
        "contribution": component.contribution,
        "reason": component.reason,
    })
}

fn context_pack_anomaly_value(anomaly: &ContextPackAnomaly) -> Value {
    json!({
        "cell_id": anomaly.cell_id.map(|cell_id| cell_id.0),
        "code": anomaly.code.as_str(),
        "message": anomaly.message,
        "why_excluded": anomaly.why_excluded,
    })
}

fn grounding_report_value(report: &AnswerGroundingReport) -> Value {
    json!({
        "answer_supported": report.answer_supported,
        "rejected": report.rejected,
        "support_q16": report.support_q16,
        "supported_span_count": report.supported_span_count,
        "unsupported_span_count": report.unsupported_span_count,
        "spans": report.spans.iter().map(grounding_span_value).collect::<Vec<_>>(),
    })
}

fn grounding_span_value(span: &AnswerGroundingSpan) -> Value {
    json!({
        "text": span.text,
        "start_byte": span.start_byte,
        "end_byte": span.end_byte,
        "support_q16": span.support_q16,
        "supported": span.supported,
        "covered_terms": span.covered_terms,
        "missing_terms": span.missing_terms,
        "supported_by_cell_ids": span.supported_by_cell_ids.iter().map(|cell_id| cell_id.0).collect::<Vec<_>>(),
        "citations": span.citations,
    })
}

fn verification_report_value(report: &VerificationReport) -> Value {
    json!({
        "schema_version": "verification_report.canonical.v1",
        "fact": report.fact,
        "status": report.status.as_str(),
        "confidence_q16": report.confidence_q16,
        "evidence": report.evidence.iter().map(verification_evidence_value).collect::<Vec<_>>(),
        "contradicting_evidence": report.contradicting_evidence.iter().map(verification_evidence_value).collect::<Vec<_>>(),
        "guards": report.guards.iter().map(verification_guard_value).collect::<Vec<_>>(),
        "numeric_conflicts": report.numeric_conflicts.iter().map(numeric_conflict_value).collect::<Vec<_>>(),
    })
}

fn verification_evidence_value(evidence: &VerificationEvidence) -> Value {
    json!({
        "cell_id": evidence.cell_id.0,
        "matched_terms": evidence.matched_terms,
        "match_score_q16": evidence.match_score_q16,
        "match_kind": evidence.match_kind.as_str(),
        "source_trust_q16": evidence.source_trust_q16,
        "source_trust_category": evidence.source_trust_category.as_str(),
        "citation": evidence.citation,
    })
}

fn verification_guard_value(guard: &VerificationGuard) -> Value {
    json!({
        "cell_id": guard.cell_id.map(|cell_id| cell_id.0),
        "code": guard.code.as_str(),
        "message": guard.message,
    })
}

fn numeric_conflict_value(conflict: &VerificationNumericConflict) -> Value {
    json!({
        "cell_id": conflict.cell_id.0,
        "kind": conflict.kind.as_str(),
        "metric": conflict.metric,
        "left": conflict.left,
        "right": conflict.right,
        "fact_value": numeric_value(&conflict.fact_value),
        "evidence_value": numeric_value(&conflict.evidence_value),
    })
}

fn numeric_value(value: &NumericValue) -> Value {
    json!({
        "raw": value.raw,
        "scaled_value": value.scaled_value,
        "currency": value.currency,
        "unit": value.unit,
        "magnitude": value.magnitude.map(magnitude_str),
    })
}

fn magnitude_str(value: Magnitude) -> &'static str {
    match value {
        Magnitude::Billion => "billion",
        Magnitude::Million => "million",
        Magnitude::Thousand => "thousand",
        Magnitude::Percent => "percent",
    }
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        out.push(char::from(HEX[(byte >> 4) as usize]));
        out.push(char::from(HEX[(byte & 0x0f) as usize]));
    }
    out
}

#[cfg(test)]
mod tests {
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
                        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                            .to_owned(),
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
}
