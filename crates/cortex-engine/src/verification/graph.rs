//! Verification enrichment from typed graph relation cells.

use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::AgentView;
use cortex_core::CellId;

use crate::database::Database;
use crate::graph::{GraphEdge, GraphEdgeKind};
use crate::query::{scope_id, CellMetadata};
use crate::search::tokenize;
use crate::source_trust::{SourceTrust, SourceTrustCategory};
use crate::typed_body::RelationBody;

use super::{support::term_coverage_q16, VerificationEvidence, VerificationMatchKind};

#[derive(Clone, Debug)]
struct SourceSupport {
    relation_cell_id: CellId,
    source_trust_q16: u16,
    source_trust_category: SourceTrustCategory,
    citation: Option<String>,
}

pub(super) fn add_graph_relation_contradictions(
    db: &Database,
    fact: &str,
    view: &AgentView,
    contradicting_evidence: &mut Vec<VerificationEvidence>,
) {
    let fact_terms = tokenize(fact);
    let mut existing = contradicting_evidence
        .iter()
        .map(|evidence| evidence.cell_id)
        .collect::<BTreeSet<_>>();
    for record in db.conflicts_for_fact(fact, view) {
        let Some(relation_cell_id) = record.relation_cell_id else {
            continue;
        };
        if !existing.insert(relation_cell_id) {
            continue;
        }
        let matched_terms = matched_terms(&record.fact, &fact_terms);
        contradicting_evidence.push(VerificationEvidence {
            cell_id: relation_cell_id,
            matched_terms,
            match_score_q16: term_coverage_q16(matched_terms, &fact_terms),
            match_kind: VerificationMatchKind::GraphContradiction,
            source_trust_q16: record.source_trust_q16,
            source_trust_category: record.source_trust_category,
            citation: record.source.or_else(|| {
                record
                    .source_cell_id
                    .map(|source_cell_id| format!("cell:{}", source_cell_id.0))
            }),
        });
    }
}

pub(super) fn is_graph_contradiction_payload(payload: &[u8]) -> bool {
    let relation = RelationBody::parse(payload);
    relation
        .predicate
        .as_deref()
        .map(GraphEdgeKind::from_predicate)
        == Some(GraphEdgeKind::FactContradictsFact)
}

pub(super) fn enrich_evidence_from_source_support_edges(
    db: &Database,
    view: &AgentView,
    evidence: &mut [VerificationEvidence],
) {
    let supports = source_supports_by_fact_cell(db, view);
    for item in evidence {
        let Some(support) = supports.get(&item.cell_id) else {
            continue;
        };
        if item.citation.is_none() {
            item.citation = support.citation.clone();
        }
        if support.source_trust_q16 > item.source_trust_q16 {
            item.source_trust_q16 = support.source_trust_q16;
            item.source_trust_category = support.source_trust_category;
        }
    }
}

fn source_supports_by_fact_cell(
    db: &Database,
    view: &AgentView,
) -> BTreeMap<CellId, SourceSupport> {
    let mut supports = BTreeMap::new();
    for edge in db.graph_source_supports_fact_edges() {
        let Some(fact_cell_id) = fact_cell_endpoint(&edge) else {
            continue;
        };
        let Some(support) = source_support_from_edge(db, view, &edge) else {
            continue;
        };
        let replace = supports
            .get(&fact_cell_id)
            .map(|existing: &SourceSupport| {
                support.source_trust_q16 > existing.source_trust_q16
                    || (support.source_trust_q16 == existing.source_trust_q16
                        && support.relation_cell_id < existing.relation_cell_id)
            })
            .unwrap_or(true);
        if replace {
            supports.insert(fact_cell_id, support);
        }
    }
    supports
}

fn source_support_from_edge(
    db: &Database,
    view: &AgentView,
    edge: &GraphEdge,
) -> Option<SourceSupport> {
    let payload = db.get_latest_cell(edge.relation_cell_id)?;
    let metadata = CellMetadata::from_payload(&payload);
    if !view.can_read_scope(scope_id(&metadata.scope)) {
        return None;
    }
    let trust = SourceTrust::from_metadata(metadata.source_trust_q16, metadata.source_trust_class);
    Some(SourceSupport {
        relation_cell_id: edge.relation_cell_id,
        source_trust_q16: trust.q16,
        source_trust_category: trust.category,
        citation: metadata
            .citation()
            .map(str::to_owned)
            .or_else(|| source_endpoint(edge)),
    })
}

fn fact_cell_endpoint(edge: &GraphEdge) -> Option<CellId> {
    cell_endpoint(&edge.object).or_else(|| cell_endpoint(&edge.subject))
}

fn cell_endpoint(value: &str) -> Option<CellId> {
    value
        .trim()
        .strip_prefix("cell:")
        .and_then(|id| id.parse::<u64>().ok())
        .map(CellId)
}

fn source_endpoint(edge: &GraphEdge) -> Option<String> {
    source_endpoint_value(&edge.subject).or_else(|| source_endpoint_value(&edge.object))
}

fn source_endpoint_value(value: &str) -> Option<String> {
    value
        .trim()
        .strip_prefix("source:")
        .map(str::to_owned)
        .filter(|value| !value.is_empty())
}

fn matched_terms(text: &str, fact_terms: &[String]) -> u32 {
    let text_terms = tokenize(text);
    fact_terms
        .iter()
        .filter(|term| text_terms.contains(term))
        .count()
        .max(1) as u32
}
