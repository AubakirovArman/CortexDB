use cortex_aql::{AgentView, Q16};
use cortex_core::{CellId, KnowledgeCell, KnowledgeCellMetadata, KnowledgeCellType};

use crate::database::Database;
use crate::error::EngineResult;
use crate::ingestion::IngestedCell;
use crate::query::CellMetadata;
use crate::search::tokenize;
use crate::source_trust::SourceTrustCategory;
use crate::typed_body::RelationBody;

use super::contradiction::{contradiction_facts, contradiction_text_matches};

mod store;

pub(crate) use store::ConflictIndexStore;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConflictRecord {
    pub cell_id: CellId,
    pub relation_cell_id: Option<CellId>,
    pub source_cell_id: Option<CellId>,
    pub fact: String,
    pub entity: Option<String>,
    pub metric: Option<String>,
    pub source: Option<String>,
    pub source_trust_q16: Q16,
    pub source_trust_category: SourceTrustCategory,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContradictionRelationOptions {
    pub scope: String,
    pub source: String,
    pub source_trust_q16: Option<Q16>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct ConflictFacets {
    entity: Option<String>,
    metric: Option<String>,
    source: Option<String>,
}

impl Database {
    pub fn conflict_index(&self, view: &AgentView) -> Vec<ConflictRecord> {
        self.conflict_index_store.records(view)
    }

    pub fn conflicts_for_fact(&self, fact: &str, view: &AgentView) -> Vec<ConflictRecord> {
        let fact_terms = tokenize(fact);
        self.conflict_index(view)
            .into_iter()
            .filter(|record| contradiction_text_matches(&record.fact, &fact_terms))
            .collect()
    }

    pub fn conflicts_for_entity(&self, entity: &str, view: &AgentView) -> Vec<ConflictRecord> {
        let query_terms = tokenize(entity);
        self.conflict_index(view)
            .into_iter()
            .filter(|record| record_matches(&record.entity, &record.fact, &query_terms))
            .collect()
    }

    pub fn conflicts_for_metric(&self, metric: &str, view: &AgentView) -> Vec<ConflictRecord> {
        let query_terms = tokenize(metric);
        self.conflict_index(view)
            .into_iter()
            .filter(|record| record_matches(&record.metric, &record.fact, &query_terms))
            .collect()
    }

    pub fn conflicts_for_source(&self, source: &str, view: &AgentView) -> Vec<ConflictRecord> {
        let query_terms = tokenize(source);
        self.conflict_index(view)
            .into_iter()
            .filter(|record| field_matches(record.source.as_deref(), &query_terms))
            .collect()
    }

    pub fn persist_contradiction_relation(
        &mut self,
        relation_cell_id: CellId,
        source_cell_id: CellId,
        contradicted_fact: &str,
        options: ContradictionRelationOptions,
    ) -> EngineResult<IngestedCell> {
        let cell = KnowledgeCell::new(
            KnowledgeCellMetadata {
                scope: options.scope,
                status: "ready".to_owned(),
                cell_type: KnowledgeCellType::Relation,
                memory_type: None,
                ttl_seconds: None,
                created_unix_seconds: None,
                source_trust_q16: options.source_trust_q16,
                source: Some(options.source),
            },
            contradiction_relation_body(source_cell_id, contradicted_fact),
        );
        let commit_seq = self.put_knowledge_cell(relation_cell_id, cell)?;
        Ok(IngestedCell {
            cell_id: relation_cell_id,
            commit_seq,
            chunk_id: None,
        })
    }
}

fn contradiction_relation_body(source_cell_id: CellId, contradicted_fact: &str) -> String {
    format!(
        "subject=cell:{}\npredicate=contradicts\nobject={}\nsource_cell_id={}",
        source_cell_id.0,
        sanitize_relation_value(contradicted_fact),
        source_cell_id.0
    )
}

pub(super) fn contradiction_relation_record(
    relation_cell_id: CellId,
    payload: &[u8],
    source_trust_q16: Q16,
    source_trust_category: SourceTrustCategory,
    relation_facets: ConflictFacets,
    visible_facets: &std::collections::BTreeMap<CellId, ConflictFacets>,
) -> Option<ConflictRecord> {
    let relation = RelationBody::parse(payload);
    let predicate = relation.predicate.as_deref()?;
    if !predicate.trim().eq_ignore_ascii_case("contradicts") {
        return None;
    }
    let source_cell_id = relation_source_cell_id(&relation);
    let source_facets = source_cell_id.and_then(|cell_id| visible_facets.get(&cell_id));
    let fact = relation.object?;
    Some(ConflictRecord {
        cell_id: source_cell_id.unwrap_or(relation_cell_id),
        relation_cell_id: Some(relation_cell_id),
        source_cell_id,
        fact,
        entity: relation_facets
            .entity
            .or_else(|| source_facets.and_then(|facets| facets.entity.clone())),
        metric: relation_facets
            .metric
            .or_else(|| source_facets.and_then(|facets| facets.metric.clone())),
        source: source_facets
            .and_then(|facets| facets.source.clone())
            .or(relation_facets.source),
        source_trust_q16,
        source_trust_category,
    })
}

pub(super) fn relation_source_cell_id(relation: &RelationBody) -> Option<CellId> {
    relation
        .subject
        .as_deref()
        .and_then(|subject| subject.strip_prefix("cell:"))
        .and_then(|value| value.trim().parse::<u64>().ok())
        .map(CellId)
}

fn sanitize_relation_value(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '\n' | '\r' => ' ',
            other => other,
        })
        .collect()
}

pub(super) fn conflict_facets_from_metadata(metadata: CellMetadata) -> ConflictFacets {
    ConflictFacets {
        entity: metadata_value(&metadata.body_text, &["entity", "project"])
            .or(metadata.entity)
            .or(metadata.project),
        metric: metadata_value(&metadata.body_text, &["metric"]),
        source: metadata.source.or_else(|| {
            metadata
                .source_ref
                .map(|source_ref| source_ref.source_id)
                .or(metadata.citation)
        }),
    }
}

fn metadata_value(body: &str, keys: &[&str]) -> Option<String> {
    body.lines().find_map(|line| {
        let (key, value) = line.split_once('=')?;
        keys.iter()
            .any(|candidate| key.trim().eq_ignore_ascii_case(candidate))
            .then(|| value.trim())
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
    })
}

fn record_matches(field: &Option<String>, fact: &str, query_terms: &[String]) -> bool {
    field_matches(field.as_deref(), query_terms) || text_matches(fact, query_terms)
}

fn field_matches(field: Option<&str>, query_terms: &[String]) -> bool {
    field.is_some_and(|value| text_matches(value, query_terms))
}

fn text_matches(value: &str, query_terms: &[String]) -> bool {
    !query_terms.is_empty() && contradiction_text_matches(value, query_terms)
}
