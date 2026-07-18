use cortex_aql::{AgentId, AgentView, BrainId};
use cortex_core::{CellDescriptor, CellId};

use crate::query::scope_id;
use crate::source_trust::SourceTrustCategory;
use crate::verification::numeric::extract_numeric_values;
use crate::verification::temporal::{parse_temporal_date, TemporalValidity};
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
    let records = FactClaimStore::records_from_payload(
        CellId(7),
        b"metric=budget\nvalue=1.2B KZT\nproject=Mirny\n\nBudget approved.",
        &descriptor,
    );
    assert_eq!(records.len(), 1);
    let record = &records[0];

    assert_eq!(record.cell_id, CellId(7));
    assert_eq!(record.scope, "project:finance");
    assert_eq!(record.metric, "budget");
    assert_eq!(record.value.scaled_value, 1_200_000_000);
    assert_eq!(record.value.currency.as_deref(), Some("KZT"));
    assert_eq!(record.project.as_deref(), Some("Mirny"));
    assert_eq!(record.source.as_deref(), Some("finance-ledger"));
}

#[test]
fn record_extracts_contextual_numeric_values_from_multivalue_body() {
    let descriptor = CellDescriptor::default();
    let records = FactClaimStore::records_from_payload(
        CellId(1),
        b"metric=budget\nproject=Solar\n\nSolar budget for 2025 increased to 1.4B KZT.",
        &descriptor,
    );

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].cell_id, CellId(1));
    assert_eq!(records[0].metric, "budget");
    assert_eq!(records[0].value.scaled_value, 1_400_000_000);
    assert_eq!(records[0].value.currency.as_deref(), Some("KZT"));
    assert_eq!(records[0].project.as_deref(), Some("Solar"));
}

#[test]
fn explicit_currency_field_applies_to_numeric_value() {
    let descriptor = CellDescriptor::default();
    let records = FactClaimStore::records_from_payload(
        CellId(2),
        b"metric=budget\nproject=Solar\nvalue=1200000000\ncurrency=KZT\n\nSolar budget for 2025.",
        &descriptor,
    );

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].value.scaled_value, 1_200_000_000);
    assert_eq!(records[0].value.currency.as_deref(), Some("KZT"));
}

#[test]
fn contextual_numeric_values_ignore_contradicts_marker() {
    let descriptor = CellDescriptor::default();
    let records = FactClaimStore::records_from_payload(
        CellId(3),
        b"metric=budget\nproject=Solar\ncontradicts=Solar budget is 1.2B KZT for 2025\n\nSolar budget is 1.4B KZT for 2025.",
        &descriptor,
    );

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].value.scaled_value, 1_400_000_000);
    assert_eq!(records[0].value.currency.as_deref(), Some("KZT"));
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
fn indexed_records_match_same_metric_scope_project_and_value_context() {
    let store = FactClaimStore::from_records([
        solar_budget_record(CellId(1), "12 KZT"),
        solar_budget_record(CellId(2), "14 KZT"),
        NumericFactRecord {
            project: Some("Lunar".to_owned()),
            ..numeric_record(CellId(3), "public", "16 KZT")
        },
        NumericFactRecord {
            metric: "headcount".to_owned(),
            project: Some("Solar".to_owned()),
            ..numeric_record(CellId(4), "public", "18 KZT")
        },
        solar_budget_record(CellId(5), "12 USD"),
    ]);
    let fact_values = extract_numeric_values("Solar budget is 12 KZT");
    let records = store.indexed_records_for_fact("Solar budget is 12 KZT", &view(), &fact_values);
    let cell_ids = records
        .iter()
        .map(|record| record.cell_id)
        .collect::<Vec<_>>();

    assert_eq!(cell_ids, vec![CellId(1), CellId(2), CellId(5)]);
}

#[test]
fn explicit_multivalue_records_are_indexed_deterministically() {
    let descriptor = CellDescriptor {
        scope: "public".to_owned(),
        ..CellDescriptor::default()
    };
    let records = FactClaimStore::records_from_payload(
        CellId(8),
        b"metric=budget\nproject=Solar\nvalue=1.2B KZT and 1.4B KZT\n\nSolar budget range.",
        &descriptor,
    );
    assert_eq!(
        records
            .iter()
            .map(|record| record.value.scaled_value)
            .collect::<Vec<_>>(),
        vec![1_200_000_000, 1_400_000_000]
    );

    let store = FactClaimStore::from_records(records);
    let fact_values = extract_numeric_values("Solar budget is 1.2B KZT");
    let indexed = store.indexed_records_for_fact("Solar budget is 1.2B KZT", &view(), &fact_values);

    assert_eq!(indexed.len(), 2);
    assert_eq!(
        indexed
            .iter()
            .map(|record| record.value.scaled_value)
            .collect::<Vec<_>>(),
        vec![1_200_000_000, 1_400_000_000]
    );
}

#[test]
fn dated_fact_indexes_overlapping_numeric_records() {
    let mut expired = solar_budget_record(CellId(1), "14 KZT");
    expired.temporal_validity = temporal_validity(None, Some("2024-12-31"));
    let mut current = solar_budget_record(CellId(2), "16 KZT");
    current.temporal_validity = temporal_validity(Some("2025-01-01"), Some("2025-12-31"));
    let store = FactClaimStore::from_records([expired, current]);
    let fact = "Solar budget is 12 KZT on 2025-05-01";
    let fact_values = extract_numeric_values(fact);

    assert_eq!(
        indexed_cell_ids(&store, fact, &fact_values),
        vec![CellId(2)]
    );

    let mut evidence = Vec::new();
    let mut contradictions = Vec::new();
    let mut conflicts = Vec::new();
    store.add_verify_matches(
        fact,
        &view(),
        &mut evidence,
        &mut contradictions,
        &mut conflicts,
    );

    assert!(evidence.is_empty());
    assert_eq!(contradictions.len(), 1);
    assert_eq!(contradictions[0].cell_id, CellId(2));
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].cell_id, CellId(2));
    assert_eq!(conflicts[0].right, "16 KZT");
}

#[test]
fn metric_value_index_tracks_incremental_apply_and_tombstone() {
    let mut store = FactClaimStore::default();
    let fact = "Solar budget is 12 KZT";
    let fact_values = extract_numeric_values(fact);

    store.apply_records(CellId(1), vec![solar_budget_record(CellId(1), "12 KZT")]);
    store.apply_records(CellId(2), vec![solar_budget_record(CellId(2), "14 KZT")]);
    assert_eq!(
        indexed_cell_ids(&store, fact, &fact_values),
        vec![CellId(1), CellId(2)]
    );

    store.apply_records(CellId(2), vec![solar_budget_record(CellId(2), "12 KZT")]);
    assert_eq!(
        indexed_cell_ids(&store, fact, &fact_values),
        vec![CellId(1), CellId(2)]
    );

    store.apply_tombstone(CellId(1));
    assert_eq!(
        indexed_cell_ids(&store, fact, &fact_values),
        vec![CellId(2)]
    );

    store.apply_tombstone(CellId(2));
    assert!(indexed_cell_ids(&store, fact, &fact_values).is_empty());
    assert!(store.index.by_metric.is_empty());
    assert!(store.index.metric_terms.is_empty());
}

#[test]
fn multivalue_index_tracks_patch_tombstone_and_reopen() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut db = Database::open(dir.path()).unwrap();
        db.put_cell(
            CellId(21),
            b"scope=public\nmetric=budget\nproject=Solar\n\nSolar budget for 2025 is 1.4B KZT."
                .to_vec(),
        )
        .unwrap();
        assert_eq!(
            db.derived_stores
                .fact_claim_store
                .visible_records(&view())
                .len(),
            1
        );

        db.patch_cell(
            CellId(21),
            b"scope=public\nmetric=budget\nproject=Solar\nvalue=1.2B KZT and 1.4B KZT\n\nSolar budget range."
                .to_vec(),
        )
        .unwrap();
        let records = db.derived_stores.fact_claim_store.visible_records(&view());
        assert_eq!(records.len(), 2);
        assert_eq!(
            records
                .iter()
                .map(|record| record.value.scaled_value)
                .collect::<Vec<_>>(),
            vec![1_200_000_000, 1_400_000_000]
        );

        db.checkpoint().unwrap();
    }

    {
        let mut db = Database::open(dir.path()).unwrap();
        assert_eq!(
            db.derived_stores
                .fact_claim_store
                .visible_records(&view())
                .len(),
            2
        );
        db.tombstone_cell(CellId(21)).unwrap();
        assert!(db
            .derived_stores
            .fact_claim_store
            .visible_records(&view())
            .is_empty());
    }
}

#[test]
fn metric_value_index_matches_incremental_property_sequence() {
    let mut store = FactClaimStore::default();
    let fact = "Solar budget is 12 KZT";
    let fact_values = extract_numeric_values(fact);
    for step in 0..48 {
        let cell_id = CellId((step % 6 + 1) as u64);
        if step % 5 == 0 {
            store.apply_tombstone(cell_id);
        } else {
            let value = if step % 3 == 0 { "12 KZT" } else { "14 KZT" };
            let project = if step % 2 == 0 { "Solar" } else { "Lunar" };
            store.apply_records(
                cell_id,
                vec![NumericFactRecord {
                    project: Some(project.to_owned()),
                    ..numeric_record(cell_id, "public", value)
                }],
            );
        }

        let indexed = indexed_cell_ids(&store, fact, &fact_values);
        let mut expected = store
            .records
            .values()
            .flatten()
            .filter(|record| {
                record.scope == "public"
                    && record.metric == "budget"
                    && record.project.as_deref() == Some("Solar")
                    && fact_values.iter().any(|fact_value| {
                        fact_value.compare_normalized(&record.value)
                            != crate::verification::numeric::NumericComparison::Incomparable
                    })
            })
            .map(|record| record.cell_id)
            .collect::<Vec<_>>();
        expected.sort();
        assert_eq!(indexed, expected);
    }
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

        let records = db.derived_stores.fact_claim_store.visible_records(&view());
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].value.scaled_value, 12);

        db.patch_cell(
            CellId(11),
            b"scope=public\nmetric=budget\nvalue=14 KZT\nproject=Solar\n\nSolar budget.".to_vec(),
        )
        .unwrap();
        let records = db.derived_stores.fact_claim_store.visible_records(&view());
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].value.scaled_value, 14);

        db.checkpoint().unwrap();
    }

    {
        let mut db = Database::open(dir.path()).unwrap();
        let records = db.derived_stores.fact_claim_store.visible_records(&view());
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].value.scaled_value, 14);

        db.tombstone_cell(CellId(11)).unwrap();
        assert!(db
            .derived_stores
            .fact_claim_store
            .visible_records(&view())
            .is_empty());
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
        source_ref: None,
        temporal_validity: TemporalValidity::default(),
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

fn indexed_cell_ids(
    store: &FactClaimStore,
    fact: &str,
    fact_values: &[NumericValue],
) -> Vec<CellId> {
    store
        .indexed_records_for_fact(fact, &view(), fact_values)
        .into_iter()
        .map(|record| record.cell_id)
        .collect()
}

fn temporal_validity(valid_from: Option<&str>, valid_to: Option<&str>) -> TemporalValidity {
    TemporalValidity {
        valid_from: valid_from.and_then(parse_temporal_date),
        valid_to: valid_to.and_then(parse_temporal_date),
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
