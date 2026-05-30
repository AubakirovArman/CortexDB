pub struct DashboardAsset {
    pub content_type: &'static str,
    pub body: &'static str,
}

pub fn html() -> &'static str {
    include_str!("../assets/dashboard/v1/index.html")
}

pub fn asset(path: &str) -> Option<DashboardAsset> {
    match path {
        "/dashboard/assets/v1/style.css" => Some(DashboardAsset {
            content_type: "text/css; charset=utf-8",
            body: include_str!("../assets/dashboard/v1/style.css"),
        }),
        "/dashboard/assets/v1/app.js" => Some(DashboardAsset {
            content_type: "application/javascript; charset=utf-8",
            body: include_str!("../assets/dashboard/v1/app.js"),
        }),
        _ => None,
    }
}
