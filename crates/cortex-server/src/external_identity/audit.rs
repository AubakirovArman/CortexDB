use serde::Serialize;

use super::{ExternalIdentityDecision, ExternalIdentityFailure};

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExternalIdentityAuditOutcome {
    Allowed,
    Denied,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ExternalIdentityAuditRecord {
    pub schema_version: &'static str,
    pub audit_event: &'static str,
    pub provider: &'static str,
    pub outcome: ExternalIdentityAuditOutcome,
    pub principal_id: String,
    pub role: String,
    pub tenant: String,
    pub scopes: Vec<String>,
    pub agent_id: Option<u64>,
    pub failure: Option<&'static str>,
    pub token_logged: bool,
    pub claims_logged: bool,
}

pub fn external_identity_decision_audit_record(
    decision: &ExternalIdentityDecision,
) -> ExternalIdentityAuditRecord {
    ExternalIdentityAuditRecord {
        schema_version: "cortexdb.external_identity.audit.v1",
        audit_event: "external_identity_decision",
        provider: "oidc",
        outcome: ExternalIdentityAuditOutcome::Allowed,
        principal_id: decision.principal_id.clone(),
        role: decision.role.as_str().to_owned(),
        tenant: decision.tenant.clone(),
        scopes: decision.scopes.clone(),
        agent_id: decision.agent_id,
        failure: None,
        token_logged: false,
        claims_logged: false,
    }
}

pub fn external_identity_failure_audit_record(
    failure: ExternalIdentityFailure,
) -> ExternalIdentityAuditRecord {
    ExternalIdentityAuditRecord {
        schema_version: "cortexdb.external_identity.audit.v1",
        audit_event: "external_identity_decision",
        provider: "oidc",
        outcome: ExternalIdentityAuditOutcome::Denied,
        principal_id: String::new(),
        role: String::new(),
        tenant: String::new(),
        scopes: Vec::new(),
        agent_id: None,
        failure: Some(failure.as_str()),
        token_logged: false,
        claims_logged: false,
    }
}

impl ExternalIdentityFailure {
    fn as_str(self) -> &'static str {
        match self {
            Self::InvalidConfig => "invalid_config",
            Self::InvalidIssuer => "invalid_issuer",
            Self::InvalidAudience => "invalid_audience",
            Self::ExpiredToken => "expired_token",
            Self::TokenNotYetValid => "token_not_yet_valid",
            Self::MissingMapping => "missing_mapping",
            Self::InvalidMapping => "invalid_mapping",
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::auth::AuthRole;

    use super::*;

    #[test]
    fn external_identity_decision_audit_record_exposes_mapping_without_claims() {
        let record = external_identity_decision_audit_record(&ExternalIdentityDecision {
            principal_id: "oidc:user-123".to_owned(),
            role: AuthRole::Data,
            tenant: "default".to_owned(),
            scopes: vec!["project:investments".to_owned()],
            agent_id: Some(11),
        });

        let json = serde_json::to_string(&record).unwrap();
        assert_eq!(record.outcome, ExternalIdentityAuditOutcome::Allowed);
        assert_eq!(record.role, "data");
        assert_eq!(record.tenant, "default");
        assert_eq!(record.scopes, vec!["project:investments"]);
        assert!(!record.token_logged);
        assert!(!record.claims_logged);
        assert!(!json.contains("access-token"));
        assert!(!json.contains("finance-analysts"));
    }

    #[test]
    fn external_identity_failure_audit_record_fails_closed_without_identity_payload() {
        let record = external_identity_failure_audit_record(ExternalIdentityFailure::InvalidIssuer);

        assert_eq!(record.outcome, ExternalIdentityAuditOutcome::Denied);
        assert_eq!(record.failure, Some("invalid_issuer"));
        assert!(record.principal_id.is_empty());
        assert!(record.scopes.is_empty());
        assert!(!record.token_logged);
        assert!(!record.claims_logged);
    }
}
