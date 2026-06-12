use super::types::GraphEdgeKind;

impl GraphEdgeKind {
    pub fn from_predicate(predicate: &str) -> Self {
        match normalize_predicate(predicate).as_str() {
            "source_supports_fact" | "supports_fact" | "source_supports" => {
                Self::SourceSupportsFact
            }
            "fact_contradicts_fact" | "contradicts_fact" | "contradicts" => {
                Self::FactContradictsFact
            }
            _ => Self::Other,
        }
    }

    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::SourceSupportsFact => "source_supports_fact",
            Self::FactContradictsFact => "fact_contradicts_fact",
            Self::Other => "other",
        }
    }

    pub(super) fn from_ackg_kind(value: &str) -> Self {
        match value {
            "source_supports_fact" => Self::SourceSupportsFact,
            "fact_contradicts_fact" => Self::FactContradictsFact,
            _ => Self::Other,
        }
    }
}

fn normalize_predicate(predicate: &str) -> String {
    predicate.trim().to_lowercase().replace([' ', '-'], "_")
}
