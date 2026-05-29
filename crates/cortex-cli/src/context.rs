use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_engine::verification::{VerificationReport, VerificationStatus};
use cortex_engine::{scope_id, ContextPack, DatabaseSearchResult, RetrievedCell};

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
            guard.code.as_str(),
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
