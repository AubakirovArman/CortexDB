use std::collections::BTreeSet;

use cortex_core::CellId;

use crate::context::ContextPackCell;
use crate::search::tokenize;

pub const DEFAULT_GROUNDING_THRESHOLD_Q16: u16 = u16::MAX;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnswerGroundingOptions {
    pub min_span_support_q16: u16,
    pub require_citations: bool,
    pub reject_unsupported: bool,
}

impl Default for AnswerGroundingOptions {
    fn default() -> Self {
        Self {
            min_span_support_q16: DEFAULT_GROUNDING_THRESHOLD_Q16,
            require_citations: false,
            reject_unsupported: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnswerGroundingSpan {
    pub text: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub support_q16: u16,
    pub supported: bool,
    pub covered_terms: Vec<String>,
    pub missing_terms: Vec<String>,
    pub supported_by_cell_ids: Vec<CellId>,
    pub citations: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnswerGroundingReport {
    pub answer_supported: bool,
    pub rejected: bool,
    pub support_q16: u16,
    pub supported_span_count: u32,
    pub unsupported_span_count: u32,
    pub spans: Vec<AnswerGroundingSpan>,
}

impl AnswerGroundingReport {
    pub fn unsupported_spans(&self) -> impl Iterator<Item = &AnswerGroundingSpan> {
        self.spans.iter().filter(|span| !span.supported)
    }
}

pub(crate) fn ground_answer(
    cells: &[ContextPackCell],
    answer: &str,
    options: AnswerGroundingOptions,
) -> AnswerGroundingReport {
    let spans = split_answer_spans(answer)
        .into_iter()
        .map(|span| ground_span(cells, span, options))
        .collect::<Vec<_>>();
    let supported_span_count = spans.iter().filter(|span| span.supported).count() as u32;
    let unsupported_span_count = spans.iter().filter(|span| !span.supported).count() as u32;
    let answer_supported = unsupported_span_count == 0;
    AnswerGroundingReport {
        answer_supported,
        rejected: !answer_supported && options.reject_unsupported,
        support_q16: average_support_q16(&spans),
        supported_span_count,
        unsupported_span_count,
        spans,
    }
}

fn ground_span(
    cells: &[ContextPackCell],
    span: AnswerSpan<'_>,
    options: AnswerGroundingOptions,
) -> AnswerGroundingSpan {
    let span_terms = normalize_terms(&tokenize(span.text));
    if span_terms.is_empty() {
        return AnswerGroundingSpan {
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
        let cell_terms = tokenize(&String::from_utf8_lossy(&cell.payload))
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

    AnswerGroundingSpan {
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

#[derive(Clone, Copy)]
struct AnswerSpan<'a> {
    text: &'a str,
    start_byte: usize,
    end_byte: usize,
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

fn normalize_terms(terms: &[String]) -> Vec<String> {
    terms
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn average_support_q16(spans: &[AnswerGroundingSpan]) -> u16 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ContextPackCell;

    fn cell(id: u64, text: &str, citation: Option<&str>) -> ContextPackCell {
        ContextPackCell {
            cell_id: CellId(id),
            payload: text.as_bytes().to_vec(),
            estimated_tokens: 0,
            citation: citation.map(str::to_owned),
            provenance: None,
            explain: None,
            access_decision: None,
        }
    }

    #[test]
    fn supported_answer_is_fully_grounded() {
        let report = ground_answer(
            &[cell(
                1,
                "source=doc-a\n\nSolar Plant budget is 1.2B KZT.",
                Some("doc-a"),
            )],
            "Solar Plant budget is 1.2B KZT.",
            AnswerGroundingOptions::default(),
        );

        assert!(report.answer_supported);
        assert_eq!(report.unsupported_span_count, 0);
        assert_eq!(report.spans[0].supported_by_cell_ids, vec![CellId(1)]);
    }

    #[test]
    fn unsupported_extra_claim_is_reported() {
        let report = ground_answer(
            &[cell(1, "Solar Plant budget is 1.2B KZT.", Some("doc-a"))],
            "Solar Plant budget is 1.2B KZT. Project TTL is 45 days.",
            AnswerGroundingOptions {
                reject_unsupported: true,
                ..AnswerGroundingOptions::default()
            },
        );

        assert!(!report.answer_supported);
        assert!(report.rejected);
        assert_eq!(report.supported_span_count, 1);
        assert_eq!(report.unsupported_span_count, 1);
        assert_eq!(
            report.unsupported_spans().next().unwrap().text,
            "Project TTL is 45 days."
        );
    }

    #[test]
    fn citation_requirement_can_make_claim_unsupported() {
        let report = ground_answer(
            &[cell(1, "Solar Plant budget is 1.2B KZT.", None)],
            "Solar Plant budget is 1.2B KZT.",
            AnswerGroundingOptions {
                require_citations: true,
                ..AnswerGroundingOptions::default()
            },
        );

        assert!(!report.answer_supported);
        assert_eq!(report.spans[0].missing_terms, Vec::<String>::new());
        assert_eq!(report.spans[0].support_q16, u16::MAX);
    }
}
