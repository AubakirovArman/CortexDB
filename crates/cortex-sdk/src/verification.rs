use crate::aql::{Aql, AqlBuildError};
use crate::http::path;
use crate::types::{EvidenceResponse, NumericConflictResponse, VerificationReportResponse};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerifyOutputFormat {
    Json,
    Markdown,
    Audit,
}

impl VerifyOutputFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Markdown => "markdown",
            Self::Audit => "audit",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifyRequest {
    scope: String,
    statement: String,
    format: VerifyOutputFormat,
}

impl VerifyRequest {
    pub fn new(scope: impl Into<String>, statement: impl Into<String>) -> Self {
        Self {
            scope: scope.into(),
            statement: statement.into(),
            format: VerifyOutputFormat::Json,
        }
    }

    pub fn fact(
        scope: impl Into<String>,
        fact: impl Into<String>,
        brain: impl Into<String>,
    ) -> Result<Self, AqlBuildError> {
        let statement = Aql::verify_fact(fact, brain).build()?;
        Ok(Self::new(scope, statement))
    }

    pub fn json(mut self) -> Self {
        self.format = VerifyOutputFormat::Json;
        self
    }

    pub fn markdown(mut self) -> Self {
        self.format = VerifyOutputFormat::Markdown;
        self
    }

    pub fn audit(mut self) -> Self {
        self.format = VerifyOutputFormat::Audit;
        self
    }

    pub fn with_format(mut self, format: VerifyOutputFormat) -> Self {
        self.format = format;
        self
    }

    pub fn scope(&self) -> &str {
        &self.scope
    }

    pub fn statement(&self) -> &str {
        &self.statement
    }

    pub fn format(&self) -> VerifyOutputFormat {
        self.format
    }

    pub fn path(&self) -> String {
        path(
            "/v1/verify",
            &[
                ("scope", self.scope.as_str()),
                ("format", self.format.as_str()),
            ],
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VerifyResult {
    Supported,
    Insufficient,
    Contradicted,
    MixedEvidence,
    Unknown(String),
}

impl VerifyResult {
    pub fn from_wire(value: &str) -> Self {
        match value {
            "supported" => Self::Supported,
            "insufficient" => Self::Insufficient,
            "contradicted" => Self::Contradicted,
            "mixed" | "mixed_evidence" => Self::MixedEvidence,
            other => Self::Unknown(other.to_owned()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Supported => "supported",
            Self::Insufficient => "insufficient",
            Self::Contradicted => "contradicted",
            Self::MixedEvidence => "mixed_evidence",
            Self::Unknown(value) => value.as_str(),
        }
    }

    pub fn is_actionable_conflict(&self) -> bool {
        matches!(self, Self::Contradicted | Self::MixedEvidence)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VerifyConflict {
    ContradictingEvidence(VerifyEvidenceConflict),
    Numeric(VerifyNumericConflict),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifyEvidenceConflict {
    pub cell_id: u64,
    pub matched_terms: u32,
    pub match_score_q16: u16,
    pub match_kind: String,
    pub source_trust_q16: u16,
    pub source_trust_category: String,
    pub citation: Option<String>,
    pub payload_text: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifyNumericConflict {
    pub metric: String,
    pub left: String,
    pub right: String,
}

impl From<EvidenceResponse> for VerifyEvidenceConflict {
    fn from(value: EvidenceResponse) -> Self {
        Self {
            cell_id: value.cell_id,
            matched_terms: value.matched_terms,
            match_score_q16: value.match_score_q16,
            match_kind: value.match_kind,
            source_trust_q16: value.source_trust_q16,
            source_trust_category: value.source_trust_category,
            citation: value.citation,
            payload_text: value.payload_text,
        }
    }
}

impl From<NumericConflictResponse> for VerifyNumericConflict {
    fn from(value: NumericConflictResponse) -> Self {
        Self {
            metric: value.metric,
            left: value.left,
            right: value.right,
        }
    }
}

pub trait VerificationReportExt {
    fn result(&self) -> VerifyResult;
    fn has_conflicts(&self) -> bool;
    fn conflicts(&self) -> Vec<VerifyConflict>;
}

impl VerificationReportExt for VerificationReportResponse {
    fn result(&self) -> VerifyResult {
        let status = VerifyResult::from_wire(&self.status);
        match status {
            VerifyResult::Unknown(_) => VerifyResult::from_wire(&self.verdict),
            known => known,
        }
    }

    fn has_conflicts(&self) -> bool {
        !self.contradicting_evidence.is_empty() || !self.numeric_conflicts.is_empty()
    }

    fn conflicts(&self) -> Vec<VerifyConflict> {
        let evidence = self
            .contradicting_evidence
            .iter()
            .cloned()
            .map(VerifyEvidenceConflict::from)
            .map(VerifyConflict::ContradictingEvidence);
        let numeric = self
            .numeric_conflicts
            .iter()
            .cloned()
            .map(VerifyNumericConflict::from)
            .map(VerifyConflict::Numeric);

        evidence.chain(numeric).collect()
    }
}
