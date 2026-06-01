use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct OidcProviderConfig {
    pub issuer: String,
    pub audience: String,
    pub jwks_url: String,
    pub allowed_algorithms: Vec<String>,
    pub jwks_cache_ttl_seconds: u64,
    pub request_timeout_ms: u64,
    #[serde(default)]
    pub fail_open: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OidcProviderConfigFailure {
    MissingIssuer,
    MissingAudience,
    InvalidJwksUrl,
    EmptyAlgorithms,
    UnsupportedAlgorithm,
    DuplicateAlgorithm,
    InvalidCacheTtl,
    InvalidTimeout,
    FailOpenNotAllowed,
}

pub fn validate_oidc_provider_config(
    config: &OidcProviderConfig,
) -> Result<(), OidcProviderConfigFailure> {
    if config.issuer.trim().is_empty() {
        return Err(OidcProviderConfigFailure::MissingIssuer);
    }
    if config.audience.trim().is_empty() {
        return Err(OidcProviderConfigFailure::MissingAudience);
    }
    if !valid_https_url(&config.jwks_url) {
        return Err(OidcProviderConfigFailure::InvalidJwksUrl);
    }
    if config.allowed_algorithms.is_empty() {
        return Err(OidcProviderConfigFailure::EmptyAlgorithms);
    }
    let mut algorithms = BTreeSet::new();
    for algorithm in &config.allowed_algorithms {
        let normalized = algorithm.trim();
        if !matches!(normalized, "RS256" | "ES256" | "PS256") {
            return Err(OidcProviderConfigFailure::UnsupportedAlgorithm);
        }
        if !algorithms.insert(normalized.to_owned()) {
            return Err(OidcProviderConfigFailure::DuplicateAlgorithm);
        }
    }
    if config.jwks_cache_ttl_seconds == 0 || config.jwks_cache_ttl_seconds > 86_400 {
        return Err(OidcProviderConfigFailure::InvalidCacheTtl);
    }
    if config.request_timeout_ms == 0 || config.request_timeout_ms > 30_000 {
        return Err(OidcProviderConfigFailure::InvalidTimeout);
    }
    if config.fail_open {
        return Err(OidcProviderConfigFailure::FailOpenNotAllowed);
    }
    Ok(())
}

fn valid_https_url(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.starts_with("https://") && trimmed.len() > "https://".len() && !trimmed.contains(' ')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> OidcProviderConfig {
        OidcProviderConfig {
            issuer: "https://idp.example.test/".to_owned(),
            audience: "cortexdb-api".to_owned(),
            jwks_url: "https://idp.example.test/.well-known/jwks.json".to_owned(),
            allowed_algorithms: vec!["RS256".to_owned()],
            jwks_cache_ttl_seconds: 300,
            request_timeout_ms: 2_000,
            fail_open: false,
        }
    }

    #[test]
    fn oidc_provider_config_accepts_safe_https_jwks_policy() {
        assert_eq!(validate_oidc_provider_config(&config()), Ok(()));
    }

    #[test]
    fn oidc_provider_config_rejects_unsafe_jwks_and_fail_open_policy() {
        let mut invalid = config();
        invalid.jwks_url = "http://idp.example.test/jwks.json".to_owned();
        assert_eq!(
            validate_oidc_provider_config(&invalid),
            Err(OidcProviderConfigFailure::InvalidJwksUrl)
        );

        let mut invalid = config();
        invalid.fail_open = true;
        assert_eq!(
            validate_oidc_provider_config(&invalid),
            Err(OidcProviderConfigFailure::FailOpenNotAllowed)
        );
    }

    #[test]
    fn oidc_provider_config_rejects_weak_or_duplicate_algorithms() {
        let mut invalid = config();
        invalid.allowed_algorithms = vec!["none".to_owned()];
        assert_eq!(
            validate_oidc_provider_config(&invalid),
            Err(OidcProviderConfigFailure::UnsupportedAlgorithm)
        );

        let mut invalid = config();
        invalid.allowed_algorithms = vec!["RS256".to_owned(), "RS256".to_owned()];
        assert_eq!(
            validate_oidc_provider_config(&invalid),
            Err(OidcProviderConfigFailure::DuplicateAlgorithm)
        );
    }
}
