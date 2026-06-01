use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct LegalVerificationPolicy {
    pub domain: String,
    pub jurisdiction: String,
    pub require_source_refs: bool,
    pub require_reviewer_approval: bool,
    pub allow_legal_advice_output: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct LegalVerificationRequest {
    pub domain: String,
    pub jurisdiction: String,
    pub claim: String,
    pub source_refs: Vec<String>,
    pub reviewer_id: Option<String>,
    pub reviewer_approved: bool,
    pub output_boundary: LegalOutputBoundary,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum LegalOutputBoundary {
    EvidenceSummaryNotLegalAdvice,
    LegalAdvice,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum LegalRefusalReason {
    OutOfDomain,
    EmptyClaim,
    MissingSourceRefs,
    MissingReviewerApproval,
    LegalAdviceOutputNotAllowed,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct LegalVerificationReview {
    pub legal_grade_ready: bool,
    pub accepted_for_reviewed_output: bool,
    pub refusal_reasons: Vec<LegalRefusalReason>,
    pub output_boundary: LegalOutputBoundary,
}

pub fn evaluate_legal_verification_boundary(
    policy: &LegalVerificationPolicy,
    request: &LegalVerificationRequest,
) -> LegalVerificationReview {
    let mut refusal_reasons = Vec::new();
    if request.domain != policy.domain || request.jurisdiction != policy.jurisdiction {
        refusal_reasons.push(LegalRefusalReason::OutOfDomain);
    }
    if request.claim.trim().is_empty() {
        refusal_reasons.push(LegalRefusalReason::EmptyClaim);
    }
    if policy.require_source_refs
        && (request.source_refs.is_empty()
            || request
                .source_refs
                .iter()
                .any(|source| source.trim().is_empty()))
    {
        refusal_reasons.push(LegalRefusalReason::MissingSourceRefs);
    }
    if policy.require_reviewer_approval
        && (!request.reviewer_approved
            || request
                .reviewer_id
                .as_deref()
                .is_none_or(|reviewer| reviewer.trim().is_empty()))
    {
        refusal_reasons.push(LegalRefusalReason::MissingReviewerApproval);
    }
    if request.output_boundary == LegalOutputBoundary::LegalAdvice
        && !policy.allow_legal_advice_output
    {
        refusal_reasons.push(LegalRefusalReason::LegalAdviceOutputNotAllowed);
    }
    LegalVerificationReview {
        legal_grade_ready: false,
        accepted_for_reviewed_output: refusal_reasons.is_empty(),
        refusal_reasons,
        output_boundary: request.output_boundary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> LegalVerificationPolicy {
        LegalVerificationPolicy {
            domain: "kazakhstan_investment_contract_disclosure".to_owned(),
            jurisdiction: "Kazakhstan".to_owned(),
            require_source_refs: true,
            require_reviewer_approval: true,
            allow_legal_advice_output: false,
        }
    }

    fn request() -> LegalVerificationRequest {
        LegalVerificationRequest {
            domain: "kazakhstan_investment_contract_disclosure".to_owned(),
            jurisdiction: "Kazakhstan".to_owned(),
            claim: "The contract contains a budget amendment clause.".to_owned(),
            source_refs: vec!["law://fixture/project-contract#section=budget-amendment".to_owned()],
            reviewer_id: Some("reviewer-1".to_owned()),
            reviewer_approved: true,
            output_boundary: LegalOutputBoundary::EvidenceSummaryNotLegalAdvice,
        }
    }

    #[test]
    fn accepts_citation_complete_reviewed_evidence_summary() {
        let review = evaluate_legal_verification_boundary(&policy(), &request());
        assert!(!review.legal_grade_ready);
        assert!(review.accepted_for_reviewed_output);
        assert!(review.refusal_reasons.is_empty());
    }

    #[test]
    fn refuses_out_of_domain_or_empty_claims() {
        let mut request = request();
        request.domain = "generic".to_owned();
        request.claim = " ".to_owned();
        let review = evaluate_legal_verification_boundary(&policy(), &request);
        assert_eq!(
            review.refusal_reasons,
            vec![
                LegalRefusalReason::OutOfDomain,
                LegalRefusalReason::EmptyClaim
            ]
        );
    }

    #[test]
    fn refuses_missing_source_refs_and_reviewer_approval() {
        let mut request = request();
        request.source_refs.clear();
        request.reviewer_id = None;
        request.reviewer_approved = false;
        let review = evaluate_legal_verification_boundary(&policy(), &request);
        assert_eq!(
            review.refusal_reasons,
            vec![
                LegalRefusalReason::MissingSourceRefs,
                LegalRefusalReason::MissingReviewerApproval
            ]
        );
    }

    #[test]
    fn refuses_legal_advice_output_when_policy_disallows_it() {
        let mut request = request();
        request.output_boundary = LegalOutputBoundary::LegalAdvice;
        let review = evaluate_legal_verification_boundary(&policy(), &request);
        assert_eq!(
            review.refusal_reasons,
            vec![LegalRefusalReason::LegalAdviceOutputNotAllowed]
        );
    }
}
