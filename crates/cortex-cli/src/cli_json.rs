use cortex_engine::verification::{VerificationReport, VerificationStatus};
use cortex_engine::{ContextPack, Database};

pub(crate) fn context_pack_to_json(pack: &ContextPack) -> String {
    serde_json::json!({
        "token_budget_tokens": pack.token_budget_tokens,
        "estimated_tokens": pack.estimated_tokens,
        "truncated": pack.truncated,
        "citations_required": pack.citations_required,
        "cells": pack.cells.iter().map(context_cell_json).collect::<Vec<_>>(),
        "anomalies": pack.anomalies.iter().map(|anomaly| {
            serde_json::json!({
                "cell_id": anomaly.cell_id.map(|id| id.0),
                "code": anomaly.code.to_string(),
                "message": anomaly.message,
            })
        }).collect::<Vec<_>>(),
    })
    .to_string()
}

pub(crate) fn verification_report_to_json(report: &VerificationReport, db: &Database) -> String {
    let numeric_conflicts = report
        .guards
        .iter()
        .filter(|guard| guard.code == "numeric_mismatch")
        .filter_map(|guard| guard.cell_id)
        .filter_map(|cell_id| db.get_latest_cell(cell_id))
        .filter_map(|payload| extract_numeric_conflict(&report.fact, &payload))
        .collect::<Vec<_>>();

    serde_json::json!({
        "verdict": verification_verdict(report.status),
        "supporting": report.evidence.iter().map(|e| evidence_json(e, db)).collect::<Vec<_>>(),
        "contradicting": report
            .contradicting_evidence
            .iter()
            .map(|e| evidence_json(e, db))
            .collect::<Vec<_>>(),
        "numeric_conflicts": numeric_conflicts,
    })
    .to_string()
}

fn context_cell_json(cell: &cortex_engine::ContextPackCell) -> serde_json::Value {
    let metadata = cortex_engine::query::CellMetadata::from_payload(&cell.payload);
    serde_json::json!({
        "cell_id": cell.cell_id.0,
        "estimated_tokens": cell.estimated_tokens,
        "citation": cell.citation,
        "payload_text": String::from_utf8_lossy(&cell.payload),
        "explain": cell.explain.as_ref().map(|exp| {
            serde_json::json!({
                "score": exp.score,
                "matched_terms": exp.matched_terms,
                "why_selected": exp.why_selected,
                "base_bm25": exp.base_bm25,
                "source_trust_bonus": exp.source_trust_bonus,
                "redundancy_penalty": exp.redundancy_penalty,
            })
        }),
        "source_ref": metadata.source_ref.as_ref().map(|sr| {
            serde_json::json!({
                "source_id": sr.source_id,
                "document_id": sr.document_id,
                "page": sr.page,
                "cell_range": sr.cell_range,
                "json_path": sr.json_path,
                "confidence_q16": sr.confidence_q16,
            })
        }),
    })
}

fn evidence_json(
    evidence: &cortex_engine::verification::VerificationEvidence,
    db: &Database,
) -> serde_json::Value {
    let payload_text = db
        .get_latest_cell(evidence.cell_id)
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
        .unwrap_or_else(|| "null".to_owned());
    serde_json::json!({
        "cell_id": evidence.cell_id.0,
        "matched_terms": evidence.matched_terms,
        "source_trust_q16": evidence.source_trust_q16,
        "citation": evidence.citation,
        "payload_text": payload_text,
    })
}

fn verification_verdict(status: VerificationStatus) -> &'static str {
    match status {
        VerificationStatus::Supported => "supported",
        VerificationStatus::Insufficient => "insufficient",
        VerificationStatus::Contradicted => "contradicted",
        VerificationStatus::Mixed => "mixed_evidence",
    }
}

/// Pure-integer display formatter for numeric magnitudes.
/// Avoids f64 to stay deterministic and fixed-point aligned.
///
/// Examples:
///   1_200_000_000 -> "1.2B KZT"
///   1_500_000_000 -> "1.5B KZT"
///   1_000_000     -> "1M KZT"
///   1_250_000     -> "1.25M KZT"
fn format_scale_currency(value_str: &str, currency: &str) -> String {
    if let Ok(val) = value_str.parse::<u64>() {
        if val >= 1_000_000_000 {
            let whole = val / 1_000_000_000;
            let rem = val % 1_000_000_000;
            if rem == 0 {
                return format!("{}B {}", whole, currency);
            }
            // Two decimal digits of precision without float
            let frac = rem / 10_000_000;
            if frac % 10 == 0 {
                return format!("{}.{:.1}B {}", whole, frac / 10, currency);
            }
            return format!("{}.{:.2}B {}", whole, frac / 10, currency);
        } else if val >= 1_000_000 {
            let whole = val / 1_000_000;
            let rem = val % 1_000_000;
            if rem == 0 {
                return format!("{}M {}", whole, currency);
            }
            let frac = rem / 10_000;
            if frac % 10 == 0 {
                return format!("{}.{:.1}M {}", whole, frac / 10, currency);
            }
            return format!("{}.{:.2}M {}", whole, frac / 10, currency);
        }
    }
    format!("{} {}", value_str, currency)
}

fn extract_numeric_conflict(fact: &str, payload: &[u8]) -> Option<serde_json::Value> {
    let text = String::from_utf8_lossy(payload);
    let mut metric = "metric".to_owned();
    let mut currency = "KZT".to_owned();
    let mut value = "unknown".to_owned();
    for line in text.lines() {
        if let Some(val) = line.strip_prefix("metric=") {
            metric = val.trim().to_owned();
        } else if let Some(val) = line.strip_prefix("currency=") {
            currency = val.trim().to_owned();
        } else if let Some(val) = line.strip_prefix("value=") {
            value = val.trim().to_owned();
        }
    }

    let formatted_right = format_scale_currency(&value, &currency);
    let formatted_left = fact_numeric_value(fact, &currency)?;
    Some(serde_json::json!({
        "metric": metric,
        "left": formatted_left,
        "right": formatted_right,
    }))
}

fn fact_numeric_value(fact: &str, default_currency: &str) -> Option<String> {
    let words = fact.split_whitespace().collect::<Vec<_>>();
    for (i, word) in words.iter().enumerate() {
        let clean_word = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '.');
        if clean_word
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit())
        {
            if i + 1 < words.len() {
                let next_word = words[i + 1].trim_matches(|c: char| !c.is_alphabetic());
                if !next_word.is_empty() && next_word.len() <= 4 {
                    return Some(format_scale_currency(clean_word, next_word));
                }
            }
            return Some(format_scale_currency(clean_word, default_currency));
        }
    }
    None
}
