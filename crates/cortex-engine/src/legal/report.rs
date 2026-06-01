use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct LegalReportContract {
    pub report_id: String,
    pub domain: String,
    pub jurisdiction: String,
    pub evidence_summary: String,
    pub legal_advice_text: Option<String>,
    pub source_refs: Vec<String>,
    pub reviewer_id: Option<String>,
    pub reviewer_approved: bool,
    pub retention: LegalReportRetention,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct LegalReportRetention {
    pub retain_source_refs: bool,
    pub retain_reviewer_decision: bool,
    pub audit_required: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum LegalReportContractIssue {
    MissingReportId,
    MissingDomainOrJurisdiction,
    MissingEvidenceSummary,
    ContainsLegalAdvice,
    MissingSourceRefs,
    MissingReviewerApproval,
    MissingRetentionPolicy,
}

pub fn evaluate_legal_report_contract(
    report: &LegalReportContract,
) -> Vec<LegalReportContractIssue> {
    let mut issues = Vec::new();
    if report.report_id.trim().is_empty() {
        issues.push(LegalReportContractIssue::MissingReportId);
    }
    if report.domain.trim().is_empty() || report.jurisdiction.trim().is_empty() {
        issues.push(LegalReportContractIssue::MissingDomainOrJurisdiction);
    }
    if report.evidence_summary.trim().is_empty() {
        issues.push(LegalReportContractIssue::MissingEvidenceSummary);
    }
    if report
        .legal_advice_text
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        issues.push(LegalReportContractIssue::ContainsLegalAdvice);
    }
    if report.source_refs.is_empty()
        || report
            .source_refs
            .iter()
            .any(|value| value.trim().is_empty())
    {
        issues.push(LegalReportContractIssue::MissingSourceRefs);
    }
    if !report.reviewer_approved
        || report
            .reviewer_id
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
    {
        issues.push(LegalReportContractIssue::MissingReviewerApproval);
    }
    if !report.retention.retain_source_refs
        || !report.retention.retain_reviewer_decision
        || !report.retention.audit_required
    {
        issues.push(LegalReportContractIssue::MissingRetentionPolicy);
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report() -> LegalReportContract {
        LegalReportContract {
            report_id: "legal-report-001".to_owned(),
            domain: "kazakhstan_investment_contract_disclosure".to_owned(),
            jurisdiction: "Kazakhstan".to_owned(),
            evidence_summary: "The cited contract section contains a budget amendment clause."
                .to_owned(),
            legal_advice_text: None,
            source_refs: vec!["law://fixture/project-contract#section=budget-amendment".to_owned()],
            reviewer_id: Some("reviewer-1".to_owned()),
            reviewer_approved: true,
            retention: LegalReportRetention {
                retain_source_refs: true,
                retain_reviewer_decision: true,
                audit_required: true,
            },
        }
    }

    #[test]
    fn legal_report_contract_accepts_reviewed_evidence_summary() {
        assert!(evaluate_legal_report_contract(&report()).is_empty());
    }

    #[test]
    fn legal_report_contract_rejects_advice_and_missing_audit_trail() {
        let mut report = report();
        report.legal_advice_text = Some("You should sue the counterparty.".to_owned());
        report.source_refs.clear();
        report.reviewer_approved = false;
        report.retention.audit_required = false;

        assert_eq!(
            evaluate_legal_report_contract(&report),
            vec![
                LegalReportContractIssue::ContainsLegalAdvice,
                LegalReportContractIssue::MissingSourceRefs,
                LegalReportContractIssue::MissingReviewerApproval,
                LegalReportContractIssue::MissingRetentionPolicy,
            ]
        );
    }
}
