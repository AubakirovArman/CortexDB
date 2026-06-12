mod parser;
mod policy;
mod routes;
#[cfg(test)]
mod tests;
mod types;

pub use parser::parse_auth_tokens;
pub(crate) use policy::{authorize_request, tenant_can_access, validate_token_policies};
pub(crate) use types::AuthDecision;
pub use types::{AuthRole, AuthRouteContext, AuthTokenPolicy};

#[cfg(test)]
pub(super) use parser::{load_auth_policy_store, load_auth_tokens_file};
#[cfg(test)]
pub(super) use routes::role_can_access;
