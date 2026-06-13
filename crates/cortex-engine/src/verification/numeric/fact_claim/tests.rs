use cortex_aql::{AgentId, AgentView, BrainId};
use cortex_core::{CellDescriptor, CellId};

use crate::query::scope_id;
use crate::source_trust::SourceTrustCategory;
use crate::verification::numeric::extract_numeric_values;
use crate::verification::VerificationMatchKind;
use crate::Database;

use super::*;

#[test]
fn conservative_record_extracts_single_typed_numeric_fact() {
    let descriptor = CellDescriptor {
        scope: "project:finance".to_owned(),
        source: Some("finance-ledger".to_owned()),
        ..CellDescriptor::default()
    };
    let record = FactClaimStore::record_from_payload(
        CellId(7),
        b"metric=budget\nvalue=1.2B KZT\nproject=Mirny\n\nBudget approved.",
        &descriptor,
    )
    .unwrap();

    assert_eq!(record.cell_id, CellId(7));
    assert_eq!(record.scope, "project:finance");
    assert_eq!(record.metric, "budget");
    assert_eq!(record.value.scaled_value, 1_200_000_000);
    assert_eq!(record.value.currency.as_deref(), Some("KZT"));
    assert_eq!(record.project.as_deref(), Some("Mirny"));
    assert_eq!(record.source.as_deref(), Some("finance-ledger"));
}

#[test]
fn conservative_record_rejects_ambiguous_numeric_values() {
    let descriptor = CellDescriptor::default();
    assert!(FactClaimStore::record_from_payload(
        CellId(1),
        b"metric=budget\nvalue=1.2B KZT and 1.4B KZT\n\nambiguous",
        &descriptor,
    )
    .is_none());
}

#[test]
fn visible_records_respect_agent_scope() {
    let store = FactClaimStore::from_records([
        numeric_record(CellId(1), "public", "12 KZT"),
        numeric_record(CellId(2), "private", "14 KZT"),
    ]);

    assert_eq!(store.visible_records(&view()).len(), 1);
    assert_eq!(store.visible_records(&view())[0].cell_id, CellId(1));
}

#[test]
fn add_verify_matches_emits_typed_numeric_support_and_conflict() {
    let store = FactClaimStore::from_records([
        solar_budget_record(CellId(1), "12 KZT"),
        solar_budget_record(CellId(2), "14 KZT"),
    ]);
    let mut evidence = Vec::new();
    let mut contradictions = Vec::new();
    let mut conflicts = Vec::new();

    store.add_verify_matches(
        "Solar budget is 12 KZT",
        &view(),
        &mut evidence,
        &mut contradictions,
        &mut conflicts,
    );

    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].cell_id, CellId(1));
    assert_eq!(
        evidence[0].match_kind,
        VerificationMatchKind::NumericEntailment
    );
    assert_eq!(contradictions.len(), 1);
    assert_eq!(contradictions[0].cell_id, CellId(2));
    assert_eq!(
        contradictions[0].match_kind,
        VerificationMatchKind::NumericContradiction
    );
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].right, "14 KZT");

    store.add_verify_matches(
        "Solar budget is 12 KZT",
        &view(),
        &mut evidence,
        &mut contradictions,
        &mut conflicts,
    );
    assert_eq!(evidence.len(), 1);
    assert_eq!(contradictions.len(), 1);
    assert_eq!(conflicts.len(), 1);
}

#[test]
fn database_fact_claim_store_tracks_write_patch_tombstone_and_reopen() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(
            CellId(11),
            b"scope=public\nmetric=budget\nvalue=12 KZT\nproject=Solar\n\nSolar budget.".to_vec(),
        )
        .unwrap();

        let records = db.fact_claim_store.visible_records(&view());
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].value.scaled_value, 12);

        db.patch_cell(
            CellId(11),
            b"scope=public\nmetric=budget\nvalue=14 KZT\nproject=Solar\n\nSolar budget.".to_vec(),
        )
        .unwrap();
        let records = db.fact_claim_store.visible_records(&view());
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].value.scaled_value, 14);

        db.checkpoint().unwrap();
    }

    {
        let mut db = Database::open(dir.path()).unwrap();
        let records = db.fact_claim_store.visible_records(&view());
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].value.scaled_value, 14);

        db.tombstone_cell(CellId(11)).unwrap();
        assert!(db.fact_claim_store.visible_records(&view()).is_empty());
    }
}

fn numeric_record(cell_id: CellId, scope: &str, value: &str) -> NumericFactRecord {
    NumericFactRecord {
        cell_id,
        scope: scope.to_owned(),
        metric: "budget".to_owned(),
        value: extract_numeric_values(value).remove(0),
        project: None,
        source: None,
        citation: None,
        source_trust_q16: u16::MAX,
        source_trust_category: SourceTrustCategory::Official,
    }
}

fn solar_budget_record(cell_id: CellId, value: &str) -> NumericFactRecord {
    NumericFactRecord {
        project: Some("Solar".to_owned()),
        source: Some("ledger".to_owned()),
        citation: Some("ledger".to_owned()),
        source_trust_q16: 60_000,
        ..numeric_record(cell_id, "public", value)
    }
}

fn view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: None,
        readable_brains: [BrainId(1)].into_iter().collect(),
        readable_scopes: [scope_id("public")].into_iter().collect(),
        writable_scopes: Default::default(),
        allowed_modes: Default::default(),
        allowed_memory_types: Default::default(),
        max_context_budget_tokens: 1000,
        default_context_budget_tokens: 1000,
        max_candidate_limit: 10,
        default_candidate_limit: 10,
        min_required_confidence_q16: 0,
        max_ttl_seconds: None,
        allow_remember: false,
        allow_verify_fact: true,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}
