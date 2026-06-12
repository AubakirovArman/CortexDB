mod api;
mod io;
mod mutations;
#[cfg(test)]
mod tests;
mod types;
mod validation;

pub(crate) use api::{handle_admin_request, load_token_policies_from_store};
pub(crate) use io::decode_store_str;
pub(crate) use types::{AuthPolicyPrincipal, AuthPolicyStoreFile};
pub(crate) use validation::{parse_capabilities, parse_tenants};

const SCHEMA_VERSION: &str = "cortexdb.auth_policy.v1";
const LEGACY_SCHEMA_VERSION_V0: &str = "cortexdb.auth_policy.v0";
