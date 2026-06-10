use std::collections::BTreeSet;

use crate::types::{
    AnswerGroundingOptionsResponse, AnswerGroundingReportResponse, AnswerGroundingSpanResponse,
    ContextAccessDecisionResponse, ContextPackAnomalyResponse, ContextPackCellResponse,
    ContextPackResponse, ContextSpanProvenanceResponse, ExplainResponse, SourceRefResponse,
};

pub type ContextPackV1 = ContextPackResponse;
pub type ContextPackCellV1 = ContextPackCellResponse;
pub type ContextPackSourceRefV1 = SourceRefResponse;
pub type ContextPackProvenanceV1 = ContextSpanProvenanceResponse;
pub type ContextPackExplainV1 = ExplainResponse;
pub type ContextPackAccessDecisionV1 = ContextAccessDecisionResponse;
pub type ContextPackAnomalyV1 = ContextPackAnomalyResponse;
pub type AnswerGroundingReportV1 = AnswerGroundingReportResponse;
pub type AnswerGroundingSpanV1 = AnswerGroundingSpanResponse;
pub type AnswerGroundingOptionsV1 = AnswerGroundingOptionsResponse;

impl Default for AnswerGroundingOptionsResponse {
    fn default() -> Self {
        Self {
            min_span_support_q16: u16::MAX,
            require_citations: false,
            reject_unsupported: false,
        }
    }
}

impl ContextPackResponse {
    pub const SCHEMA_VERSION_V1: &'static str = "context_pack.v1";

    pub fn is_v1(&self) -> bool {
        self.schema_version == Self::SCHEMA_VERSION_V1
    }

    pub fn cell_ids(&self) -> impl Iterator<Item = u64> + '_ {
        self.cells.iter().map(|cell| cell.cell_id)
    }

    pub fn citation_count(&self) -> usize {
        self.cells
            .iter()
            .filter(|cell| {
                cell.citation
                    .as_deref()
                    .is_some_and(|citation| !citation.is_empty())
            })
            .count()
    }

    pub fn anomaly_count(&self, code: &str) -> usize {
        self.anomalies
            .iter()
            .filter(|anomaly| anomaly.code == code)
            .count()
    }

    pub fn is_over_budget(&self) -> bool {
        self.estimated_tokens > self.token_budget_tokens
    }

    pub fn ground_answer(&self, answer: &str) -> AnswerGroundingReportResponse {
        self.ground_answer_with_options(answer, AnswerGroundingOptionsResponse::default())
    }

    pub fn ground_answer_with_options(
        &self,
        answer: &str,
        options: AnswerGroundingOptionsResponse,
    ) -> AnswerGroundingReportResponse {
        let spans = split_answer_spans(answer)
            .into_iter()
            .map(|span| ground_span(&self.cells, span, options))
            .collect::<Vec<_>>();
        let supported_span_count = spans.iter().filter(|span| span.supported).count() as u32;
        let unsupported_span_count = spans.iter().filter(|span| !span.supported).count() as u32;
        let answer_supported = unsupported_span_count == 0;
        AnswerGroundingReportResponse {
            answer_supported,
            rejected: !answer_supported && options.reject_unsupported,
            support_q16: average_support_q16(&spans),
            supported_span_count,
            unsupported_span_count,
            spans,
        }
    }
}

#[derive(Clone, Copy)]
struct AnswerSpan<'a> {
    text: &'a str,
    start_byte: usize,
    end_byte: usize,
}

fn ground_span(
    cells: &[ContextPackCellResponse],
    span: AnswerSpan<'_>,
    options: AnswerGroundingOptionsResponse,
) -> AnswerGroundingSpanResponse {
    let span_terms = normalize_terms(&tokenize(span.text));
    if span_terms.is_empty() {
        return AnswerGroundingSpanResponse {
            text: span.text.trim().to_owned(),
            start_byte: span.start_byte,
            end_byte: span.end_byte,
            support_q16: u16::MAX,
            supported: true,
            covered_terms: Vec::new(),
            missing_terms: Vec::new(),
            supported_by_cell_ids: Vec::new(),
            citations: Vec::new(),
        };
    }

    let mut covered = BTreeSet::new();
    let mut supporting_cells = BTreeSet::new();
    let mut citations = BTreeSet::new();
    for cell in cells {
        let cell_terms = tokenize(&cell.payload_text)
            .into_iter()
            .collect::<BTreeSet<_>>();
        let mut cell_matched = false;
        for term in &span_terms {
            if cell_terms.contains(term) {
                covered.insert(term.clone());
                cell_matched = true;
            }
        }
        if cell_matched {
            supporting_cells.insert(cell.cell_id);
            if let Some(citation) = &cell.citation {
                if !citation.trim().is_empty() {
                    citations.insert(citation.clone());
                }
            }
        }
    }

    let missing_terms = span_terms
        .iter()
        .filter(|term| !covered.contains(*term))
        .cloned()
        .collect::<Vec<_>>();
    let support_q16 = q16_ratio(covered.len(), span_terms.len());
    let has_required_citation = !options.require_citations || !citations.is_empty();
    let supported = support_q16 >= options.min_span_support_q16 && has_required_citation;

    AnswerGroundingSpanResponse {
        text: span.text.trim().to_owned(),
        start_byte: span.start_byte,
        end_byte: span.end_byte,
        support_q16,
        supported,
        covered_terms: covered.into_iter().collect(),
        missing_terms,
        supported_by_cell_ids: supporting_cells.into_iter().collect(),
        citations: citations.into_iter().collect(),
    }
}

fn split_answer_spans(answer: &str) -> Vec<AnswerSpan<'_>> {
    let mut spans = Vec::new();
    let mut start = 0usize;
    for (index, ch) in answer.char_indices() {
        if is_span_boundary(answer, index, ch) {
            push_span(answer, start, index + ch.len_utf8(), &mut spans);
            start = index + ch.len_utf8();
        }
    }
    push_span(answer, start, answer.len(), &mut spans);
    spans
}

fn is_span_boundary(answer: &str, index: usize, ch: char) -> bool {
    if matches!(ch, '!' | '?' | '\n') {
        return true;
    }
    if ch != '.' {
        return false;
    }
    let previous = answer[..index].chars().next_back();
    let next = answer[index + ch.len_utf8()..].chars().next();
    !matches!((previous, next), (Some(prev), Some(next)) if prev.is_ascii_digit() && next.is_ascii_digit())
}

fn push_span<'a>(answer: &'a str, start: usize, end: usize, spans: &mut Vec<AnswerSpan<'a>>) {
    let text = &answer[start..end];
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
    }
    let leading = text.len() - text.trim_start().len();
    let trailing = text.len() - text.trim_end().len();
    spans.push(AnswerSpan {
        text: trimmed,
        start_byte: start + leading,
        end_byte: end - trailing,
    });
}

fn tokenize(text: &str) -> Vec<String> {
    text.split(|value: char| !value.is_alphanumeric())
        .filter_map(normalize_term)
        .filter(|term| !is_stopword(term))
        .collect()
}

fn normalize_term(term: &str) -> Option<String> {
    let normalized = term
        .chars()
        .flat_map(char::to_lowercase)
        .collect::<String>();
    (!normalized.is_empty()).then_some(normalized)
}

fn normalize_terms(terms: &[String]) -> Vec<String> {
    terms
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn is_stopword(term: &str) -> bool {
    matches!(
        term,
        "a" | "an"
            | "and"
            | "the"
            | "or"
            | "of"
            | "to"
            | "in"
            | "и"
            | "в"
            | "на"
            | "для"
            | "мен"
            | "және"
            | "с"
            | "со"
            | "за"
            | "от"
            | "до"
            | "по"
            | "о"
            | "об"
            | "у"
            | "да"
            | "де"
            | "та"
            | "те"
            | "үшін"
    )
}

fn average_support_q16(spans: &[AnswerGroundingSpanResponse]) -> u16 {
    if spans.is_empty() {
        return u16::MAX;
    }
    let total = spans
        .iter()
        .map(|span| u64::from(span.support_q16))
        .sum::<u64>();
    u16::try_from(total / spans.len() as u64).unwrap_or(u16::MAX)
}

fn q16_ratio(numerator: usize, denominator: usize) -> u16 {
    if denominator == 0 {
        return u16::MAX;
    }
    ((numerator as u64 * u16::MAX as u64) / denominator as u64) as u16
}
