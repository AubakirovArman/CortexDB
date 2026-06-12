use cortex_core::CellId;

use super::large_cell::estimate_cell_tokens;
use super::{
    ContextPackAnomaly, ContextPackAnomalyCode, ContextPackOptions, ContextSpanProvenance,
};
use crate::query::metadata::SourceRef;
use crate::search::tokenize;

pub(crate) struct ContextSpanRequest<'a> {
    pub cell_id: CellId,
    pub payload: &'a [u8],
    pub citation: Option<&'a str>,
    pub citations_required: bool,
    pub original_tokens: u32,
    pub remaining_tokens: u32,
    pub query_terms: &'a [String],
    pub options: &'a ContextPackOptions,
    pub source_ref: Option<&'a SourceRef>,
}

pub(crate) struct ContextSpanSelection {
    pub payload: Vec<u8>,
    pub estimated_tokens: u32,
    pub provenance: ContextSpanProvenance,
    pub anomaly: ContextPackAnomaly,
}

pub(crate) fn select_relevant_span(
    request: ContextSpanRequest<'_>,
) -> Option<ContextSpanSelection> {
    if !request.options.span_level_packing
        || request.remaining_tokens == 0
        || request.query_terms.is_empty()
        || request.original_tokens <= request.remaining_tokens
    {
        return None;
    }

    let parsed = ParsedPayload::parse(request.payload);
    let lines = body_lines(parsed.body, parsed.body_start_byte);
    let best = best_line(&lines, request.query_terms)?;
    let (start, end, selected, estimated_tokens) =
        select_budgeted_window(&request, &parsed, &lines, best)?;
    if selected.is_empty() {
        return None;
    }

    Some(ContextSpanSelection {
        payload: selected,
        estimated_tokens,
        provenance: ContextSpanProvenance {
            source_cell_id: request.cell_id,
            source_byte_start: lines[start].start_byte,
            source_byte_end: lines[end].end_byte,
            source_line_start: u32::try_from(start + 1).unwrap_or(u32::MAX),
            source_line_end: u32::try_from(end + 1).unwrap_or(u32::MAX),
            source_ref: request.source_ref.cloned(),
        },
        anomaly: ContextPackAnomaly {
            cell_id: Some(request.cell_id),
            code: ContextPackAnomalyCode::TokenOverload,
            message: "large cell was reduced to a query-relevant span".to_owned(),
            why_excluded: Some(format!(
                "span_level_packing selected lines {}..{}; original_estimated_tokens={} remaining_tokens={}",
                start + 1,
                end + 1,
                request.original_tokens,
                request.remaining_tokens
            )),
        },
    })
}

struct ParsedPayload<'a> {
    header: &'a str,
    body: &'a str,
    body_start_byte: usize,
}

impl<'a> ParsedPayload<'a> {
    fn parse(payload: &'a [u8]) -> Self {
        let text = std::str::from_utf8(payload).unwrap_or_default();
        if let Some((header, body)) = text.split_once("\n\n") {
            Self {
                header,
                body,
                body_start_byte: header.len() + 2,
            }
        } else {
            Self {
                header: "",
                body: text,
                body_start_byte: 0,
            }
        }
    }
}

struct BodyLine<'a> {
    text: &'a str,
    start_byte: usize,
    end_byte: usize,
}

fn body_lines(body: &str, body_start_byte: usize) -> Vec<BodyLine<'_>> {
    let mut lines = Vec::new();
    let mut offset = body_start_byte;
    for raw in body.split_inclusive('\n') {
        let text = raw.trim_end_matches(['\r', '\n']);
        lines.push(BodyLine {
            text,
            start_byte: offset,
            end_byte: offset + text.len(),
        });
        offset += raw.len();
    }
    if body.is_empty() {
        return lines;
    }
    if !body.ends_with('\n') && lines.is_empty() {
        lines.push(BodyLine {
            text: body,
            start_byte: body_start_byte,
            end_byte: body_start_byte + body.len(),
        });
    }
    lines
}

fn best_line(lines: &[BodyLine<'_>], query_terms: &[String]) -> Option<usize> {
    let mut best_index = None;
    let mut best_score = 0u32;
    for (index, line) in lines.iter().enumerate() {
        let score = line_score(line.text, query_terms);
        if score > best_score {
            best_index = Some(index);
            best_score = score;
        }
    }
    best_index
}

fn line_score(line: &str, query_terms: &[String]) -> u32 {
    let lower = line.to_ascii_lowercase();
    let line_terms = tokenize(line);
    let mut score = 0u32;
    for term in query_terms {
        if line_terms.iter().any(|line_term| line_term == term) {
            score = score.saturating_add(8);
        } else if lower.contains(term) {
            score = score.saturating_add(3);
        }
    }
    score
}

fn expand_window(lines: &[BodyLine<'_>], best: usize, context_lines: usize) -> (usize, usize) {
    let start = best.saturating_sub(context_lines);
    let end = best
        .saturating_add(context_lines)
        .min(lines.len().saturating_sub(1));
    (start, end)
}

fn select_budgeted_window(
    request: &ContextSpanRequest<'_>,
    parsed: &ParsedPayload<'_>,
    lines: &[BodyLine<'_>],
    best: usize,
) -> Option<(usize, usize, Vec<u8>, u32)> {
    let (target_start, target_end) = expand_window(lines, best, request.options.span_context_lines);
    let mut start = best;
    let mut end = best;
    let (mut selected, mut estimated_tokens) =
        build_estimated_range(request, parsed, lines, start, end)?;
    if estimated_tokens > request.remaining_tokens {
        return None;
    }

    while start > target_start || end < target_end {
        let mut changed = false;
        if start > target_start {
            let next_start = start - 1;
            if let Some((candidate, tokens)) =
                build_estimated_range(request, parsed, lines, next_start, end)
            {
                if tokens <= request.remaining_tokens {
                    start = next_start;
                    selected = candidate;
                    estimated_tokens = tokens;
                    changed = true;
                }
            }
        }
        if end < target_end {
            let next_end = end + 1;
            if let Some((candidate, tokens)) =
                build_estimated_range(request, parsed, lines, start, next_end)
            {
                if tokens <= request.remaining_tokens {
                    end = next_end;
                    selected = candidate;
                    estimated_tokens = tokens;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }

    Some((start, end, selected, estimated_tokens))
}

fn build_estimated_range(
    request: &ContextSpanRequest<'_>,
    parsed: &ParsedPayload<'_>,
    lines: &[BodyLine<'_>],
    start: usize,
    end: usize,
) -> Option<(Vec<u8>, u32)> {
    if start > end || end >= lines.len() {
        return None;
    }
    let payload = build_payload(parsed.header, &lines[start..=end], start, end);
    let tokens = estimate_cell_tokens(
        &payload,
        request.citation,
        request.citations_required,
        request.options,
    );
    Some((payload, tokens))
}

fn build_payload(header: &str, lines: &[BodyLine<'_>], start: usize, end: usize) -> Vec<u8> {
    let mut out = String::new();
    if !header.is_empty() {
        out.push_str(header);
        out.push_str("\n\n");
    }
    out.push_str(
        &lines
            .iter()
            .map(|line| line.text)
            .collect::<Vec<_>>()
            .join("\n"),
    );
    out.push_str(&format!(
        "\n[context_pack_span=true line_start={} line_end={}]\n",
        start + 1,
        end + 1
    ));
    out.into_bytes()
}
