use std::borrow::Cow;

use cortex_aql::{parse_aql, AqlParseErrorKind, AqlStatement, Condition, Literal, RetrievalMode};

#[test]
fn parse_basic_retrieve_context() {
    let statement = parse_aql(
        r#"RETRIEVE CONTEXT
FOR TASK "compare"
IN BRAIN investment_projects
USING MODE balanced
BUDGET 12000 TOKENS
WHERE space = "project:investments" AND status = "ready";"#,
    )
    .unwrap();

    let AqlStatement::RetrieveContext(raw) = statement else {
        panic!("expected retrieve");
    };
    assert_eq!(raw.task.node.value, "compare");
    assert_eq!(raw.brain.node.value, "investment_projects");
    assert_eq!(raw.mode.node, RetrievalMode::Balanced);
    assert_eq!(raw.budget_tokens.node, 12_000);
}

#[test]
fn parse_escaped_string_as_owned_cow() {
    let statement = parse_aql(
        "RETRIEVE CONTEXT FOR TASK \"a \\\"b\\\"\\n\" IN BRAIN b USING MODE fast BUDGET 1 TOKENS;",
    )
    .unwrap();
    let AqlStatement::RetrieveContext(raw) = statement else {
        panic!("expected retrieve");
    };
    assert_eq!(raw.task.node.value, "a \"b\"\n");
    assert!(matches!(raw.task.node.value, Cow::Owned(_)));
}

#[test]
fn missing_semicolon_fails() {
    let error =
        parse_aql(r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN b USING MODE fast BUDGET 1 TOKENS"#)
            .unwrap_err();
    assert!(matches!(
        error.kind,
        AqlParseErrorKind::Unexpected | AqlParseErrorKind::Incomplete
    ));
}

#[test]
fn where_and_parse() {
    let statement = parse_aql(
        r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN b USING MODE fast BUDGET 1 TOKENS
WHERE space = "s" AND status = "ready";"#,
    )
    .unwrap();
    let AqlStatement::RetrieveContext(raw) = statement else {
        panic!("expected retrieve");
    };
    assert!(matches!(
        raw.where_clause.unwrap().node,
        Condition::And(_, _)
    ));
}

#[test]
fn parse_verify_fact() {
    let statement = parse_aql(r#"VERIFY FACT "revenue increased" IN BRAIN finance;"#).unwrap();
    let AqlStatement::VerifyFact(raw) = statement else {
        panic!("expected verify fact");
    };
    assert_eq!(raw.fact.node.value, "revenue increased");
    assert_eq!(raw.brain.node.value, "finance");
}

#[test]
fn parse_remember_with_ttl() {
    let statement = parse_aql(
        r#"REMEMBER "use conservative budget" IN SCOPE team AS TYPE decision TTL 60 SECONDS;"#,
    )
    .unwrap();
    let AqlStatement::Remember(raw) = statement else {
        panic!("expected remember");
    };
    assert_eq!(raw.content.node.value, "use conservative budget");
    assert_eq!(raw.scope.node.value, "team");
    assert_eq!(raw.memory_type.node.value, "decision");
    assert!(raw.ttl.is_some());
}

#[test]
fn huge_integer_does_not_become_zero() {
    let error = parse_aql(
        r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN b USING MODE fast
BUDGET 184467440737095516160 TOKENS;"#,
    )
    .unwrap_err();
    assert_eq!(error.kind, AqlParseErrorKind::InvalidInteger);
}

#[test]
fn invalid_mode_fails() {
    let error =
        parse_aql(r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN b USING MODE turbo BUDGET 1 TOKENS;"#)
            .unwrap_err();
    assert_eq!(error.kind, AqlParseErrorKind::InvalidMode);
}

#[test]
fn where_string_literal_without_escape_is_borrowed() {
    let statement = parse_aql(
        r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN b USING MODE fast BUDGET 1 TOKENS
WHERE status = "ready";"#,
    )
    .unwrap();
    let AqlStatement::RetrieveContext(raw) = statement else {
        panic!("expected retrieve");
    };
    let Condition::Predicate { literal, .. } = raw.where_clause.unwrap().node else {
        panic!("expected predicate");
    };
    let Literal::String(value) = literal.node else {
        panic!("expected string literal");
    };
    assert!(matches!(value.value, Cow::Borrowed("ready")));
}
