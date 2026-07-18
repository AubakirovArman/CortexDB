use super::AuthRole;
use crate::audit;
use crate::auth_capability::EffectiveAuthPolicy;
use crate::route_registry::{route_spec, RouteAccess};

pub(crate) fn role_can_access(role: AuthRole, method: &str, path: &str) -> bool {
    match role {
        AuthRole::Admin => true,
        AuthRole::Data => route_spec(method, path).access != RouteAccess::Admin,
    }
}

pub(super) fn capabilities_can_access(
    policy: &EffectiveAuthPolicy,
    method: &str,
    path: &str,
) -> bool {
    if route_spec(method, path).access == RouteAccess::Public {
        return true;
    }
    let Some(capabilities) = &policy.capabilities else {
        return true;
    };
    let action = audit::classify(method, path);
    capabilities
        .iter()
        .any(|capability| capability.allows(action))
}
