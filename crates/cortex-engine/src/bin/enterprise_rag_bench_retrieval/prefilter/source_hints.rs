pub(crate) fn inferred_source_types_from_query(query: &str) -> Vec<String> {
    let lower = query.to_lowercase();
    let mut values = Vec::<String>::new();
    let mut push = |source: &str| {
        if !values.iter().any(|value| value == source) {
            values.push(source.to_owned());
        }
    };
    if contains_query_marker(&lower, &["slack", "slack thread", "channel"]) {
        push("slack");
    }
    if contains_query_marker(&lower, &["gmail", "email", "mail thread", "customer email"]) {
        push("gmail");
    }
    if contains_query_marker(&lower, &["jira", "jira issue", "jira ticket"]) {
        push("jira");
    }
    if contains_query_marker(
        &lower,
        &["github", "pull request", "pr #", "repository", "repo"],
    ) {
        push("github");
    }
    if contains_query_marker(
        &lower,
        &["google drive", "drive doc", "drive document", "drive file"],
    ) {
        push("google_drive");
    }
    if contains_query_marker(&lower, &["linear", "linear issue"]) {
        push("linear");
    }
    if contains_query_marker(&lower, &["hubspot", "account note", "crm"]) {
        push("hubspot");
    }
    if contains_query_marker(
        &lower,
        &["fireflies", "meeting transcript", "call transcript"],
    ) {
        push("fireflies");
    }
    if contains_query_marker(&lower, &["confluence", "wiki page", "runbook", "adr"]) {
        push("confluence");
    }
    values
}

fn contains_query_marker(query: &str, markers: &[&str]) -> bool {
    markers
        .iter()
        .any(|marker| contains_query_marker_value(query, marker))
}

fn contains_query_marker_value(query: &str, marker: &str) -> bool {
    if marker.chars().any(|ch| !ch.is_ascii_alphanumeric()) {
        return query.contains(marker);
    }
    let mut start = 0usize;
    while let Some(relative) = query[start..].find(marker) {
        let index = start + relative;
        let end = index + marker.len();
        let before = query[..index].chars().next_back();
        let after = query[end..].chars().next();
        let left_boundary = before.is_none_or(|ch| !ch.is_ascii_alphanumeric());
        let right_boundary = after.is_none_or(|ch| !ch.is_ascii_alphanumeric());
        if left_boundary && right_boundary {
            return true;
        }
        start = end;
    }
    false
}
