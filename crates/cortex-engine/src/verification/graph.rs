//! Verification enrichment from typed graph relation cells.

use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::AgentView;
use cortex_core::memtable::CellVersion;
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

pub(super) fn graph_contradiction_for_version(
    cell_id: CellId,
    payload: &[u8],
    fact: &str,
    view: &AgentView,
    existing: &BTreeSet<CellId>,
) -> Option<VerificationEvidence> {
    if existing.contains(&cell_id) {
        return None;
    }
    let metadata = CellMetadata::from_payload(payload);
    if !view.can_read_scope(scope_id(&metadata.scope)) {
        return None;
    }
    let relation = RelationBody::parse(payload);
    let kind = relation
        .predicate
        .as_deref()
        .map(GraphEdgeKind::from_predicate)?;
    if kind != GraphEdgeKind::FactContradictsFact {
        return None;
    }
    let fact_terms = tokenize(fact);
    let relation_fact = relation.object.as_deref().filter(|value| {
        value
            .trim()
            .strip_prefix("cell:")
            .is_none_or(|id| id.parse::<u64>().is_err())
    })?;
    let matched_terms = matched_terms(relation_fact, &fact_terms);
    if matched_terms == 0 {
        return None;
    }
    let trust = SourceTrust::from_metadata(metadata.source_trust_q16, metadata.source_trust_class);
    Some(VerificationEvidence {
        cell_id,
        matched_terms,
        match_score_q16: term_coverage_q16(matched_terms, &fact_terms),
        match_kind: VerificationMatchKind::GraphContradiction,
        source_trust_q16: trust.q16,
        source_trust_category: trust.category,
        citation: metadata
            .citation()
            .map(str::to_owned)
            .or_else(|| relation.subject.as_deref().and_then(source_endpoint_value))
            .or_else(|| relation.object.as_deref().and_then(source_endpoint_value)),
    })
}

pub(super) fn add_graph_relation_contradictions_from_versions(
    versions: &[&CellVersion],
    fact: &str,
    view: &AgentView,
    contradicting_evidence: &mut Vec<VerificationEvidence>,
) {
    let fact_terms = tokenize(fact);
    let mut existing = contradicting_evidence
        .iter()
        .map(|evidence| evidence.cell_id)
        .collect::<BTreeSet<_>>();
    for version in versions {
        let Some(mut evidence) = graph_contradiction_for_version(
            version.cell_id,
            &version.payload,
            fact,
            view,
            &existing,
        ) else {
            continue;
        };
        if evidence.matched_terms == 0 {
            evidence.matched_terms = matched_terms(fact, &fact_terms);
        }
        existing.insert(evidence.cell_id);
        contradicting_evidence.push(evidence);
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
    versions: &[&CellVersion],
    view: &AgentView,
    evidence: &mut [VerificationEvidence],
) {
    let target_cells = evidence
        .iter()
        .map(|item| item.cell_id)
        .collect::<BTreeSet<_>>();
    if target_cells.is_empty() {
        return;
    }
    let mut supports = source_supports_by_fact_cell_from_versions(versions, view, &target_cells);
    if let Ok(Some(index)) = db.read_persisted_knowledge_graph_index() {
        merge_source_supports_from_edges(
            &mut supports,
            db,
            view,
            index.source_supports_fact_edges().into_iter(),
            &target_cells,
        );
    }
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

fn source_supports_by_fact_cell_from_versions(
    versions: &[&CellVersion],
    view: &AgentView,
    target_cells: &BTreeSet<CellId>,
) -> BTreeMap<CellId, SourceSupport> {
    let mut supports = BTreeMap::new();
    for version in versions {
        let Some((fact_cell_id, support)) =
            source_support_from_payload(version.cell_id, &version.payload, view, target_cells)
        else {
            continue;
        };
        insert_source_support(&mut supports, fact_cell_id, support);
    }
    supports
}

fn merge_source_supports_from_edges(
    supports: &mut BTreeMap<CellId, SourceSupport>,
    db: &Database,
    view: &AgentView,
    edges: impl Iterator<Item = GraphEdge>,
    target_cells: &BTreeSet<CellId>,
) {
    for edge in edges {
        let Some(fact_cell_id) = fact_cell_endpoint(&edge) else {
            continue;
        };
        if !target_cells.contains(&fact_cell_id) {
            continue;
        }
        let Some(support) = source_support_from_edge(db, view, &edge) else {
            continue;
        };
        insert_source_support(supports, fact_cell_id, support);
    }
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

fn source_support_from_payload(
    relation_cell_id: CellId,
    payload: &[u8],
    view: &AgentView,
    target_cells: &BTreeSet<CellId>,
) -> Option<(CellId, SourceSupport)> {
    let metadata = CellMetadata::from_payload(payload);
    if !view.can_read_scope(scope_id(&metadata.scope)) {
        return None;
    }
    let relation = RelationBody::parse(payload);
    let kind = relation
        .predicate
        .as_deref()
        .map(GraphEdgeKind::from_predicate)?;
    if kind != GraphEdgeKind::SourceSupportsFact {
        return None;
    }
    let fact_cell_id = relation
        .object
        .as_deref()
        .and_then(cell_endpoint)
        .or_else(|| relation.subject.as_deref().and_then(cell_endpoint))?;
    if !target_cells.contains(&fact_cell_id) {
        return None;
    }
    let trust = SourceTrust::from_metadata(metadata.source_trust_q16, metadata.source_trust_class);
    Some((
        fact_cell_id,
        SourceSupport {
            relation_cell_id,
            source_trust_q16: trust.q16,
            source_trust_category: trust.category,
            citation: metadata
                .citation()
                .map(str::to_owned)
                .or_else(|| relation.subject.as_deref().and_then(source_endpoint_value))
                .or_else(|| relation.object.as_deref().and_then(source_endpoint_value)),
        },
    ))
}

fn insert_source_support(
    supports: &mut BTreeMap<CellId, SourceSupport>,
    fact_cell_id: CellId,
    support: SourceSupport,
) {
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
