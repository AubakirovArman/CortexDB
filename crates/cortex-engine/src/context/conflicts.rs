use std::collections::BTreeMap;

use crate::context::dedup::extract_project_metric_value;
use crate::context::{ContextPackAnomaly, ContextPackAnomalyCode, ContextPackCell};
use crate::verification::numeric::{compare_numeric_values, extract_numeric_values, NumericValue};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ContextConflictVisibility {
    pub visible_conflict_count: u32,
    pub conflict_visibility_q16: u16,
}

pub(crate) fn measure(cells: &[ContextPackCell]) -> ContextConflictVisibility {
    let mut values_by_key: BTreeMap<(String, String), Vec<ConflictValue>> = BTreeMap::new();
    for cell in cells {
        let (project, metric, value) = extract_project_metric_value(&cell.payload);
        let Some(project) = normalized(project) else {
            continue;
        };
        let Some(metric) = normalized(metric) else {
            continue;
        };
        let Some(value) = normalized(value) else {
            continue;
        };
        values_by_key
            .entry((project, metric))
            .or_default()
            .push(ConflictValue::from_raw(value));
    }

    let visible_conflict_count = values_by_key
        .values()
        .filter(|values| has_visible_conflict(values))
        .count()
        .try_into()
        .unwrap_or(u32::MAX);
    ContextConflictVisibility {
        visible_conflict_count,
        conflict_visibility_q16: conflict_intensity_q16(visible_conflict_count),
    }
}

pub(crate) fn anomaly(visibility: &ContextConflictVisibility) -> Option<ContextPackAnomaly> {
    (visibility.visible_conflict_count > 0).then(|| ContextPackAnomaly {
        cell_id: None,
        code: ContextPackAnomalyCode::VisibleConflict,
        message: format!(
            "context pack contains {} visible conflict group(s)",
            visibility.visible_conflict_count
        ),
        why_excluded: None,
    })
}

fn conflict_intensity_q16(visible_conflict_count: u32) -> u16 {
    if visible_conflict_count == 0 {
        return 0;
    }

    let count = u64::from(visible_conflict_count);
    ((count * u64::from(u16::MAX)) / (count + 1)) as u16
}

fn normalized(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ConflictValue {
    Numeric(NumericValue),
    Text(String),
}

impl ConflictValue {
    fn from_raw(value: String) -> Self {
        let numeric_values = extract_numeric_values(&value);
        if numeric_values.len() == 1 {
            return Self::Numeric(numeric_values[0].clone());
        }
        Self::Text(value)
    }
}

fn has_visible_conflict(values: &[ConflictValue]) -> bool {
    for (index, left) in values.iter().enumerate() {
        for right in values.iter().skip(index + 1) {
            if values_conflict(left, right) {
                return true;
            }
        }
    }
    false
}

fn values_conflict(left: &ConflictValue, right: &ConflictValue) -> bool {
    match (left, right) {
        (ConflictValue::Numeric(left), ConflictValue::Numeric(right)) => {
            compare_numeric_values(left, right).is_conflict()
        }
        (ConflictValue::Text(left), ConflictValue::Text(right)) => left != right,
        (ConflictValue::Numeric(_), ConflictValue::Text(_))
        | (ConflictValue::Text(_), ConflictValue::Numeric(_)) => false,
    }
}
