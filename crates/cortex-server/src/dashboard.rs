pub struct DashboardAsset {
    pub content_type: &'static str,
    pub body: &'static str,
}

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
        "/dashboard/assets/v1/app.js" => Some(DashboardAsset {
            content_type: "application/javascript; charset=utf-8",
            body: include_str!("../assets/dashboard/v1/app.js"),
        }),
        "/dashboard/assets/v1/reporting.js" => Some(DashboardAsset {
            content_type: "application/javascript; charset=utf-8",
            body: include_str!("../assets/dashboard/v1/reporting.js"),
        }),
        _ => None,
    }
}
