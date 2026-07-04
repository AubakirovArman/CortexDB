use std::collections::BTreeMap;
use std::collections::BTreeSet;

use cortex_aql::AgentView;
use cortex_core::memtable::{MemTable, ReadTxn};
use cortex_core::{CellDescriptor, CellId};

use crate::query::{CellMetadata, SourceRef};
use crate::search::tokenize;
use crate::source_trust::{SourceTrust, SourceTrustCategory};
use crate::typed_body::FactBody;
// Used only by the `#[cfg(test)]` `visible_records` helper below.
#[cfg(test)]
use crate::plan::PolicyRewrite;
#[cfg(test)]
use crate::query::scope_id;

use super::super::temporal::{
    extract_temporal_query_range, temporal_validity_from_metadata, TemporalQueryRange,
    TemporalValidity,
};
use super::super::{
    VerificationEvidence, VerificationMatchKind, VerificationNumericConflict,
    VerificationNumericConflictKind,
};
use super::{
    extract_numeric_values, normalized_numeric_equal, numeric_conflict, parse_currency_code,
    NumericValue,
};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FactClaimStore {
    records: BTreeMap<CellId, Vec<NumericFactRecord>>,
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
    pub(crate) source_ref: Option<SourceRef>,
    pub(crate) temporal_validity: TemporalValidity,
    pub(crate) source_trust_q16: u16,
    pub(crate) source_trust_category: SourceTrustCategory,
}

impl FactClaimStore {
    pub fn from_memtable(memtable: &MemTable, txn: ReadTxn) -> Self {
        Self::from_records(memtable.visible_iter(txn).flat_map(|version| {
            Self::records_from_payload(version.cell_id, &version.payload, &version.descriptor)
        }))
    }

    pub fn from_records(records: impl IntoIterator<Item = NumericFactRecord>) -> Self {
        let mut store = Self::default();
        for record in records {
            store.insert_record(record);
        }
        store
    }

    pub fn records_from_payload(
        cell_id: CellId,
        payload: &[u8],
        descriptor: &CellDescriptor,
    ) -> Vec<NumericFactRecord> {
        let metadata = CellMetadata::from_payload_with_descriptor(payload, descriptor);
        let body = FactBody::parse(metadata.body_text.as_bytes());
        let Some(metric) = body.metric else {
            return Vec::new();
        };
        let values = body
            .value
            .as_deref()
            .map(|value| explicit_numeric_values(value, body.currency.as_deref()))
            .map(prefer_contextual_numeric_values)
            .unwrap_or_else(|| contextual_numeric_values(&metadata.body_text, &metric));
        if values.is_empty() {
            return Vec::new();
        }
        let source_trust =
            SourceTrust::from_metadata(metadata.source_trust_q16, metadata.source_trust_class);
        let citation = metadata.citation().map(str::to_owned);
        let source = metadata
            .source
            .clone()
            .or_else(|| metadata.citation.clone())
            .or_else(|| {
                metadata
                    .source_ref
                    .as_ref()
                    .map(|source_ref| source_ref.source_id.clone())
            });
        let source_ref = metadata.source_ref.clone();
        let temporal_validity = temporal_validity_from_metadata(&metadata);
        let project = body.project.or(metadata.project);
        values
            .into_iter()
            .map(|value| NumericFactRecord {
                cell_id,
                scope: metadata.scope.clone(),
                metric: metric.clone(),
                value,
                project: project.clone(),
                source: source.clone(),
                citation: citation.clone(),
                source_ref: source_ref.clone(),
                temporal_validity,
                source_trust_q16: source_trust.q16,
                source_trust_category: source_trust.category,
            })
            .collect()
    }

    pub fn apply_records(&mut self, cell_id: CellId, records: Vec<NumericFactRecord>) {
        self.apply_tombstone(cell_id);
        for record in records {
            self.insert_record(record);
        }
    }

    pub fn apply_tombstone(&mut self, cell_id: CellId) {
        if let Some(records) = self.records.remove(&cell_id) {
            for record in records {
                self.index.remove(&record);
            }
        }
    }

    #[cfg(test)]
    pub fn visible_records(&self, view: &AgentView) -> Vec<NumericFactRecord> {
        self.records
            .values()
            .flatten()
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

        let temporal_query = extract_temporal_query_range(fact);
        let indexed_records = self.indexed_records_for_fact(fact, view, &fact_values);
        for record in indexed_records.iter().cloned() {
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
            let kind =
                citation_conflict_kind(&record, &indexed_records, &fact_values, temporal_query);
            if kind != VerificationNumericConflictKind::Numeric {
                if let Some(existing) = numeric_conflicts
                    .iter_mut()
                    .find(|item| item.cell_id == record.cell_id)
                {
                    existing.kind = kind;
                    continue;
                }
            }
            if conflict_seen.insert(record.cell_id) {
                numeric_conflicts.push(VerificationNumericConflict {
                    cell_id: record.cell_id,
                    kind,
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
        self.records.entry(record.cell_id).or_default().push(record);
    }

    fn indexed_records_for_fact(
        &self,
        fact: &str,
        view: &AgentView,
        fact_values: &[NumericValue],
    ) -> Vec<NumericFactRecord> {
        let query = NumericFactQuery::from_fact(fact);
        let temporal_query = extract_temporal_query_range(fact);
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
            .filter_map(|cell_id| self.records.get(&cell_id))
            .flat_map(|records| records.iter().cloned())
            .filter(|record| {
                fact_values.iter().any(|fact_value| {
                    NumericValueKey::from_value(&record.value).is_comparable_with(fact_value)
                }) && record_matches_temporal_query(record, temporal_query)
            })
            .collect()
    }
}

mod index;
use index::{NumericFactIndex, NumericFactQuery, NumericValueKey};
fn contextual_numeric_values(text: &str, metric: &str) -> Vec<NumericValue> {
    let metric_terms = tokenize(metric);
    let mut values = Vec::new();
    for segment in text.split(['\n', ';']) {
        if is_contradiction_marker_segment(segment) {
            continue;
        }
        let terms = tokenize(segment);
        if metric_terms.iter().any(|term| terms.contains(term)) {
            values.extend(extract_numeric_values(segment));
        }
    }
    if values.is_empty() {
        values = extract_numeric_values(text);
    }
    prefer_contextual_numeric_values(values)
}

fn is_contradiction_marker_segment(segment: &str) -> bool {
    segment
        .trim_start()
        .to_ascii_lowercase()
        .starts_with("contradicts=")
}

fn explicit_numeric_values(value: &str, currency: Option<&str>) -> Vec<NumericValue> {
    let mut values = extract_numeric_values(value);
    let currency = currency.and_then(parse_currency_code);
    if let Some(currency) = currency {
        for value in &mut values {
            if value.currency.is_none() && value.unit.is_none() {
                value.currency = Some(currency.clone());
            }
        }
    }
    values
}

fn prefer_contextual_numeric_values(values: Vec<NumericValue>) -> Vec<NumericValue> {
    if values
        .iter()
        .any(|value| value.currency.is_some() || value.unit.is_some())
    {
        return values
            .into_iter()
            .filter(|value| value.currency.is_some() || value.unit.is_some())
            .collect();
    }
    values
}

fn record_matches_temporal_query(
    record: &NumericFactRecord,
    query: Option<TemporalQueryRange>,
) -> bool {
    query
        .map(|query| temporal_window_overlaps_query(record.temporal_validity, query))
        .unwrap_or(true)
}

fn citation_conflict_kind(
    record: &NumericFactRecord,
    records: &[NumericFactRecord],
    fact_values: &[NumericValue],
    temporal_query: Option<TemporalQueryRange>,
) -> VerificationNumericConflictKind {
    if records.iter().any(|candidate| {
        candidate.cell_id != record.cell_id
            && same_source_ref(record, candidate)
            && !normalized_numeric_equal(&candidate.value, &record.value)
            && candidate.value.conflicts_with(&record.value)
            && fact_values
                .iter()
                .any(|fact_value| normalized_numeric_equal(fact_value, &candidate.value))
    }) {
        VerificationNumericConflictKind::Citation
    } else if temporal_query.is_some() && !record.temporal_validity.is_empty() {
        VerificationNumericConflictKind::Temporal
    } else {
        VerificationNumericConflictKind::Numeric
    }
}

pub(crate) fn same_source_ref(left: &NumericFactRecord, right: &NumericFactRecord) -> bool {
    citation_source_key(left).is_some_and(|left_key| {
        citation_source_key(right).is_some_and(|right_key| left_key == right_key)
    })
}

pub(crate) fn citation_source_key(record: &NumericFactRecord) -> Option<CitationSourceKey> {
    CitationSourceKey::from_source_ref(record.source_ref.as_ref()?)
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct CitationSourceKey {
    source_id: String,
    source_url: Option<String>,
    document_id: Option<String>,
    page: Option<u32>,
    row: Option<u32>,
    cell_range: Option<String>,
    json_path: Option<String>,
}

impl CitationSourceKey {
    fn from_source_ref(source_ref: &SourceRef) -> Option<Self> {
        if source_ref.source_url.is_none()
            && source_ref.document_id.is_none()
            && source_ref.page.is_none()
            && source_ref.row.is_none()
            && source_ref.cell_range.is_none()
            && source_ref.json_path.is_none()
        {
            return None;
        }
        Some(Self {
            source_id: normalized_source_text(&source_ref.source_id)?,
            source_url: normalized_source_opt(&source_ref.source_url),
            document_id: normalized_source_opt(&source_ref.document_id),
            page: source_ref.page,
            row: source_ref.row,
            cell_range: normalized_source_opt(&source_ref.cell_range),
            json_path: normalized_source_opt(&source_ref.json_path),
        })
    }
}

fn temporal_window_overlaps_query(
    temporal_validity: TemporalValidity,
    query: TemporalQueryRange,
) -> bool {
    temporal_validity.overlaps_query(query)
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

fn normalized_source_opt(value: &Option<String>) -> Option<String> {
    value.as_deref().and_then(normalized_source_text)
}

fn normalized_source_text(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_ascii_lowercase())
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
