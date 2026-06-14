use crate::search::tokenize;
use cortex_core::CellId;

use super::{
    ContextAccessDecision, ContextPack, ContextPackAnomaly, ContextPackAnomalyCode,
    ContextScoreComponent,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextCellExplain {
    pub cell_id: CellId,
    pub outcome: ContextCellExplainOutcome,
    pub first_excluding_stage: Option<String>,
    pub why_selected: Option<String>,
    pub why_excluded: Option<String>,
    pub score: Option<u32>,
    pub matched_terms: Vec<String>,
    pub score_components: Vec<ContextScoreComponent>,
    pub access_decision: Option<ContextAccessDecision>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextCellExplainOutcome {
    Selected,
    Excluded,
    NotConsidered,
}

impl ContextCellExplainOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Selected => "selected",
            Self::Excluded => "excluded",
            Self::NotConsidered => "not_considered",
        }
    }
}

impl ContextPack {
    pub fn explain_cell(&self, cell_id: CellId) -> ContextCellExplain {
        if let Some(cell) = self.cells.iter().find(|cell| cell.cell_id == cell_id) {
            let explain = cell.explain.as_ref();
            return ContextCellExplain {
                cell_id,
                outcome: ContextCellExplainOutcome::Selected,
                first_excluding_stage: None,
                why_selected: explain.map(|explain| explain.why_selected.clone()),
                why_excluded: None,
                score: explain.map(|explain| explain.score),
                matched_terms: explain
                    .map(|explain| explain.matched_terms.clone())
                    .unwrap_or_default(),
                score_components: explain
                    .map(|explain| explain.score_components.clone())
                    .unwrap_or_default(),
                access_decision: cell.access_decision.clone(),
            };
        }

        if let Some(anomaly) = self
            .anomalies
            .iter()
            .find(|anomaly| anomaly.cell_id == Some(cell_id))
        {
            return excluded_cell_explain(cell_id, anomaly);
        }

        ContextCellExplain {
            cell_id,
            outcome: ContextCellExplainOutcome::NotConsidered,
            first_excluding_stage: None,
            why_selected: None,
            why_excluded: Some(
                "cell was not selected and no exclusion anomaly was recorded in this ContextPack"
                    .to_owned(),
            ),
            score: None,
            matched_terms: Vec::new(),
            score_components: Vec::new(),
            access_decision: None,
        }
    }
}

fn excluded_cell_explain(cell_id: CellId, anomaly: &ContextPackAnomaly) -> ContextCellExplain {
    ContextCellExplain {
        cell_id,
        outcome: ContextCellExplainOutcome::Excluded,
        first_excluding_stage: Some(first_excluding_stage(anomaly.code).to_owned()),
        why_selected: None,
        why_excluded: Some(
            anomaly
                .why_excluded
                .clone()
                .unwrap_or_else(|| anomaly.message.clone()),
        ),
        score: None,
        matched_terms: Vec::new(),
        score_components: Vec::new(),
        access_decision: None,
    }
}

fn first_excluding_stage(code: ContextPackAnomalyCode) -> &'static str {
    match code {
        ContextPackAnomalyCode::RedundantCell => "redundancy",
        ContextPackAnomalyCode::MissingCitation => "citation_requirement",
        ContextPackAnomalyCode::TokenOverload => "token_budget",
        ContextPackAnomalyCode::ScopeMismatch => "agent_scope",
        ContextPackAnomalyCode::InsufficientContext => "answerability",
        ContextPackAnomalyCode::VisibleConflict => "conflict_visibility",
    }
}

pub fn generate_selection_reason(
    _score: u32,
    base_bm25: u32,
    source_trust: u32,
    penalty: u32,
) -> String {
    if penalty > 10000 {
        "Selected due to matching terms, but penalized for high redundancy with other context"
            .to_owned()
    } else if source_trust > 40000 {
        "Selected due to high provenance source trust and relevant query terms".to_owned()
    } else if base_bm25 > 8000 {
        "Selected due to strong keyword relevance match".to_owned()
    } else {
        "Selected as relevant background context".to_owned()
    }
}

pub(crate) fn extract_query_terms(aql: &str) -> Vec<String> {
    let mut terms = Vec::new();
    let mut in_quotes = false;
    let mut current = String::new();
    for character in aql.chars() {
        if character == '"' {
            if in_quotes {
                terms.extend(tokenize(&current));
                current.clear();
            }
            in_quotes = !in_quotes;
        } else if in_quotes {
            current.push(character);
        }
    }
    terms
}
