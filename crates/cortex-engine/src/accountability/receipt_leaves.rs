use std::collections::BTreeMap;

use cortex_core::CellId;
use serde_json::{json, Value};

use super::receipt::{
    hash_bytes, hash_value, ACCOUNTABILITY_RECEIPT_ACCESS_ROOT_DOMAIN,
    ACCOUNTABILITY_RECEIPT_VERIFICATION_ROOT_DOMAIN,
};
use crate::canonical::canonical_verification_report_bytes;
use crate::context::{ContextAccessDecision, ContextAccessDecisionOutcome, ContextPack};
use crate::database::CapturedAccessDenialSet;
use crate::error::{EngineError, EngineResult};
use crate::verification::{VerificationEvidence, VerificationNumericConflict, VerificationReport};

pub(super) fn access_leaves(
    pack: &ContextPack,
    captured_access_denials: &CapturedAccessDenialSet,
) -> EngineResult<Vec<Value>> {
    let mut leaves = Vec::new();
    for cell in &pack.cells {
        let decision = cell
            .access_decision
            .as_ref()
            .ok_or(EngineError::InvalidOperation)?;
        leaves.push(admitted_access_leaf(decision)?);
    }
    for denial in &captured_access_denials.denials {
        leaves.push(json!({
            "leaf_type": "denied_candidate",
            "cell_id": Value::Null,
            "candidate": denial.candidate,
            "cell_id_hash": denial.cell_id_hash,
            "decision": "denied",
            "policy": denial.policy,
            "policy_version": denial.policy_version,
            "reason": denial.reason,
            "scope_id": Value::Null,
            "agent_id": denial.agent_id,
            "agent_view_digest": denial.agent_view_digest,
            "evidence_digest": denial.evidence_digest,
        }));
    }
    Ok(leaves)
}

pub(super) fn provenance_leaves(
    pack: &ContextPack,
    cell_hashes: &BTreeMap<CellId, String>,
) -> Vec<Value> {
    pack.cells
        .iter()
        .map(|cell| {
            let provenance = cell.provenance.as_ref();
            json!({
                "cell_id": cell.cell_id.0,
                "cell_content_hash": cell_hashes.get(&cell.cell_id),
                "source_cell_id": provenance.map(|item| item.source_cell_id.0),
                "source_byte_start": provenance.map(|item| item.source_byte_start),
                "source_byte_end": provenance.map(|item| item.source_byte_end),
                "source_line_start": provenance.map(|item| item.source_line_start),
                "source_line_end": provenance.map(|item| item.source_line_end),
                "source_ref": provenance
                    .and_then(|item| item.source_ref.as_ref())
                    .map(|source_ref| source_ref.source_id.as_str()),
                "citation": cell.citation,
            })
        })
        .collect()
}

pub(super) fn cell_set_leaves(
    pack: &ContextPack,
    cell_hashes: &BTreeMap<CellId, String>,
) -> Vec<Value> {
    pack.cells
        .iter()
        .map(|cell| {
            json!({
                "cell_id": cell.cell_id.0,
                "cell_content_hash": cell_hashes.get(&cell.cell_id),
            })
        })
        .collect()
}

pub(super) fn verification_leaves(report: Option<&VerificationReport>) -> Vec<Value> {
    let Some(report) = report else {
        return Vec::new();
    };
    let mut leaves = Vec::new();
    leaves.push(json!({
        "leaf_type": "verification_report",
        "cell_id": Value::Null,
        "status": report.status.as_str(),
        "confidence_q16": report.confidence_q16,
        "evidence_digest": hash_bytes(
            ACCOUNTABILITY_RECEIPT_VERIFICATION_ROOT_DOMAIN,
            &canonical_verification_report_bytes(report),
        ),
    }));
    for evidence in &report.evidence {
        leaves.push(verification_evidence_leaf(
            "verification_evidence",
            report.status.as_str(),
            evidence,
        ));
    }
    for evidence in &report.contradicting_evidence {
        leaves.push(verification_evidence_leaf(
            "verification_evidence",
            report.status.as_str(),
            evidence,
        ));
    }
    for conflict in &report.numeric_conflicts {
        leaves.push(verification_conflict_leaf(report, conflict));
    }
    leaves
}

pub(super) fn budget_leaves(pack: &ContextPack) -> Vec<Value> {
    let mut leaves = vec![json!({
        "token_budget_tokens": pack.token_budget_tokens,
        "estimated_tokens": pack.estimated_tokens,
        "truncated": pack.truncated,
        "cell_id": Value::Null,
        "cell_estimated_tokens": Value::Null,
    })];
    leaves.extend(pack.cells.iter().map(|cell| {
        json!({
            "token_budget_tokens": pack.token_budget_tokens,
            "estimated_tokens": pack.estimated_tokens,
            "truncated": pack.truncated,
            "cell_id": cell.cell_id.0,
            "cell_estimated_tokens": cell.estimated_tokens,
        })
    }));
    leaves
}

pub(super) fn conflict_leaves(pack: &ContextPack) -> Vec<Value> {
    let mut anomalies = pack
        .anomalies
        .iter()
        .map(|anomaly| anomaly.code.as_str())
        .collect::<Vec<_>>();
    anomalies.sort();
    vec![json!({
        "conflict_visibility_q16": pack.conflict_visibility_q16,
        "visible_conflict_count": pack.visible_conflict_count,
        "anomalies": anomalies,
    })]
}

fn admitted_access_leaf(decision: &ContextAccessDecision) -> EngineResult<Value> {
    if decision.decision != ContextAccessDecisionOutcome::Allowed {
        return Err(EngineError::InvalidOperation);
    }
    let policy_version = decision
        .policy_version
        .as_ref()
        .ok_or(EngineError::InvalidOperation)?;
    let agent_view_digest = decision
        .agent_view_digest
        .as_ref()
        .ok_or(EngineError::InvalidOperation)?;
    let evidence = json!({
        "cell_id": decision.cell_id.0,
        "decision": decision.decision.as_str(),
        "policy": decision.policy,
        "policy_version": policy_version,
        "reason": decision.reason,
        "scope_id": decision.scope_id,
        "agent_id": decision.agent_id,
        "agent_view_digest": agent_view_digest,
    });

    Ok(json!({
        "leaf_type": "admitted_cell",
        "cell_id": decision.cell_id.0,
        "candidate": Value::Null,
        "cell_id_hash": Value::Null,
        "decision": "allowed",
        "policy": decision.policy,
        "policy_version": policy_version,
        "reason": decision.reason,
        "scope_id": decision.scope_id,
        "agent_id": decision.agent_id,
        "agent_view_digest": agent_view_digest,
        "evidence_digest": hash_value(
            ACCOUNTABILITY_RECEIPT_ACCESS_ROOT_DOMAIN,
            &evidence,
        ),
    }))
}

fn verification_evidence_leaf(
    leaf_type: &'static str,
    status: &str,
    evidence: &VerificationEvidence,
) -> Value {
    let evidence_value = json!({
        "cell_id": evidence.cell_id.0,
        "matched_terms": evidence.matched_terms,
        "match_score_q16": evidence.match_score_q16,
        "match_kind": evidence.match_kind.as_str(),
        "source_trust_q16": evidence.source_trust_q16,
        "source_trust_category": evidence.source_trust_category.as_str(),
        "citation": evidence.citation,
    });
    json!({
        "leaf_type": leaf_type,
        "cell_id": evidence.cell_id.0,
        "status": status,
        "confidence_q16": evidence.match_score_q16,
        "evidence_digest": hash_value(
            ACCOUNTABILITY_RECEIPT_VERIFICATION_ROOT_DOMAIN,
            &evidence_value,
        ),
    })
}

fn verification_conflict_leaf(
    report: &VerificationReport,
    conflict: &VerificationNumericConflict,
) -> Value {
    let conflict_value = json!({
        "cell_id": conflict.cell_id.0,
        "kind": conflict.kind.as_str(),
        "metric": conflict.metric,
        "left": conflict.left,
        "right": conflict.right,
        "fact_value": conflict.fact_value.raw,
        "evidence_value": conflict.evidence_value.raw,
    });
    json!({
        "leaf_type": "verification_conflict",
        "cell_id": conflict.cell_id.0,
        "status": report.status.as_str(),
        "confidence_q16": report.confidence_q16,
        "evidence_digest": hash_value(
            ACCOUNTABILITY_RECEIPT_VERIFICATION_ROOT_DOMAIN,
            &conflict_value,
        ),
    })
}
