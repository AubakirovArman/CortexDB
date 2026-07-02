use std::collections::BTreeSet;

use super::{ContextPackCell, DEFAULT_REDUNDANCY_THRESHOLD_Q16};
use crate::query::CellMetadata;
use crate::search::tokenize;
use crate::search::vector_similarity::cosine_similarity_q16;

pub(crate) fn extract_project_metric_value(
    payload: &[u8],
) -> (Option<String>, Option<String>, Option<String>) {
    let text = String::from_utf8_lossy(payload);
    let mut project = None;
    let mut metric = None;
    let mut value = None;
    let mut currency = None;
    let mut unit = None;
    for line in text.lines() {
        let Some((key, val)) = split_metadata_line(line) else {
            continue;
        };
        match key.as_str() {
            "project" => project = Some(val),
            "metric" => metric = Some(val),
            "value" => value = Some(val),
            "currency" => currency = Some(val),
            "unit" => unit = Some(val),
            _ => {}
        }
    }
    if let Some(base_value) = value.as_mut() {
        append_value_context(base_value, currency.as_deref());
        append_value_context(base_value, unit.as_deref());
    }
    (project, metric, value)
}

fn split_metadata_line(line: &str) -> Option<(String, String)> {
    let (key, value) = line.split_once('=').or_else(|| line.split_once(':'))?;
    let key = key.trim().to_ascii_lowercase();
    let value = value.trim();
    (!key.is_empty() && !value.is_empty()).then(|| (key, value.to_owned()))
}

fn append_value_context(value: &mut String, context: Option<&str>) {
    let Some(context) = context.map(str::trim).filter(|context| !context.is_empty()) else {
        return;
    };
    let value_lower = value.to_ascii_lowercase();
    if value_lower
        .split_whitespace()
        .any(|token| token.eq_ignore_ascii_case(context))
    {
        return;
    }
    value.push(' ');
    value.push_str(context);
}

pub(crate) fn is_redundant(
    payload: &[u8],
    metadata: &CellMetadata,
    packed: &[ContextPackCell],
    threshold_q16: u16,
) -> bool {
    let (cur_proj, cur_metric, cur_val) = extract_project_metric_value(payload);
    let current_vec = crate::search::vector::vector_from_payload(payload);
    let current_terms = term_set(metadata);

    if current_vec.is_none() && current_terms.is_empty() {
        return false;
    }

    packed.iter().any(|cell| {
        let (cell_proj, cell_metric, cell_val) = extract_project_metric_value(&cell.payload);
        if cur_proj.is_some()
            && cur_proj == cell_proj
            && cur_metric.is_some()
            && cur_metric == cell_metric
            && cur_val != cell_val
        {
            return false;
        }
        if let Some(current_vec) = current_vec.as_deref() {
            if let Some(cell_vec) = crate::search::vector::vector_from_payload(&cell.payload) {
                return cosine_similarity_q16(current_vec, &cell_vec) >= threshold_q16;
            }
        }
        weighted_jaccard_q16(&current_terms, &term_set(&cell.metadata)) >= threshold_q16
    })
}

pub(crate) fn term_set(metadata: &CellMetadata) -> BTreeSet<String> {
    tokenize(&metadata.body_text).into_iter().collect()
}

pub(crate) fn weighted_jaccard_q16(left: &BTreeSet<String>, right: &BTreeSet<String>) -> u16 {
    if left.is_empty() || right.is_empty() {
        return 0;
    }
    let intersection = left.intersection(right).count() as u64;
    let union = left.union(right).count() as u64;
    ((intersection * 65_535 + union / 2) / union) as u16
}

pub(crate) fn effective_redundancy_threshold(value: u16) -> u16 {
    if value == 0 {
        DEFAULT_REDUNDANCY_THRESHOLD_Q16
    } else {
        value
    }
}
