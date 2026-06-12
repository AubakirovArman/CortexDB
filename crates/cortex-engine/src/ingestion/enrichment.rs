use crate::ingestion::chunking::sanitize_header_value;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SourceMetadataEnrichment {
    pub project: Option<String>,
    pub entity: Option<String>,
    pub owner: Option<String>,
    pub status_tag: Option<String>,
    pub event_date: Option<String>,
    pub topic: Option<String>,
}

impl SourceMetadataEnrichment {
    pub fn from_text(text: &str, source: &str) -> Self {
        let mut enrichment = Self::default();
        for line in text.lines().take(80) {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if enrichment.topic.is_none() {
                enrichment.topic = topic_from_heading(trimmed);
            }
            if enrichment.project.is_none() {
                enrichment.project =
                    value_after_label(trimmed, &["project", "program", "initiative"]);
            }
            if enrichment.entity.is_none() {
                enrichment.entity =
                    value_after_label(trimmed, &["entity", "customer", "client", "team"]);
            }
            if enrichment.owner.is_none() {
                enrichment.owner =
                    value_after_label(trimmed, &["owner", "dri", "assignee", "responsible"]);
            }
            if enrichment.status_tag.is_none() {
                enrichment.status_tag = status_from_line(trimmed);
            }
            if enrichment.event_date.is_none() {
                enrichment.event_date = iso_date_from_line(trimmed);
            }
        }
        if enrichment.topic.is_none() {
            enrichment.topic = topic_from_source(source);
        }
        enrichment
    }

    pub fn header_pairs(&self) -> Vec<(&'static str, String)> {
        let mut headers = Vec::new();
        push_header(&mut headers, "project", self.project.as_deref());
        push_header(&mut headers, "entity", self.entity.as_deref());
        push_header(&mut headers, "owner", self.owner.as_deref());
        push_header(&mut headers, "status_tag", self.status_tag.as_deref());
        push_header(&mut headers, "event_date", self.event_date.as_deref());
        push_header(&mut headers, "topic", self.topic.as_deref());
        headers
    }
}

fn push_header(headers: &mut Vec<(&'static str, String)>, key: &'static str, value: Option<&str>) {
    if let Some(value) = value {
        headers.push((key, sanitize_header_value(value)));
    }
}

fn value_after_label(line: &str, labels: &[&str]) -> Option<String> {
    let (key, value) = line.split_once(':').or_else(|| line.split_once('='))?;
    let key = normalize_key(key);
    labels
        .iter()
        .any(|label| key == *label)
        .then(|| clean_value(value))
        .filter(|value| !value.is_empty())
}

fn normalize_key(key: &str) -> String {
    key.trim().to_lowercase().replace([' ', '-'], "_")
}

fn clean_value(value: &str) -> String {
    value
        .trim()
        .trim_matches(|character| matches!(character, '"' | '\'' | '`' | '[' | ']'))
        .trim_end_matches(['.', ',', ';'])
        .trim()
        .to_owned()
}

fn status_from_line(line: &str) -> Option<String> {
    if let Some(value) = value_after_label(line, &["status", "state"]) {
        return Some(normalize_status(&value));
    }
    let lower = line.to_lowercase();
    [
        "blocked",
        "approved",
        "delayed",
        "done",
        "closed",
        "open",
        "ready",
        "risk",
        "launched",
        "cancelled",
    ]
    .into_iter()
    .find(|word| {
        lower
            .split(|c: char| !c.is_alphanumeric())
            .any(|part| part == *word)
    })
    .map(ToOwned::to_owned)
}

fn normalize_status(value: &str) -> String {
    value
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .find(|part| !part.is_empty())
        .unwrap_or("unknown")
        .to_owned()
}

fn iso_date_from_line(line: &str) -> Option<String> {
    let bytes = line.as_bytes();
    if bytes.len() < 10 {
        return None;
    }
    for index in 0..=bytes.len() - 10 {
        let candidate = &line[index..index + 10];
        if is_iso_date(candidate) {
            return Some(candidate.to_owned());
        }
    }
    None
}

fn is_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[8..10].iter().all(u8::is_ascii_digit)
}

fn topic_from_heading(line: &str) -> Option<String> {
    line.strip_prefix('#')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(clean_value)
}

fn topic_from_source(source: &str) -> Option<String> {
    source
        .rsplit(['/', '\\'])
        .next()
        .and_then(|file_name| file_name.split('.').next())
        .map(|value| value.replace(['_', '-'], " "))
        .map(|value| clean_value(&value))
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enrichment_extracts_project_owner_status_date_and_topic() {
        let enrichment = SourceMetadataEnrichment::from_text(
            "# Migration Runbook\nProject: Apollo\nOwner: Alice Lee\nStatus: Blocked\nDue 2026-05-14",
            "docs/runbooks/migration.md",
        );

        assert_eq!(enrichment.project.as_deref(), Some("Apollo"));
        assert_eq!(enrichment.owner.as_deref(), Some("Alice Lee"));
        assert_eq!(enrichment.status_tag.as_deref(), Some("blocked"));
        assert_eq!(enrichment.event_date.as_deref(), Some("2026-05-14"));
        assert_eq!(enrichment.topic.as_deref(), Some("Migration Runbook"));
    }
}
