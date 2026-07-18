use crate::audit::AuditAction;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RouteAccess {
    Public,
    Data,
    Admin,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RouteTimeoutClass {
    Read,
    Write,
    Admin,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RouteSpec {
    pub(crate) access: RouteAccess,
    pub(crate) timeout: RouteTimeoutClass,
    pub(crate) mutating: bool,
    pub(crate) agent_scoped: bool,
}

pub(crate) fn route_spec(method: &str, target: &str) -> RouteSpec {
    let (path, _) = target.split_once('?').unwrap_or((target, ""));
    let mutating = is_mutating(method, path);
    RouteSpec {
        access: match crate::audit::classify(method, path) {
            AuditAction::Health => RouteAccess::Public,
            AuditAction::Admin | AuditAction::Metrics => RouteAccess::Admin,
            _ => RouteAccess::Data,
        },
        timeout: if is_operational_admin(method, path) {
            RouteTimeoutClass::Admin
        } else if mutating {
            RouteTimeoutClass::Write
        } else {
            RouteTimeoutClass::Read
        },
        mutating,
        agent_scoped: is_agent_scoped(method, path),
    }
}

fn is_mutating(method: &str, path: &str) -> bool {
    matches!(
        (method, path),
        ("POST", "/put")
            | ("POST", "/v1/cell")
            | ("POST", "/v1/batch")
            | ("POST", "/v1/transactions")
            | ("POST", "/v1/handoff")
            | ("POST", "/v1/memory/consolidate/commit")
            | ("POST", "/tombstone")
            | ("DELETE", "/v1/cell")
            | ("POST", "/flush")
            | ("POST", "/v1/flush")
            | ("POST", "/v1/compact")
            | ("POST", "/v1/admin/compact/trigger")
            | ("PUT", "/v1/admin/search/hnsw/no-fallback-profile")
            | ("DELETE", "/v1/admin/search/hnsw/no-fallback-profile")
            | ("POST", "/v1/remember")
            | ("POST", "/v1/feedback")
            | ("POST", "/v1/forget")
            | ("POST", "/v1/ingest/text")
            | ("POST", "/v1/ingest/json")
            | ("POST", "/v1/ingest/csv")
            | ("POST", "/v1/embedding/backfill")
    ) || (method == "DELETE" && path.starts_with("/v1/ingest/jobs/"))
        || (method == "POST"
            && path.starts_with("/v1/ingest/jobs/")
            && (path.ends_with("/cancel") || path.ends_with("/retry")))
}

fn is_operational_admin(method: &str, path: &str) -> bool {
    matches!(
        (method, path),
        ("GET", "/v1/health")
            | ("GET", "/v1/stats")
            | ("GET", "/v1/metrics")
            | ("GET", "/v1/ann/metrics")
            | ("GET", "/v1/validate")
            | ("GET", "/v1/cluster/status")
            | ("POST", "/v1/backup")
            | ("POST", "/v1/backup/verify")
            | ("POST", "/v1/backup/drill")
            | ("POST", "/v1/backup/offsite/stage")
            | ("POST", "/v1/backup/prune")
    )
}

fn is_agent_scoped(method: &str, path: &str) -> bool {
    matches!(
        (method, path),
        ("GET", "/get")
            | ("GET", "/v1/cell")
            | ("POST", "/put")
            | ("POST", "/v1/cell")
            | ("POST", "/v1/batch")
            | ("POST", "/tombstone")
            | ("DELETE", "/v1/cell")
            | ("POST", "/v1/context")
            | ("POST", "/v1/context/trace")
            | ("POST", "/v1/transactions")
            | ("POST", "/v1/handoff")
            | ("POST", "/v1/memory/consolidate/plan")
            | ("POST", "/v1/memory/consolidate/commit")
            | ("POST", "/v1/aql")
            | ("POST", "/v1/search")
            | ("POST", "/v1/search/explain")
            | ("POST", "/v1/search/ann-evaluate")
            | ("POST", "/v1/remember")
            | ("POST", "/v1/forget")
            | ("POST", "/v1/verify")
            | ("POST", "/v1/feedback")
            | ("GET", "/v1/feedback/stats")
            | ("GET", "/v1/conflicts")
            | ("POST", "/v1/ingest/text")
            | ("POST", "/v1/ingest/json")
            | ("POST", "/v1/ingest/csv")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_keeps_security_and_execution_attributes_together() {
        assert_eq!(
            route_spec("POST", "/v1/cell"),
            RouteSpec {
                access: RouteAccess::Data,
                timeout: RouteTimeoutClass::Write,
                mutating: true,
                agent_scoped: true,
            }
        );
        assert_eq!(route_spec("GET", "/v1/metrics").access, RouteAccess::Admin);
        assert_eq!(route_spec("GET", "/v1/health").access, RouteAccess::Public);
        assert!(route_spec("POST", "/v1/ingest/jobs/7/retry").mutating);
    }
}
