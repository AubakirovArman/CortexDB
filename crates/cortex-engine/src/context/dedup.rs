use std::collections::BTreeSet;

use super::{ContextPackCell, DEFAULT_REDUNDANCY_THRESHOLD_Q16};
use crate::query::CellMetadata;
use crate::search::tokenize;

pub(crate) fn integer_sqrt(val: u128) -> u128 {
    let mut res = 0;
    let mut add = 1u128 << 63;
    while add > 0 {
        let temp = res + add;
        if temp * temp <= val {
            res = temp;
        }
        add >>= 1;
    }
    res
}

pub(crate) fn cosine_similarity_q16(u: &[i16], v: &[i16]) -> u16 {
    if u.len() != v.len() || u.is_empty() {
        return 0;
    }
    let mut dot_product: i128 = 0;
    let mut norm_u_sq: u128 = 0;
    let mut norm_v_sq: u128 = 0;
    for i in 0..u.len() {
        let ui = u[i] as i128;
        let vi = v[i] as i128;
        dot_product += ui * vi;
        norm_u_sq += (ui * ui) as u128;
        norm_v_sq += (vi * vi) as u128;
    }
    if norm_u_sq == 0 || norm_v_sq == 0 || dot_product <= 0 {
        return 0;
    }

    let norm_u = integer_sqrt(norm_u_sq);
    let norm_v = integer_sqrt(norm_v_sq);
    if norm_u == 0 || norm_v == 0 {
        return 0;
    }

    let dot_scaled = (dot_product as u128).saturating_mul(65536);
    let norm_product = norm_u.saturating_mul(norm_v);
    if norm_product == 0 {
        return 0;
    }
    let sim = dot_scaled / norm_product;
    sim.min(65535) as u16
}

pub(crate) fn extract_project_metric_value(
    payload: &[u8],
) -> (Option<String>, Option<String>, Option<String>) {
    let text = String::from_utf8_lossy(payload);
    let mut project = None;
    let mut metric = None;
    let mut value = None;
    for line in text.lines() {
        if let Some(val) = line.strip_prefix("project=") {
            project = Some(val.trim().to_owned());
        } else if let Some(val) = line.strip_prefix("metric=") {
            metric = Some(val.trim().to_owned());
        } else if let Some(val) = line.strip_prefix("value=") {
            value = Some(val.trim().to_owned());
        }
    }
    (project, metric, value)
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
