use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::{AgentView, ScopeId};
use cortex_core::memtable::{MemTable, ReadTxn};
use cortex_core::{CellDescriptor, CellId};

use crate::plan::PolicyRewrite;
use crate::query::{scope_id, CellMetadata};
use crate::source_trust::SourceTrust;
use crate::verification::numeric::fact_claim::{
    citation_source_key, same_source_ref, FactClaimStore, NumericFactRecord,
};
use crate::verification::numeric::{format_scaled_value, normalized_numeric_equal};
use crate::verification::VerificationNumericConflictKind;

use super::{
    conflict_facets_from_metadata, contradiction_facts, contradiction_relation_record,
    ConflictFacets, ConflictRecord,
};
use crate::source_trust::SourceTrustCategory;
use crate::typed_body::RelationBody;
use crate::verification::temporal::TemporalValidity;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ConflictIndexStore {
    inline_records: BTreeMap<CellId, Vec<StoredConflictRecord>>,
    relation_records: BTreeMap<CellId, StoredRelationRecord>,
    facets_by_cell: BTreeMap<CellId, StoredConflictFacets>,
    numeric_facts: BTreeMap<CellId, Vec<NumericFactRecord>>,
    numeric_records: Vec<StoredConflictRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StoredConflictRecord {
    scope_id: ScopeId,
    record: ConflictRecord,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StoredRelationRecord {
    scope_id: ScopeId,
    relation_cell_id: CellId,
    payload: Vec<u8>,
    relation_facets: ConflictFacets,
    source_trust_q16: u16,
    source_trust_category: SourceTrustCategory,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct StoredConflictFacets {
    scope_id: ScopeId,
    facets: ConflictFacets,
}

impl ConflictIndexStore {
    pub(crate) fn from_memtable(memtable: &MemTable, txn: ReadTxn) -> Self {
        let mut store = Self::default();
        for version in memtable.visible_iter(txn) {
            store.apply_record_inner(
                version.cell_id,
                &version.payload,
                &version.descriptor,
                false,
            );
        }
        store.rebuild_numeric_records();
        store
    }

    pub(crate) fn apply_record(
        &mut self,
        cell_id: CellId,
        payload: &[u8],
        descriptor: &CellDescriptor,
    ) {
        self.apply_record_inner(cell_id, payload, descriptor, true);
    }

    fn apply_record_inner(
        &mut self,
        cell_id: CellId,
        payload: &[u8],
        descriptor: &CellDescriptor,
        rebuild_numeric: bool,
    ) {
        self.apply_tombstone(cell_id);
        let metadata = CellMetadata::from_payload_with_descriptor(payload, descriptor);
        let record_scope_id = scope_id(&metadata.scope);
        let trust =
            SourceTrust::from_metadata(metadata.source_trust_q16, metadata.source_trust_class);
        let facets = conflict_facets_from_metadata(metadata.clone());
        self.facets_by_cell.insert(
            cell_id,
            StoredConflictFacets {
                scope_id: record_scope_id,
                facets: facets.clone(),
            },
        );

        let records = contradiction_facts(payload)
            .into_iter()
            .map(|fact| StoredConflictRecord {
                scope_id: record_scope_id,
                record: ConflictRecord {
                    cell_id,
                    relation_cell_id: None,
                    source_cell_id: Some(cell_id),
                    fact,
                    entity: facets.entity.clone(),
                    metric: facets.metric.clone(),
                    source: facets.source.clone(),
                    source_trust_q16: trust.q16,
                    source_trust_category: trust.category,
                },
            })
            .collect::<Vec<_>>();
        if !records.is_empty() {
            self.inline_records.insert(cell_id, records);
        }

        if let Some(relation) = relation_record(
            cell_id,
            payload,
            record_scope_id,
            trust.q16,
            trust.category,
            facets,
        ) {
            self.relation_records.insert(cell_id, relation);
        }

        let mut numeric_records = Vec::new();
        for record in FactClaimStore::records_from_payload(cell_id, payload, descriptor) {
            numeric_records.push(record);
        }
        if !numeric_records.is_empty() {
            self.numeric_facts.insert(cell_id, numeric_records);
        }
        if rebuild_numeric {
            self.rebuild_numeric_records();
        }
    }

    pub(crate) fn apply_tombstone(&mut self, cell_id: CellId) {
        let had_numeric = self.numeric_facts.remove(&cell_id).is_some();
        self.inline_records.remove(&cell_id);
        self.relation_records.remove(&cell_id);
        self.facets_by_cell.remove(&cell_id);
        if had_numeric {
            self.rebuild_numeric_records();
        }
    }

    pub(crate) fn records(&self, view: &AgentView) -> Vec<ConflictRecord> {
        let visible_facets = self.visible_facets(view);
        let mut records = Vec::new();
        records.extend(self.visible_stored_records(self.inline_records.values().flatten(), view));
        records.extend(self.visible_stored_records(self.numeric_records.iter(), view));
        for relation in self.relation_records.values() {
            if !PolicyRewrite::allows_scope(view, relation.scope_id) {
                continue;
            }
            if let Some(record) = contradiction_relation_record(
                relation.relation_cell_id,
                &relation.payload,
                relation.source_trust_q16,
                relation.source_trust_category,
                relation.relation_facets.clone(),
                &visible_facets,
            ) {
                records.push(record);
            }
        }
        sort_and_dedup(&mut records);
        records
    }

    fn visible_stored_records<'a>(
        &self,
        records: impl Iterator<Item = &'a StoredConflictRecord>,
        view: &AgentView,
    ) -> Vec<ConflictRecord> {
        records
            .filter(|record| PolicyRewrite::allows_scope(view, record.scope_id))
            .map(|record| record.record.clone())
            .collect()
    }

    fn visible_facets(&self, view: &AgentView) -> BTreeMap<CellId, ConflictFacets> {
        self.facets_by_cell
            .iter()
            .filter(|(_, facets)| PolicyRewrite::allows_scope(view, facets.scope_id))
            .map(|(cell_id, facets)| (*cell_id, facets.facets.clone()))
            .collect()
    }

    fn rebuild_numeric_records(&mut self) {
        let mut records = Vec::new();
        let mut seen_pairs = BTreeSet::new();
        let mut groups = BTreeMap::<NumericConflictGroupKey, Vec<&NumericFactRecord>>::new();
        for fact in self.numeric_facts.values().flatten() {
            groups
                .entry(NumericConflictGroupKey::from_record(fact))
                .or_default()
                .push(fact);
        }
        for facts in groups.values() {
            for (left_index, left) in facts.iter().enumerate() {
                for right in facts.iter().skip(left_index + 1) {
                    if left.cell_id == right.cell_id {
                        continue;
                    }
                    if !temporal_windows_overlap(left.temporal_validity, right.temporal_validity) {
                        continue;
                    }
                    if normalized_numeric_equal(&left.value, &right.value)
                        || !left.value.conflicts_with(&right.value)
                    {
                        continue;
                    }
                    let pair = ordered_pair(left.cell_id, right.cell_id);
                    if seen_pairs.insert(pair) {
                        let kind = numeric_conflict_record_kind(left, right);
                        records.push(numeric_conflict_record(left, right, kind));
                    }
                }
            }
        }
        records.sort_by_key(|record| {
            (
                record.record.fact.clone(),
                record.record.cell_id,
                record.record.source_cell_id.unwrap_or(CellId(0)),
            )
        });
        self.numeric_records = records;
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct NumericConflictGroupKey {
    scope: String,
    metric: String,
    project: Option<String>,
}

impl NumericConflictGroupKey {
    fn from_record(record: &NumericFactRecord) -> Self {
        Self {
            scope: record.scope.trim().to_owned(),
            metric: record.metric.trim().to_ascii_lowercase(),
            project: normalized_opt(&record.project),
        }
    }
}

fn relation_record(
    relation_cell_id: CellId,
    payload: &[u8],
    scope_id: ScopeId,
    source_trust_q16: u16,
    source_trust_category: SourceTrustCategory,
    relation_facets: ConflictFacets,
) -> Option<StoredRelationRecord> {
    let relation = RelationBody::parse(payload);
    let predicate = relation.predicate.as_deref()?;
    predicate
        .trim()
        .eq_ignore_ascii_case("contradicts")
        .then(|| StoredRelationRecord {
            scope_id,
            relation_cell_id,
            payload: payload.to_vec(),
            relation_facets,
            source_trust_q16,
            source_trust_category,
        })
}

fn numeric_conflict_record(
    left: &NumericFactRecord,
    right: &NumericFactRecord,
    kind: VerificationNumericConflictKind,
) -> StoredConflictRecord {
    let (left, right) = if left.cell_id <= right.cell_id {
        (left, right)
    } else {
        (right, left)
    };
    let project = left.project.clone().or_else(|| right.project.clone());
    let source = left.source.clone().or_else(|| right.source.clone());
    let prefix = match kind {
        VerificationNumericConflictKind::Citation => "citation conflict: ",
        VerificationNumericConflictKind::Temporal => "temporal conflict: ",
        VerificationNumericConflictKind::Numeric => "",
    };
    let suffix = match kind {
        VerificationNumericConflictKind::Citation => citation_source_key(left)
            .map(|_| " from same source_ref")
            .unwrap_or(""),
        VerificationNumericConflictKind::Temporal => " over overlapping validity window",
        VerificationNumericConflictKind::Numeric => "",
    };
    let fact = format!(
        "{prefix}{}{}{} conflicts: {} vs {}{suffix}",
        project.as_deref().unwrap_or(""),
        project.as_ref().map(|_| " ").unwrap_or(""),
        left.metric,
        display_numeric_value(left),
        display_numeric_value(right)
    );
    StoredConflictRecord {
        scope_id: scope_id(&left.scope),
        record: ConflictRecord {
            cell_id: left.cell_id,
            relation_cell_id: None,
            source_cell_id: Some(right.cell_id),
            fact,
            entity: project,
            metric: Some(left.metric.clone()),
            source,
            source_trust_q16: left.source_trust_q16.min(right.source_trust_q16),
            source_trust_category: left.source_trust_category,
        },
    }
}

fn numeric_conflict_record_kind(
    left: &NumericFactRecord,
    right: &NumericFactRecord,
) -> VerificationNumericConflictKind {
    if same_source_ref(left, right) {
        VerificationNumericConflictKind::Citation
    } else if !left.temporal_validity.is_empty() || !right.temporal_validity.is_empty() {
        VerificationNumericConflictKind::Temporal
    } else {
        VerificationNumericConflictKind::Numeric
    }
}

fn display_numeric_value(record: &NumericFactRecord) -> String {
    format_scaled_value(
        record.value.scaled_value,
        record.value.currency.as_deref(),
        record.value.unit.as_deref(),
    )
}

fn temporal_windows_overlap(left: TemporalValidity, right: TemporalValidity) -> bool {
    left.overlaps(right)
}

fn normalized_opt(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
}

fn ordered_pair(left: CellId, right: CellId) -> (CellId, CellId) {
    if left <= right {
        (left, right)
    } else {
        (right, left)
    }
}

fn sort_and_dedup(records: &mut Vec<ConflictRecord>) {
    records.sort_by_key(|record| {
        (
            record.fact.clone(),
            record.cell_id,
            record.relation_cell_id.unwrap_or(CellId(0)),
            record.source_cell_id.unwrap_or(CellId(0)),
        )
    });
    records.dedup_by(|left, right| {
        left.cell_id == right.cell_id
            && left.relation_cell_id == right.relation_cell_id
            && left.source_cell_id == right.source_cell_id
            && left.fact == right.fact
    });
}
