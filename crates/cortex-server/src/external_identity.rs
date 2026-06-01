use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::auth::AuthRole;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ExternalIdentityConfig {
    pub issuer: String,
    pub audience: String,
    #[serde(default)]
    pub clock_skew_seconds: u64,
    pub mappings: Vec<ExternalIdentityMapping>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ExternalIdentityMapping {
    pub external_group: String,
    pub role: String,
    pub tenant: String,
    pub scopes: Vec<String>,
    pub agent_id: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ExternalIdentityClaims {
    pub issuer: String,
    pub audience: Vec<String>,
    pub subject: String,
    #[serde(default)]
    pub groups: Vec<String>,
    pub expires_at: u64,
    #[serde(default)]
    pub not_before: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalIdentityDecision {
    pub principal_id: String,
    pub role: AuthRole,
    pub tenant: String,
    pub scopes: Vec<String>,
    pub agent_id: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternalIdentityFailure {
    InvalidConfig,
    InvalidIssuer,
    InvalidAudience,
    ExpiredToken,
    TokenNotYetValid,
    MissingMapping,
    InvalidMapping,
}

pub fn verify_oidc_claims(
    config: &ExternalIdentityConfig,
    claims: &ExternalIdentityClaims,
    now_unix_seconds: u64,
) -> Result<ExternalIdentityDecision, ExternalIdentityFailure> {
    validate_external_identity_config(config)?;
    if claims.issuer != config.issuer {
        return Err(ExternalIdentityFailure::InvalidIssuer);
    }
    if !claims
        .audience
        .iter()
        .any(|audience| audience == &config.audience)
    {
        return Err(ExternalIdentityFailure::InvalidAudience);
    }
    let skew = config.clock_skew_seconds;
    if claims.expires_at.saturating_add(skew) <= now_unix_seconds {
        return Err(ExternalIdentityFailure::ExpiredToken);
    }
    if claims
        .not_before
        .is_some_and(|not_before| not_before > now_unix_seconds.saturating_add(skew))
    {
        return Err(ExternalIdentityFailure::TokenNotYetValid);
    }

    let Some(mapping) = config.mappings.iter().find(|mapping| {
        claims
            .groups
            .iter()
            .any(|group| group == &mapping.external_group)
    }) else {
        return Err(ExternalIdentityFailure::MissingMapping);
    };
    decision_from_mapping(&claims.subject, mapping)
}

pub fn validate_external_identity_config(
    config: &ExternalIdentityConfig,
) -> Result<(), ExternalIdentityFailure> {
    if config.issuer.trim().is_empty()
        || config.audience.trim().is_empty()
        || config.mappings.is_empty()
    {
        return Err(ExternalIdentityFailure::InvalidConfig);
    }

    let mut seen_groups = BTreeSet::new();
    for mapping in &config.mappings {
        let external_group = mapping.external_group.trim();
        if external_group.is_empty() || !seen_groups.insert(external_group.to_owned()) {
            return Err(ExternalIdentityFailure::InvalidConfig);
        }
        decision_from_mapping("config-validation", mapping)?;
    }
    Ok(())
}

fn decision_from_mapping(
    subject: &str,
    mapping: &ExternalIdentityMapping,
) -> Result<ExternalIdentityDecision, ExternalIdentityFailure> {
    if subject.trim().is_empty()
        || mapping.external_group.trim().is_empty()
        || mapping.tenant.trim().is_empty()
        || mapping.scopes.is_empty()
        || mapping.scopes.iter().any(|scope| scope.trim().is_empty())
        || matches!(mapping.agent_id, Some(0))
    {
        return Err(ExternalIdentityFailure::InvalidMapping);
    }
    let role = match mapping.role.trim().to_ascii_lowercase().as_str() {
        "admin" => AuthRole::Admin,
        "data" => AuthRole::Data,
        _ => return Err(ExternalIdentityFailure::InvalidMapping),
    };
    Ok(ExternalIdentityDecision {
        principal_id: format!("oidc:{subject}"),
        role,
        tenant: mapping.tenant.clone(),
        scopes: mapping.scopes.clone(),
        agent_id: mapping.agent_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> ExternalIdentityConfig {
        ExternalIdentityConfig {
            issuer: "https://idp.example.test/".to_owned(),
            audience: "cortexdb-api".to_owned(),
            clock_skew_seconds: 30,
            mappings: vec![ExternalIdentityMapping {
                external_group: "finance-analysts".to_owned(),
                role: "data".to_owned(),
                tenant: "default".to_owned(),
                scopes: vec!["project:investments".to_owned()],
                agent_id: Some(11),
            }],
        }
    }

    fn claims() -> ExternalIdentityClaims {
        ExternalIdentityClaims {
            issuer: "https://idp.example.test/".to_owned(),
            audience: vec!["cortexdb-api".to_owned()],
            subject: "user-123".to_owned(),
            groups: vec!["finance-analysts".to_owned()],
            expires_at: 1_700_000_300,
            not_before: Some(1_699_999_900),
        }
    }

    #[test]
    fn maps_valid_oidc_claims_to_explicit_policy_decision() {
        let decision = verify_oidc_claims(&config(), &claims(), 1_700_000_000).unwrap();
        assert_eq!(decision.principal_id, "oidc:user-123");
        assert_eq!(decision.role, AuthRole::Data);
        assert_eq!(decision.tenant, "default");
        assert_eq!(decision.scopes, vec!["project:investments"]);
        assert_eq!(decision.agent_id, Some(11));
    }

    #[test]
    fn rejects_invalid_issuer_audience_and_time_claims() {
        let base_config = config();
        let mut token = claims();
        token.issuer = "https://evil.example.test/".to_owned();
        assert_eq!(
            verify_oidc_claims(&base_config, &token, 1_700_000_000),
            Err(ExternalIdentityFailure::InvalidIssuer)
        );

        let mut token = claims();
        token.audience = vec!["other-api".to_owned()];
        assert_eq!(
            verify_oidc_claims(&base_config, &token, 1_700_000_000),
            Err(ExternalIdentityFailure::InvalidAudience)
        );

        let mut token = claims();
        token.expires_at = 1_699_999_900;
        assert_eq!(
            verify_oidc_claims(&base_config, &token, 1_700_000_000),
            Err(ExternalIdentityFailure::ExpiredToken)
        );

        let mut token = claims();
        token.not_before = Some(1_700_001_000);
        assert_eq!(
            verify_oidc_claims(&base_config, &token, 1_700_000_000),
            Err(ExternalIdentityFailure::TokenNotYetValid)
        );
    }

    #[test]
    fn fails_closed_when_group_mapping_is_missing() {
        let mut token = claims();
        token.groups = vec!["unmapped-provider-group".to_owned()];
        assert_eq!(
            verify_oidc_claims(&config(), &token, 1_700_000_000),
            Err(ExternalIdentityFailure::MissingMapping)
        );
    }

    #[test]
    fn rejects_invalid_mapping_instead_of_trusting_provider_group_as_scope() {
        let mut config = config();
        config.mappings[0].scopes.clear();
        assert_eq!(
            verify_oidc_claims(&config, &claims(), 1_700_000_000),
            Err(ExternalIdentityFailure::InvalidMapping)
        );
    }

    #[test]
    fn rejects_invalid_external_identity_config_before_mapping_claims() {
        let mut invalid_config = config();
        invalid_config.issuer.clear();
        assert_eq!(
            verify_oidc_claims(&invalid_config, &claims(), 1_700_000_000),
            Err(ExternalIdentityFailure::InvalidConfig)
        );

        let mut invalid_config = config();
        invalid_config
            .mappings
            .push(invalid_config.mappings[0].clone());
        assert_eq!(
            verify_oidc_claims(&invalid_config, &claims(), 1_700_000_000),
            Err(ExternalIdentityFailure::InvalidConfig)
        );
    }
}
