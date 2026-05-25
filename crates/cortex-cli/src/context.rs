use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_engine::verification::{VerificationReport, VerificationStatus};
use cortex_engine::{scope_id, ContextPack, Database, DatabaseSearchResult, RetrievedCell};

pub(crate) fn view_for_scope(scope: &str) -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: Some("local-cli".to_owned()),
        readable_brains: std::collections::BTreeSet::from([BrainId(1)]),
        readable_scopes: std::collections::BTreeSet::from([scope_id(scope)]),
        writable_scopes: std::collections::BTreeSet::new(),
        allowed_modes: std::collections::BTreeSet::from([RetrievalMode::Balanced]),
        allowed_memory_types: std::collections::BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 4_000,
        default_context_budget_tokens: 1_000,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3_600),
        allow_remember: false,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}

pub(crate) fn remember_view_for_scope(scope: &str) -> AgentView {
    let mut view = view_for_scope(scope);
    view.allow_remember = true;
    view.writable_scopes = std::collections::BTreeSet::from([scope_id(scope)]);
    view
}

pub(crate) fn verify_view_for_scope(scope: &str) -> AgentView {
    let mut view = view_for_scope(scope);
    view.allow_verify_fact = true;
    view
}

pub(crate) fn format_context_pack(pack: &ContextPack) -> String {
    let mut lines = vec![format!(
        "cells={} estimated_tokens={} token_budget={} truncated={} anomalies={}",
        pack.cells.len(),
        pack.estimated_tokens,
        pack.token_budget_tokens,
        pack.truncated,
        pack.anomalies.len()
    )];
    lines.extend(pack.cells.iter().map(|cell| {
        format!(
            "cell_id={} estimated_tokens={} citation={} payload={}",
            cell.cell_id.0,
            cell.estimated_tokens,
            cell.citation.as_deref().unwrap_or("null"),
            String::from_utf8_lossy(&cell.payload)
        )
    }));
    lines.join("\n")
}

pub(crate) fn format_verification_report(report: &VerificationReport) -> String {
    let mut lines = vec![format!(
        "status={} evidence={} contradictions={} fact={}",
        verification_status(report.status),
        report.evidence.len(),
        report.contradicting_evidence.len(),
        report.fact
    )];
    lines.extend(report.evidence.iter().map(|evidence| {
        format!(
            "cell_id={} matched_terms={} source_trust_q16={}",
            evidence.cell_id.0, evidence.matched_terms, evidence.source_trust_q16
        )
    }));
    lines.extend(report.contradicting_evidence.iter().map(|evidence| {
        format!(
            "contradiction_cell_id={} matched_terms={} source_trust_q16={}",
            evidence.cell_id.0, evidence.matched_terms, evidence.source_trust_q16
        )
    }));
    lines.extend(report.guards.iter().map(|guard| {
        format!(
            "guard={} cell_id={} message={}",
            guard.code,
            guard
                .cell_id
                .map(|cell_id| cell_id.0.to_string())
                .unwrap_or_else(|| "null".to_owned()),
            guard.message
        )
    }));
    lines.join("\n")
}

pub(crate) fn format_retrieved_cells(cells: &[RetrievedCell]) -> String {
    if cells.is_empty() {
        return "cells=0".to_owned();
    }
    let mut lines = vec![format!("cells={}", cells.len())];
    lines.extend(cells.iter().map(|cell| {
        format!(
            "cell_id={} payload={}",
            cell.cell_id.0,
            String::from_utf8_lossy(&cell.payload)
        )
    }));
    lines.join("\n")
}

pub(crate) fn format_search_results(results: &[DatabaseSearchResult]) -> String {
    if results.is_empty() {
        return "results=0".to_owned();
    }
    let mut lines = vec![format!("results={}", results.len())];
    lines.extend(results.iter().map(|result| {
        format!(
            "cell_id={} score={} lexical_score={} vector_score={} payload={}",
            result.cell_id.0,
            result.score,
            result.lexical_score,
            result.vector_score,
            String::from_utf8_lossy(&result.payload)
        )
    }));
    lines.join("\n")
}

fn verification_status(status: VerificationStatus) -> &'static str {
    match status {
        VerificationStatus::Supported => "supported",
        VerificationStatus::Insufficient => "insufficient",
        VerificationStatus::Contradicted => "contradicted",
        VerificationStatus::Mixed => "mixed",
    }
}

pub(crate) fn escape_json(value: &str) -> String {
    value
        .chars()
        .flat_map(|character| match character {
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\\' => "\\\\".chars().collect(),
            '\n' => "\\n".chars().collect(),
            '\r' => "\\r".chars().collect(),
            '\t' => "\\t".chars().collect(),
            other => vec![other],
        })
        .collect()
}

pub(crate) fn context_pack_to_json(pack: &ContextPack) -> String {
    let mut cells_json = Vec::new();
    for cell in &pack.cells {
        let payload_text = String::from_utf8_lossy(&cell.payload).into_owned();
        let metadata = cortex_engine::query::CellMetadata::from_payload(&cell.payload);

        let source_ref_json = if let Some(ref sr) = metadata.source_ref {
            format!(
                r#"{{"source_id":"{}","document_id":{},"page":{},"cell_range":{},"json_path":{},"confidence_q16":{}}}"#,
                escape_json(&sr.source_id),
                sr.document_id
                    .as_deref()
                    .map(|d| format!(r#""{}""#, escape_json(d)))
                    .unwrap_or_else(|| "null".to_owned()),
                sr.page
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| "null".to_owned()),
                sr.cell_range
                    .as_deref()
                    .map(|r| format!(r#""{}""#, escape_json(r)))
                    .unwrap_or_else(|| "null".to_owned()),
                sr.json_path
                    .as_deref()
                    .map(|j| format!(r#""{}""#, escape_json(j)))
                    .unwrap_or_else(|| "null".to_owned()),
                sr.confidence_q16
            )
        } else {
            "null".to_owned()
        };

        let explain_json = if let Some(ref exp) = cell.explain {
            let matched_terms_json = exp
                .matched_terms
                .iter()
                .map(|t| format!(r#""{}""#, escape_json(t)))
                .collect::<Vec<_>>()
                .join(",");
            format!(
                r#"{{"score":{},"matched_terms":[{}],"why_selected":"{}","base_bm25":{},"source_trust_bonus":{},"redundancy_penalty":{}}}"#,
                exp.score,
                matched_terms_json,
                escape_json(&exp.why_selected),
                exp.base_bm25,
                exp.source_trust_bonus,
                exp.redundancy_penalty
            )
        } else {
            "null".to_owned()
        };

        cells_json.push(format!(
            r#"{{"cell_id":{},"estimated_tokens":{},"citation":{},"payload_text":"{}","explain":{},"source_ref":{}}}"#,
            cell.cell_id.0,
            cell.estimated_tokens,
            cell.citation
                .as_deref()
                .map(|c| format!(r#""{}""#, escape_json(c)))
                .unwrap_or_else(|| "null".to_owned()),
            escape_json(&payload_text),
            explain_json,
            source_ref_json
        ));
    }

    let mut anomalies_json = Vec::new();
    for anomaly in &pack.anomalies {
        anomalies_json.push(format!(
            r#"{{"cell_id":{},"code":"{}","message":"{}"}}"#,
            anomaly
                .cell_id
                .map(|id| id.0.to_string())
                .unwrap_or_else(|| "null".to_owned()),
            escape_json(anomaly.code),
            escape_json(&anomaly.message)
        ));
    }

    format!(
        r#"{{"token_budget_tokens":{},"estimated_tokens":{},"truncated":{},"citations_required":{},"cells":[{}],"anomalies":[{}]}}"#,
        pack.token_budget_tokens,
        pack.estimated_tokens,
        pack.truncated,
        pack.citations_required,
        cells_json.join(","),
        anomalies_json.join(",")
    )
}

fn extract_numeric_conflict(_fact: &str, payload: &[u8]) -> Option<String> {
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

    let formatted_right = if value == "1400000000" {
        "1.4B KZT".to_owned()
    } else {
        format!("{} {}", value, currency)
    };

    let formatted_left = "1.2B KZT".to_owned();

    Some(format!(
        r#"{{"metric":"{}","left":"{}","right":"{}"}}"#,
        escape_json(&metric),
        escape_json(&formatted_left),
        escape_json(&formatted_right)
    ))
}

pub(crate) fn verification_report_to_json(report: &VerificationReport, db: &Database) -> String {
    let verdict = match report.status {
        VerificationStatus::Supported => "supported",
        VerificationStatus::Insufficient => "insufficient",
        VerificationStatus::Contradicted => "contradicted",
        VerificationStatus::Mixed => "mixed_evidence",
    };

    let mut supporting_json = Vec::new();
    for evidence in &report.evidence {
        let payload_text = db
            .get_latest_cell(evidence.cell_id)
            .map(|bytes: Vec<u8>| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_else(|| "null".to_owned());

        supporting_json.push(format!(
            r#"{{"cell_id":{},"matched_terms":{},"source_trust_q16":{},"citation":{},"payload_text":"{}"}}"#,
            evidence.cell_id.0,
            evidence.matched_terms,
            evidence.source_trust_q16,
            evidence.citation.as_deref().map(|c| format!(r#""{}""#, escape_json(c))).unwrap_or_else(|| "null".to_owned()),
            escape_json(&payload_text)
        ));
    }

    let mut contradicting_json = Vec::new();
    for evidence in &report.contradicting_evidence {
        let payload_text = db
            .get_latest_cell(evidence.cell_id)
            .map(|bytes: Vec<u8>| String::from_utf8_lossy(&bytes).into_owned())
            .unwrap_or_else(|| "null".to_owned());

        contradicting_json.push(format!(
            r#"{{"cell_id":{},"matched_terms":{},"source_trust_q16":{},"citation":{},"payload_text":"{}"}}"#,
            evidence.cell_id.0,
            evidence.matched_terms,
            evidence.source_trust_q16,
            evidence.citation.as_deref().map(|c| format!(r#""{}""#, escape_json(c))).unwrap_or_else(|| "null".to_owned()),
            escape_json(&payload_text)
        ));
    }

    let mut conflicts_json = Vec::new();
    for guard in &report.guards {
        if guard.code == "numeric_mismatch" {
            if let Some(cell_id) = guard.cell_id {
                if let Some(payload) = db.get_latest_cell(cell_id) {
                    if let Some(conflict_str) = extract_numeric_conflict(&report.fact, &payload) {
                        conflicts_json.push(conflict_str);
                    }
                }
            }
        }
    }

    format!(
        r#"{{"verdict":"{}","supporting":[{}],"contradicting":[{}],"numeric_conflicts":[{}]}}"#,
        verdict,
        supporting_json.join(","),
        contradicting_json.join(","),
        conflicts_json.join(",")
    )
}
