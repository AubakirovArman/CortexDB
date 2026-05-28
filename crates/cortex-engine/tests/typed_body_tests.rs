use cortex_engine::{EntityBody, FactBody, RelationBody};

#[test]
fn fact_body_parses_metric_value_currency() {
    let body = b"metric=budget\nvalue=1200000000\ncurrency=KZT\nproject=Solar Plant";
    let fact = FactBody::parse(body);
    assert_eq!(fact.metric, Some("budget".to_owned()));
    assert_eq!(fact.value, Some("1200000000".to_owned()));
    assert_eq!(fact.currency, Some("KZT".to_owned()));
    assert_eq!(fact.project, Some("Solar Plant".to_owned()));
}

#[test]
fn entity_body_parses_name_and_kind() {
    let body = b"name=Solar Plant\nkind=project";
    let entity = EntityBody::parse(body);
    assert_eq!(entity.name, Some("Solar Plant".to_owned()));
    assert_eq!(entity.kind, Some("project".to_owned()));
}

#[test]
fn relation_body_parses_subject_predicate_object() {
    let body = b"subject=Solar Plant\npredicate=has_budget\nobject=1.2B KZT";
    let relation = RelationBody::parse(body);
    assert_eq!(relation.subject, Some("Solar Plant".to_owned()));
    assert_eq!(relation.predicate, Some("has_budget".to_owned()));
    assert_eq!(relation.object, Some("1.2B KZT".to_owned()));
}

#[test]
fn cell_type_from_str_roundtrips_all_variants() {
    use cortex_core::KnowledgeCellType;
    for variant in [
        KnowledgeCellType::DocumentBlock,
        KnowledgeCellType::Fact,
        KnowledgeCellType::Entity,
        KnowledgeCellType::Relation,
        KnowledgeCellType::Memory,
        KnowledgeCellType::Feedback,
        KnowledgeCellType::Tool,
        KnowledgeCellType::SourceRef,
        KnowledgeCellType::Raw,
    ] {
        let s = variant.as_str();
        assert_eq!(s.parse::<KnowledgeCellType>(), Ok(variant));
    }
    assert!("unknown_type".parse::<KnowledgeCellType>().is_err());
}
