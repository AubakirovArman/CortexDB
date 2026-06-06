use std::collections::BTreeSet;

use crate::context::{ContextPackAnomaly, ContextPackAnomalyCode, ContextPackCell};
use crate::search::tokenize;

pub const DEFAULT_ANSWERABILITY_THRESHOLD_Q16: u16 = u16::MAX;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ContextAnswerability {
    pub score_q16: u16,
    pub covered_terms: Vec<String>,
    pub missing_terms: Vec<String>,
}

impl ContextAnswerability {
    pub fn is_insufficient(&self) -> bool {
        self.score_q16 < DEFAULT_ANSWERABILITY_THRESHOLD_Q16
    }
}

pub(crate) fn estimate(cells: &[ContextPackCell], query_terms: &[String]) -> ContextAnswerability {
    if cells.is_empty() {
        return ContextAnswerability {
            score_q16: 0,
            covered_terms: Vec::new(),
            missing_terms: normalized_terms(query_terms),
        };
    }

    let required_terms = normalized_terms(query_terms);
    if required_terms.is_empty() {
        return ContextAnswerability {
            score_q16: u16::MAX,
            covered_terms: Vec::new(),
            missing_terms: Vec::new(),
        };
    }

    let cell_terms = cells
        .iter()
        .flat_map(|cell| tokenize(&String::from_utf8_lossy(&cell.payload)))
        .collect::<BTreeSet<_>>();
    let mut covered_terms = Vec::new();
    let mut missing_terms = Vec::new();

    for term in required_terms {
        if cell_terms.contains(&term) {
            covered_terms.push(term);
        } else {
            missing_terms.push(term);
        }
    }

    ContextAnswerability {
        score_q16: q16_ratio(
            covered_terms.len(),
            covered_terms.len() + missing_terms.len(),
        ),
        covered_terms,
        missing_terms,
    }
}

pub(crate) fn insufficient_context_anomaly(
    answerability: &ContextAnswerability,
) -> ContextPackAnomaly {
    ContextPackAnomaly {
        cell_id: None,
        code: ContextPackAnomalyCode::InsufficientContext,
        message: format!(
            "context answerability score {}/65535 is below the required threshold",
            answerability.score_q16
        ),
        why_excluded: Some(format!(
            "covered_terms=[{}]; missing_terms=[{}]",
            answerability.covered_terms.join(","),
            answerability.missing_terms.join(",")
        )),
    }
}

fn normalized_terms(query_terms: &[String]) -> Vec<String> {
    query_terms
        .iter()
        .flat_map(|term| tokenize(term))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn q16_ratio(numerator: usize, denominator: usize) -> u16 {
    if denominator == 0 {
        return u16::MAX;
    }
    let scaled = (numerator as u64 * u16::MAX as u64) / denominator as u64;
    u16::try_from(scaled).unwrap_or(u16::MAX)
}
