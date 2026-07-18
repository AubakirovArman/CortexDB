pub struct DashboardAsset {
    pub content_type: &'static str,
    pub body: &'static str,
}

pub const SECURITY_HEADERS: &[(&str, &str)] = &[
    (
        "content-security-policy",
        "default-src 'self'; script-src 'self'; style-src 'self'; connect-src 'self'; img-src 'self' data:; object-src 'none'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'",
    ),
    ("x-content-type-options", "nosniff"),
    ("x-frame-options", "DENY"),
    ("referrer-policy", "no-referrer"),
];

pub const ROUTES: &[&str] = &[
    "overview",
    "permissions",
    "cells",
    "search",
    "ann-eval",
    "aql",
    "context",
    "verify",
    "ingest",
    "storage",
    "cluster",
];

pub fn html() -> &'static str {
    include_str!("../assets/dashboard/v1/index.html")
}

pub fn is_page(path: &str) -> bool {
    if matches!(path, "/" | "/dashboard" | "/dashboard/") {
        return true;
    }
    let Some(route) = path.strip_prefix("/dashboard/") else {
        return false;
    };
    ROUTES.contains(&route.trim_end_matches('/'))
}

pub fn asset(path: &str) -> Option<DashboardAsset> {
    match path {
        "/dashboard/assets/v1/style.css" => Some(DashboardAsset {
            content_type: "text/css; charset=utf-8",
            body: include_str!("../assets/dashboard/v1/style.css"),
        }),
        "/dashboard/assets/v1/reporting_common.js" => Some(DashboardAsset {
            content_type: "application/javascript; charset=utf-8",
            body: include_str!("../assets/dashboard/v1/reporting_common.js"),
        }),
        "/dashboard/assets/v1/reporting_retrieval.js" => Some(DashboardAsset {
            content_type: "application/javascript; charset=utf-8",
            body: include_str!("../assets/dashboard/v1/reporting_retrieval.js"),
        }),
        "/dashboard/assets/v1/reporting_operations.js" => Some(DashboardAsset {
            content_type: "application/javascript; charset=utf-8",
            body: include_str!("../assets/dashboard/v1/reporting_operations.js"),
        }),
        "/dashboard/assets/v1/reporting_operations_status.js" => Some(DashboardAsset {
            content_type: "application/javascript; charset=utf-8",
            body: include_str!("../assets/dashboard/v1/reporting_operations_status.js"),
        }),
        "/dashboard/assets/v1/reporting_operations_permissions.js" => Some(DashboardAsset {
            content_type: "application/javascript; charset=utf-8",
            body: include_str!("../assets/dashboard/v1/reporting_operations_permissions.js"),
        }),
        "/dashboard/assets/v1/reporting_slo.js" => Some(DashboardAsset {
            content_type: "application/javascript; charset=utf-8",
            body: include_str!("../assets/dashboard/v1/reporting_slo.js"),
        }),
        "/dashboard/assets/v1/reporting_ingest.js" => Some(DashboardAsset {
            content_type: "application/javascript; charset=utf-8",
            body: include_str!("../assets/dashboard/v1/reporting_ingest.js"),
        }),
        "/dashboard/assets/v1/reporting_audit.js" => Some(DashboardAsset {
            content_type: "application/javascript; charset=utf-8",
            body: include_str!("../assets/dashboard/v1/reporting_audit.js"),
        }),
        "/dashboard/assets/v1/app.js" => Some(DashboardAsset {
            content_type: "application/javascript; charset=utf-8",
            body: include_str!("../assets/dashboard/v1/app.js"),
        }),
        "/dashboard/assets/v1/reporting.js" => Some(DashboardAsset {
            content_type: "application/javascript; charset=utf-8",
            body: include_str!("../assets/dashboard/v1/reporting.js"),
        }),
        "/dashboard/assets/v1/app_state.js" => Some(DashboardAsset {
            content_type: "application/javascript; charset=utf-8",
            body: include_str!("../assets/dashboard/v1/app_state.js"),
        }),
        "/dashboard/assets/v1/app_api.js" => Some(DashboardAsset {
            content_type: "application/javascript; charset=utf-8",
            body: include_str!("../assets/dashboard/v1/app_api.js"),
        }),
        "/dashboard/assets/v1/app_access.js" => Some(DashboardAsset {
            content_type: "application/javascript; charset=utf-8",
            body: include_str!("../assets/dashboard/v1/app_access.js"),
        }),
        "/dashboard/assets/v1/app_status.js" => Some(DashboardAsset {
            content_type: "application/javascript; charset=utf-8",
            body: include_str!("../assets/dashboard/v1/app_status.js"),
        }),
        "/dashboard/assets/v1/app_incidents.js" => Some(DashboardAsset {
            content_type: "application/javascript; charset=utf-8",
            body: include_str!("../assets/dashboard/v1/app_incidents.js"),
        }),
        "/dashboard/assets/v1/app_status_summaries.js" => Some(DashboardAsset {
            content_type: "application/javascript; charset=utf-8",
            body: include_str!("../assets/dashboard/v1/app_status_summaries.js"),
        }),
        "/dashboard/assets/v1/app_slo_backup.js" => Some(DashboardAsset {
            content_type: "application/javascript; charset=utf-8",
            body: include_str!("../assets/dashboard/v1/app_slo_backup.js"),
        }),
        "/dashboard/assets/v1/app_bindings.js" => Some(DashboardAsset {
            content_type: "application/javascript; charset=utf-8",
            body: include_str!("../assets/dashboard/v1/app_bindings.js"),
        }),
        _ => None,
    }
}
