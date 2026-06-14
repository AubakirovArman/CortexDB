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
        Self {
            records: records
                .into_iter()
                .map(|record| (record.cell_id, record))
                .collect(),
        }
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
        if let Some(record) = record {
            self.records.insert(cell_id, record);
        } else {
            self.records.remove(&cell_id);
        }
    }

    pub fn apply_tombstone(&mut self, cell_id: CellId) {
        self.records.remove(&cell_id);
    }

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

        for record in self.visible_records(view) {
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
