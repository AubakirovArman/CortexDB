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
mod tests;
