use std::collections::{BTreeMap, BTreeSet};

pub(super) fn average_len_q16(doc_lengths: &BTreeMap<u32, u32>, allowed: &BTreeSet<u32>) -> u64 {
    let mut count = 0u64;
    let mut total = 0u64;
    for candidate in allowed {
        if let Some(length) = doc_lengths.get(candidate) {
            count += 1;
            total += u64::from(*length);
        }
    }
    total
        .saturating_mul(65_536)
        .checked_div(count)
        .unwrap_or(65_536)
}

pub(super) fn average_field_len_q16(
    field_doc_lengths: &BTreeMap<String, BTreeMap<u32, u32>>,
    field: &str,
    allowed: &BTreeSet<u32>,
    all_allowed: bool,
) -> u64 {
    let Some(lengths) = field_doc_lengths.get(field) else {
        return 65_536;
    };
    let mut count = 0u64;
    let mut total = 0u64;
    for (candidate, length) in lengths {
        if all_allowed || allowed.contains(candidate) {
            count += 1;
            total = total.saturating_add(u64::from((*length).max(1)));
        }
    }
    total
        .saturating_mul(65_536)
        .checked_div(count)
        .unwrap_or(65_536)
}
