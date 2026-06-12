use super::AuthRole;
use crate::audit::{self, AuditAction};
use crate::auth_capability::EffectiveAuthPolicy;
use crate::dashboard;

pub(crate) fn role_can_access(role: AuthRole, method: &str, path: &str) -> bool {
    match role {
        AuthRole::Admin => true,
        AuthRole::Data => !matches!(route_class(method, path), RouteClass::Admin),
    }
}

pub(super) fn capabilities_can_access(
    policy: &EffectiveAuthPolicy,
    method: &str,
    path: &str,
) -> bool {
    if matches!(route_class(method, path), RouteClass::Public) {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RouteClass {
    Public,
    Data,
    Admin,
}

fn route_class(method: &str, path: &str) -> RouteClass {
    if dashboard::is_page(path) || path.starts_with("/dashboard/assets/") {
        return RouteClass::Admin;
    }
    match audit::classify(method, path) {
        AuditAction::Health => RouteClass::Public,
        AuditAction::Admin | AuditAction::Metrics => RouteClass::Admin,
        _ => RouteClass::Data,
    }
}
