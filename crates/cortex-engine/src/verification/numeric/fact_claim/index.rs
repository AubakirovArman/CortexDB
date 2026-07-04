//! Numeric-fact index structures (moved from fact_claim.rs; behavior unchanged).

use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::AgentView;
use cortex_core::CellId;

use crate::plan::PolicyRewrite;
use crate::query::scope_id;
use crate::search::tokenize;
use crate::typed_body::FactBody;

use super::super::{
    compare_numeric_values, extract_numeric_values, NumericComparison, NumericValue,
};
use super::{normalized_text, text_terms_intersect, NumericFactRecord};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct NumericFactIndex {
    pub(super) by_metric: BTreeMap<MetricIndexKey, BTreeMap<NumericValueKey, BTreeSet<CellId>>>,
    pub(super) metric_terms: BTreeMap<String, BTreeSet<MetricIndexKey>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct MetricIndexKey {
    scope: String,
    metric: String,
    project: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct NumericValueKey {
    scaled_value: u64,
    currency: Option<String>,
    unit: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub(super) struct NumericFactQuery {
    metric: Option<String>,
    project: Option<String>,
    terms: BTreeSet<String>,
}

impl NumericFactIndex {
    pub(super) fn insert(&mut self, record: &NumericFactRecord) {
        let metric_key = MetricIndexKey::from_record(record);
        let value_key = NumericValueKey::from_value(&record.value);
        self.by_metric
            .entry(metric_key.clone())
            .or_default()
            .entry(value_key)
            .or_default()
            .insert(record.cell_id);
        for term in tokenize(&metric_key.metric) {
            self.metric_terms
                .entry(term)
                .or_default()
                .insert(metric_key.clone());
        }
    }

    pub(super) fn remove(&mut self, record: &NumericFactRecord) {
        let metric_key = MetricIndexKey::from_record(record);
        let value_key = NumericValueKey::from_value(&record.value);
        if let Some(value_buckets) = self.by_metric.get_mut(&metric_key) {
            let remove_bucket = if let Some(ids) = value_buckets.get_mut(&value_key) {
                ids.remove(&record.cell_id);
                ids.is_empty()
            } else {
                false
            };
            if remove_bucket {
                value_buckets.remove(&value_key);
            }
            if value_buckets.is_empty() {
                self.by_metric.remove(&metric_key);
            }
        }
        if !self.by_metric.contains_key(&metric_key) {
            for term in tokenize(&metric_key.metric) {
                let remove_term = if let Some(keys) = self.metric_terms.get_mut(&term) {
                    keys.remove(&metric_key);
                    keys.is_empty()
                } else {
                    false
                };
                if remove_term {
                    self.metric_terms.remove(&term);
                }
            }
        }
    }

    pub(super) fn matching_metric_keys(
        &self,
        query: &NumericFactQuery,
        view: &AgentView,
    ) -> Vec<MetricIndexKey> {
        let mut candidates = BTreeSet::new();
        if let Some(metric) = &query.metric {
            for term in tokenize(metric) {
                if let Some(keys) = self.metric_terms.get(&term) {
                    candidates.extend(keys.iter().filter(|key| &key.metric == metric).cloned());
                }
            }
        } else {
            for term in &query.terms {
                if let Some(keys) = self.metric_terms.get(term) {
                    candidates.extend(keys.iter().cloned());
                }
            }
        }

        candidates.retain(|key| PolicyRewrite::allows_scope(view, scope_id(&key.scope)));
        if let Some(project) = &query.project {
            candidates.retain(|key| key.project.as_ref() == Some(project));
        } else {
            let project_matches = candidates
                .iter()
                .filter(|key| {
                    key.project
                        .as_ref()
                        .map(|project| text_terms_intersect(project, &query.terms))
                        .unwrap_or(false)
                })
                .cloned()
                .collect::<BTreeSet<_>>();
            if !project_matches.is_empty() {
                candidates = project_matches;
            } else {
                candidates.retain(|key| key.project.is_none());
            }
        }
        candidates.into_iter().collect()
    }
}

impl MetricIndexKey {
    fn from_record(record: &NumericFactRecord) -> Self {
        Self {
            scope: record.scope.trim().to_owned(),
            metric: normalized_text(&record.metric).unwrap_or_default(),
            project: record.project.as_deref().and_then(normalized_text),
        }
    }
}

impl NumericValueKey {
    pub(super) fn from_value(value: &NumericValue) -> Self {
        Self {
            scaled_value: value.scaled_value,
            currency: value.currency.as_deref().and_then(normalized_text),
            unit: value.unit.as_deref().and_then(normalized_text),
        }
    }

    pub(super) fn is_comparable_with(&self, value: &NumericValue) -> bool {
        match (
            &self.currency,
            value.currency.as_deref().and_then(normalized_text),
        ) {
            (Some(_), Some(_)) => return true,
            (Some(_), None) | (None, Some(_)) => return false,
            (None, None) => {}
        }
        match (&self.unit, value.unit.as_deref().and_then(normalized_text)) {
            (Some(_), Some(_)) => {
                compare_numeric_values(&self.as_numeric_value(), value)
                    != NumericComparison::Incomparable
            }
            (Some(_), None) | (None, Some(_)) => false,
            (None, None) => true,
        }
    }

    fn as_numeric_value(&self) -> NumericValue {
        NumericValue {
            raw: String::new(),
            scaled_value: self.scaled_value,
            currency: self.currency.clone(),
            unit: self.unit.clone(),
            magnitude: None,
        }
    }
}

impl NumericFactQuery {
    pub(super) fn from_fact(fact: &str) -> Self {
        let body = FactBody::parse(fact.as_bytes());
        Self {
            metric: body.metric.as_deref().and_then(normalized_text),
            project: body.project.as_deref().and_then(normalized_text),
            terms: tokenize(fact)
                .into_iter()
                .filter(|term| extract_numeric_values(term).is_empty())
                .collect(),
        }
    }
}
