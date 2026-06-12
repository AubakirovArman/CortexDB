mod builder;
mod hints;
mod normalize;
mod scoring;
#[cfg(test)]
mod tests;
mod types;

use super::{analyze_search_query, QueryAnchorKind};
use builder::ScopeMappingBuilder;
use hints::{lexicon_scope_hints, project_name_hints, team_name_hints};
use normalize::compact_whitespace;

pub use scoring::{scope_mapping_metadata_bonus, scope_mapping_payload_bonus};
pub use types::{QueryScopeDirective, QueryScopeField, QueryScopeMapping};

pub fn map_query_to_scope(query: &str) -> QueryScopeMapping {
    let query = compact_whitespace(query);
    let analyzed = analyze_search_query(&query);
    let mut builder = ScopeMappingBuilder::default();

    for source in analyzed.source_hints {
        builder.push(
            QueryScopeField::Source,
            source,
            61_000,
            true,
            "explicit_source_hint",
        );
    }
    for anchor in analyzed.anchors {
        match anchor.kind {
            QueryAnchorKind::TicketId => builder.push(
                QueryScopeField::Source,
                "jira".to_owned(),
                58_000,
                false,
                "ticket_anchor_source_hint",
            ),
            QueryAnchorKind::PullRequest | QueryAnchorKind::FilePath => builder.push(
                QueryScopeField::Source,
                "github".to_owned(),
                58_000,
                false,
                "code_anchor_source_hint",
            ),
            _ => {}
        }
    }

    for (field, value, confidence, reason) in lexicon_scope_hints(&query) {
        builder.push(field, value, confidence, false, reason);
    }
    for project in project_name_hints(&query).into_iter().take(8) {
        builder.push(
            QueryScopeField::Project,
            project,
            50_000,
            false,
            "project_name_hint",
        );
    }
    for team in team_name_hints(&query).into_iter().take(6) {
        builder.push(QueryScopeField::Team, team, 48_000, false, "team_name_hint");
    }

    QueryScopeMapping {
        query,
        directives: builder.finish(),
    }
}
