use std::collections::{BTreeMap, BTreeSet};

use crate::context::dedup::extract_project_metric_value;
use crate::context::{ContextPackAnomaly, ContextPackAnomalyCode, ContextPackCell};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ContextConflictVisibility {
    pub visible_conflict_count: u32,
    pub conflict_visibility_q16: u16,
}

pub(crate) fn measure(cells: &[ContextPackCell]) -> ContextConflictVisibility {
    let mut values_by_key: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
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
            .insert(value);
    }

    let visible_conflict_count = values_by_key
        .values()
        .filter(|values| values.len() > 1)
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
