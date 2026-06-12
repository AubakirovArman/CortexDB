use super::utils::normalize_label;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EnterpriseRagQuestionType {
    Basic,
    Semantic,
    IntraDocumentReasoning,
    ProjectRelated,
    Constrained,
    ConflictingInfo,
    Completeness,
    HighLevel,
    InfoNotFound,
    Miscellaneous,
}

impl EnterpriseRagQuestionType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Basic => "basic",
            Self::Semantic => "semantic",
            Self::IntraDocumentReasoning => "intra_document_reasoning",
            Self::ProjectRelated => "project_related",
            Self::Constrained => "constrained",
            Self::ConflictingInfo => "conflicting_info",
            Self::Completeness => "completeness",
            Self::HighLevel => "high_level",
            Self::InfoNotFound => "info_not_found",
            Self::Miscellaneous => "miscellaneous",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match normalize_label(value).as_str() {
            "basic" => Some(Self::Basic),
            "semantic" => Some(Self::Semantic),
            "intra_document_reasoning" | "intradocumentreasoning" => {
                Some(Self::IntraDocumentReasoning)
            }
            "project_related" | "projectrelated" => Some(Self::ProjectRelated),
            "constrained" => Some(Self::Constrained),
            "conflicting_info" | "conflictinginfo" => Some(Self::ConflictingInfo),
            "completeness" => Some(Self::Completeness),
            "high_level" | "highlevel" => Some(Self::HighLevel),
            "info_not_found" | "infonotfound" | "unavailable" | "null_query" | "nullquery" => {
                Some(Self::InfoNotFound)
            }
            "miscellaneous" | "misc" => Some(Self::Miscellaneous),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnterpriseRagIntentClassification {
    pub question_type: EnterpriseRagQuestionType,
    pub confidence_q16: u16,
    pub matched_signals: Vec<&'static str>,
}
