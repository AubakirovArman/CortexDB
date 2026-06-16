pub(super) fn expand_overview_query(query: &str) -> Option<String> {
    if !is_overview_query(query) {
        return None;
    }
    let mut expanded = query.to_owned();
    expanded.push(' ');
    expanded.push_str(
        "redwood inference company overview strategy platform product business model \
         commercial offering organization departments reliability security private deployment \
         routing serving runtime differentiation go to market support success",
    );
    Some(expanded)
}

pub(super) fn is_overview_query(query: &str) -> bool {
    let query = query.to_lowercase();
    let markers = [
        "mission statement",
        "company's thesis",
        "company thesis",
        "competitive advantage",
        "security-oriented features",
        "security oriented features",
        "serving-runtime optimizations",
        "serving runtime optimizations",
        "policy dimensions",
        "smart routing",
        "stated differentiation",
        "graceful degradation",
        "revenue streams",
        "business model",
        "add-on categories",
        "add on categories",
        "commercial offering",
        "plg-led adoption",
        "plg led adoption",
        "sales-assisted enterprise",
        "sales assisted enterprise",
        "major departments",
        "high-level organization",
        "high level organization",
    ];
    if markers.iter().any(|marker| query.contains(marker)) {
        return true;
    }
    query.contains("redwood")
        && query.contains("company")
        && [
            "mission",
            "strategy",
            "overview",
            "organization",
            "departments",
            "business",
            "commercial",
            "revenue",
            "differentiation",
        ]
        .iter()
        .any(|marker| query.contains(marker))
}

pub(super) struct OverviewQueryProfile {
    lower: String,
    terms: Vec<String>,
}

impl OverviewQueryProfile {
    pub(super) fn new(query: &str) -> Self {
        let lower = query.to_lowercase();
        let terms = cortex_engine::search::tokenize(&lower)
            .into_iter()
            .filter(|term| term.len() >= 4)
            .collect();
        Self { lower, terms }
    }
}

pub(super) fn overview_path_score(query: &OverviewQueryProfile, path: &str) -> u32 {
    let mut score = 0u32;

    if path.contains("product-docs/product-overview") {
        score += 80;
    }
    if path.contains("sales-enablement") {
        score += 55;
    }
    if path.contains("company-handbook/00_overview") {
        score += 50;
    }
    if path.contains("pricing-and-packaging") || path.contains("finance-and-legal") {
        score += 25;
    }

    if contains_any(
        &query.lower,
        &["security", "private", "deployment", "compliance"],
    ) && contains_any(
        path,
        &[
            "security-and-compliance",
            "eng-private-deployments",
            "private",
            "security",
        ],
    ) {
        score += 120;
    }
    if contains_any(
        &query.lower,
        &["serving", "runtime", "optimizations", "engine"],
    ) && contains_any(
        path,
        &[
            "eng-serving-runtime",
            "runtime-architecture",
            "kernel-and-scheduling",
            "model-serving",
        ],
    ) {
        score += 120;
    }
    if contains_any(&query.lower, &["routing", "policy", "route"]) && path.contains("routing") {
        score += 55;
    }
    if contains_any(
        &query.lower,
        &[
            "revenue",
            "business",
            "commercial",
            "add-on",
            "add on",
            "pricing",
        ],
    ) && contains_any(
        path,
        &[
            "pricing-and-packaging",
            "sales-enablement",
            "finance-and-legal",
            "product-overview",
        ],
    ) {
        score += 65;
    }
    if contains_any(
        &query.lower,
        &["plg", "sales-assisted", "sales assisted", "enterprise"],
    ) && contains_any(path, &["sales-enablement", "requirements", "product-docs"])
    {
        score += 60;
    }
    if contains_any(
        &query.lower,
        &["department", "organization", "organisation"],
    ) && contains_any(path, &["company-handbook", "people-ops", "team-wiki"])
    {
        score += 60;
    }

    for term in &query.terms {
        if path.contains(term) {
            score += 10;
        }
    }
    score
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

pub(super) fn is_overview_candidate_path(path: &str) -> bool {
    contains_any(
        path,
        &[
            "product-docs/product-overview",
            "sales-enablement",
            "company-handbook/00_overview",
            "pricing-and-packaging",
            "finance-and-legal",
            "security-and-compliance",
            "eng-private-deployments",
            "eng-serving-runtime",
            "runtime-architecture",
            "kernel-and-scheduling",
            "model-serving",
            "routing",
            "requirements",
            "people-ops",
            "team-wiki",
        ],
    )
}
