use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_engine::{
    scope_id, ContextCellExplain, ContextPack, DatabaseSearchResult, RetrievedCell,
    VerificationReport, VerificationStatus,
};

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
    view.allowed_memory_types = std::collections::BTreeSet::from([
        MemoryType::Decision,
        MemoryType::Preference,
        MemoryType::WorkflowResult,
        MemoryType::ErrorLog,
        MemoryType::Observation,
    ]);
    view.max_ttl_seconds = Some(2_592_000);
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

pub(crate) fn format_context_cell_explain(explain: &ContextCellExplain) -> String {
    let mut lines = vec![format!(
        "cell_id={} outcome={} first_excluding_stage={} score={} why_selected={} why_excluded={}",
        explain.cell_id.0,
        explain.outcome.as_str(),
        explain.first_excluding_stage.as_deref().unwrap_or("null"),
        explain
            .score
            .map(|score| score.to_string())
            .unwrap_or_else(|| "null".to_owned()),
        explain.why_selected.as_deref().unwrap_or("null"),
        explain.why_excluded.as_deref().unwrap_or("null")
    )];
    if !explain.matched_terms.is_empty() {
        lines.push(format!("matched_terms={}", explain.matched_terms.join(",")));
    }
    lines.extend(explain.score_components.iter().map(|component| {
        format!(
            "score_component={} value={} contribution={} reason={}",
            component.name, component.value, component.contribution, component.reason
        )
    }));
    if let Some(decision) = &explain.access_decision {
        lines.push(format!(
            "access_decision={} policy={} scope={} scope_id={} reason={}",
            decision.decision.as_str(),
            decision.policy,
            decision.scope,
            decision.scope_id,
            decision.reason
        ));
    }
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
            "cell_id={} matched_terms={} match_kind={} match_score_q16={} source_trust_q16={} source_trust_category={}",
            evidence.cell_id.0,
            evidence.matched_terms,
            evidence.match_kind.as_str(),
            evidence.match_score_q16,
            evidence.source_trust_q16,
            evidence.source_trust_category.as_str()
        )
    }));
    lines.extend(report.contradicting_evidence.iter().map(|evidence| {
        format!(
            "contradiction_cell_id={} matched_terms={} match_kind={} match_score_q16={} source_trust_q16={} source_trust_category={}",
            evidence.cell_id.0,
            evidence.matched_terms,
            evidence.match_kind.as_str(),
            evidence.match_score_q16,
            evidence.source_trust_q16,
            evidence.source_trust_category.as_str()
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
