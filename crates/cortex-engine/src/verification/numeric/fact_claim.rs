use std::collections::BTreeMap;
use std::collections::BTreeSet;

use cortex_aql::AgentView;
use cortex_core::memtable::{MemTable, ReadTxn};
use cortex_core::{CellDescriptor, CellId};

use crate::plan::PolicyRewrite;
use crate::query::{scope_id, CellMetadata};
use crate::search::tokenize;
use crate::source_trust::{SourceTrust, SourceTrustCategory};
use crate::typed_body::FactBody;

use super::super::temporal::extract_temporal_query_range;
use super::super::{VerificationEvidence, VerificationMatchKind, VerificationNumericConflict};
use super::{extract_numeric_values, normalized_numeric_equal, numeric_conflict, NumericValue};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FactClaimStore {
    records: BTreeMap<CellId, NumericFactRecord>,
    index: NumericFactIndex,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NumericFactRecord {
    pub(crate) cell_id: CellId,
    pub(crate) scope: String,
    pub(crate) metric: String,
    pub(crate) value: NumericValue,
    pub(crate) project: Option<String>,
    pub(crate) source: Option<String>,
    pub(crate) citation: Option<String>,
    pub(crate) source_trust_q16: u16,
    pub(crate) source_trust_category: SourceTrustCategory,
}

impl FactClaimStore {
    pub fn from_memtable(memtable: &MemTable, txn: ReadTxn) -> Self {
        Self::from_records(memtable.visible_iter(txn).filter_map(|version| {
            Self::record_from_payload(version.cell_id, &version.payload, &version.descriptor)
        }))
    }

    pub fn from_records(records: impl IntoIterator<Item = NumericFactRecord>) -> Self {
        let mut store = Self::default();
        for record in records {
            store.insert_record(record);
        }
        store
    }

    pub fn record_from_payload(
        cell_id: CellId,
        payload: &[u8],
        descriptor: &CellDescriptor,
    ) -> Option<NumericFactRecord> {
        let metadata = CellMetadata::from_payload_with_descriptor(payload, descriptor);
        let body = FactBody::parse(metadata.body_text.as_bytes());
        let metric = body.metric?;
        let values = body
            .value
            .as_deref()
            .map(extract_numeric_values)
            .unwrap_or_else(|| extract_numeric_values(&metadata.body_text));
        let value = single_numeric_value(values)?;
        let source_trust =
            SourceTrust::from_metadata(metadata.source_trust_q16, metadata.source_trust_class);
        let citation = metadata.citation().map(str::to_owned);
        Some(NumericFactRecord {
            cell_id,
            scope: metadata.scope,
            metric,
            value,
            project: body.project.or(metadata.project),
            source: metadata.source.or(metadata.citation),
            citation,
            source_trust_q16: source_trust.q16,
            source_trust_category: source_trust.category,
        })
    }

    pub fn apply_record(&mut self, cell_id: CellId, record: Option<NumericFactRecord>) {
        self.apply_tombstone(cell_id);
        if let Some(record) = record {
            self.insert_record(record);
        }
    }

    pub fn apply_tombstone(&mut self, cell_id: CellId) {
        if let Some(record) = self.records.remove(&cell_id) {
            self.index.remove(&record);
        }
    }

    #[cfg(test)]
    pub fn visible_records(&self, view: &AgentView) -> Vec<NumericFactRecord> {
        self.records
            .values()
            .filter(|record| PolicyRewrite::allows_scope(view, scope_id(&record.scope)))
            .cloned()
            .collect()
    }

    pub(crate) fn add_verify_matches(
        &self,
        fact: &str,
        view: &AgentView,
        evidence: &mut Vec<VerificationEvidence>,
        contradicting_evidence: &mut Vec<VerificationEvidence>,
        numeric_conflicts: &mut Vec<VerificationNumericConflict>,
    ) {
        if extract_temporal_query_range(fact).is_some() {
            return;
        }
        let fact_values = extract_numeric_values(fact);
        if fact_values.is_empty() {
            return;
        }
        let mut support_seen = evidence
            .iter()
            .map(|item| item.cell_id)
            .collect::<BTreeSet<_>>();
        let mut contradiction_seen = contradicting_evidence
            .iter()
            .map(|item| item.cell_id)
            .collect::<BTreeSet<_>>();
        let mut conflict_seen = numeric_conflicts
            .iter()
            .map(|item| item.cell_id)
            .collect::<BTreeSet<_>>();

        for record in self.indexed_records_for_fact(fact, view, &fact_values) {
            let Some(matched_terms) = typed_claim_matched_terms(fact, &record) else {
                continue;
            };
            let evidence_item = || typed_claim_evidence(&record, matched_terms);
            if fact_values
                .iter()
                .any(|fact_value| normalized_numeric_equal(fact_value, &record.value))
            {
                if support_seen.insert(record.cell_id) {
                    evidence.push(evidence_item());
                }
                continue;
            }

            let Some(fact_value) = fact_values
                .iter()
                .find(|fact_value| numeric_conflict(fact_value, &record.value))
            else {
                continue;
            };
            if contradiction_seen.insert(record.cell_id) {
                let mut item = evidence_item();
                item.match_score_q16 = u16::MAX;
                item.match_kind = VerificationMatchKind::NumericContradiction;
                contradicting_evidence.push(item);
            }
            if conflict_seen.insert(record.cell_id) {
                numeric_conflicts.push(VerificationNumericConflict {
                    cell_id: record.cell_id,
                    metric: record.metric.clone(),
                    left: numeric_display(fact_value),
                    right: numeric_display(&record.value),
                    fact_value: fact_value.clone(),
                    evidence_value: record.value.clone(),
                });
            }
        }
    }

    pub(crate) fn indexed_cell_ids_for_fact(&self, fact: &str, view: &AgentView) -> Vec<CellId> {
        if extract_temporal_query_range(fact).is_some() {
            return Vec::new();
        }
        let fact_values = extract_numeric_values(fact);
        if fact_values.is_empty() {
            return Vec::new();
        }
        self.indexed_records_for_fact(fact, view, &fact_values)
            .into_iter()
            .map(|record| record.cell_id)
            .collect()
    }

    fn insert_record(&mut self, record: NumericFactRecord) {
        self.index.insert(&record);
        self.records.insert(record.cell_id, record);
    }

    fn indexed_records_for_fact(
        &self,
        fact: &str,
        view: &AgentView,
        fact_values: &[NumericValue],
    ) -> Vec<NumericFactRecord> {
        let query = NumericFactQuery::from_fact(fact);
        let mut cell_ids = BTreeSet::new();
        for metric_key in self.index.matching_metric_keys(&query, view) {
            let Some(value_buckets) = self.index.by_metric.get(&metric_key) else {
                continue;
            };
            for (value_key, ids) in value_buckets {
                if fact_values
                    .iter()
                    .any(|fact_value| value_key.is_comparable_with(fact_value))
                {
                    cell_ids.extend(ids.iter().copied());
                }
            }
        }
        cell_ids
            .into_iter()
            .filter_map(|cell_id| self.records.get(&cell_id).cloned())
            .collect()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct NumericFactIndex {
    by_metric: BTreeMap<MetricIndexKey, BTreeMap<NumericValueKey, BTreeSet<CellId>>>,
    metric_terms: BTreeMap<String, BTreeSet<MetricIndexKey>>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct MetricIndexKey {
    scope: String,
    metric: String,
    project: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct NumericValueKey {
    scaled_value: u64,
    currency: Option<String>,
    unit: Option<String>,
}

#[derive(Clone, Debug, Default)]
struct NumericFactQuery {
    metric: Option<String>,
    project: Option<String>,
    terms: BTreeSet<String>,
}

impl NumericFactIndex {
    fn insert(&mut self, record: &NumericFactRecord) {
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

    fn remove(&mut self, record: &NumericFactRecord) {
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

    fn matching_metric_keys(
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
    fn from_value(value: &NumericValue) -> Self {
        Self {
            scaled_value: value.scaled_value,
            currency: value.currency.as_deref().and_then(normalized_text),
            unit: value.unit.as_deref().and_then(normalized_text),
        }
    }

    fn is_comparable_with(&self, value: &NumericValue) -> bool {
        match (
            &self.currency,
            value.currency.as_deref().and_then(normalized_text),
        ) {
            (Some(_), Some(_)) => return true,
            (Some(_), None) | (None, Some(_)) => return false,
            (None, None) => {}
        }
        match (&self.unit, value.unit.as_deref().and_then(normalized_text)) {
            (Some(left), Some(right)) => left == &right,
            (Some(_), None) | (None, Some(_)) => false,
            (None, None) => true,
        }
    }
}

impl NumericFactQuery {
    fn from_fact(fact: &str) -> Self {
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

fn single_numeric_value(values: Vec<NumericValue>) -> Option<NumericValue> {
    let mut values = values.into_iter();
    let value = values.next()?;
    values.next().is_none().then_some(value)
}

fn typed_claim_evidence(record: &NumericFactRecord, matched_terms: u32) -> VerificationEvidence {
    VerificationEvidence {
        cell_id: record.cell_id,
        matched_terms,
        match_score_q16: 62_000,
        match_kind: VerificationMatchKind::NumericEntailment,
        source_trust_q16: record.source_trust_q16,
        source_trust_category: record.source_trust_category,
        citation: record.citation.clone().or(record.source.clone()),
    }
}

fn typed_claim_matched_terms(fact: &str, record: &NumericFactRecord) -> Option<u32> {
    let fact_terms = tokenize(fact)
        .into_iter()
        .filter(|term| extract_numeric_values(term).is_empty())
        .collect::<Vec<_>>();
    if fact_terms.is_empty() {
        return None;
    }
    let record_text = format!(
        "{} {} {}",
        record.metric,
        record.project.as_deref().unwrap_or(""),
        record.source.as_deref().unwrap_or("")
    );
    let record_terms = tokenize(&record_text);
    let matched = fact_terms
        .iter()
        .filter(|term| record_terms.contains(term))
        .count();
    (matched > 0).then_some(matched as u32)
}

fn normalized_text(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_ascii_lowercase())
}

fn text_terms_intersect(value: &str, terms: &BTreeSet<String>) -> bool {
    tokenize(value).iter().any(|term| terms.contains(term))
}

fn numeric_display(value: &NumericValue) -> String {
    let context = value.currency.as_deref().or(value.unit.as_deref());
    let raw = if context == Some("%") {
        value.raw.trim_end_matches(['%', '.', ','])
    } else {
        value.raw.trim_end_matches(['.', ','])
    };
    match context {
        Some("%") => format!("{raw} %"),
        Some(context)
            if raw == context
                || raw.ends_with(context)
                || raw.to_ascii_uppercase().ends_with(context) =>
        {
            raw.to_owned()
        }
        Some(context) => format!("{raw} {context}"),
        None => raw.to_owned(),
    }
}

#[cfg(test)]
mod tests;
