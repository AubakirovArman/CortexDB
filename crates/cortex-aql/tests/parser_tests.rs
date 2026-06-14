use std::borrow::Cow;

use cortex_aql::{
    parse_aql, parse_aql_diagnostic, AqlParseErrorKind, AqlStatement, Condition, Literal,
    Requirement, RetrievalMode,
};

fn retrieve(query: &str) -> Box<cortex_aql::RawRetrieveContext<'_>> {
    let AqlStatement::RetrieveContext(raw) = parse_aql(query).unwrap() else {
        panic!("expected retrieve");
    };
    raw
}

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
        panic!("expected retrieve")
    };
    assert_eq!(raw.task.node.value, "compare");
    assert_eq!(raw.task.span.len, "\"compare\"".len());
    assert_eq!(raw.brain.node.value, "investment_projects");
    assert_eq!(raw.mode.unwrap().node, RetrievalMode::Balanced);
    assert_eq!(raw.budget_tokens.unwrap().node, 12_000);
}

#[test]
fn parse_hybrid_retrieve_mode() {
    let raw = retrieve(
        r#"RETRIEVE CONTEXT FOR TASK "compare" IN BRAIN investment_projects
USING MODE hybrid BUDGET 12000 TOKENS;"#,
    );
    assert_eq!(raw.mode.unwrap().node, RetrievalMode::Hybrid);
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
    let raw = retrieve(
        r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN b USING MODE fast BUDGET 1 TOKENS
WHERE space = "s" AND status = "ready";"#,
    );
    assert!(matches!(
        raw.where_clause.unwrap().node,
        Condition::And(_, _)
    ));
}

#[test]
fn parse_limit_and_require() {
    let raw = retrieve(
        r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN b
LIMIT 500 CANDIDATES REQUIRE citations = true, confidence >= 0.80, source_trust >= 0.90,
freshness <= 86400 SECONDS;"#,
    );
    assert_eq!(raw.candidate_limit.unwrap().node, 500);
    assert_eq!(raw.requirements.len(), 4);
    assert!(matches!(
        raw.requirements[0].node,
        Requirement::RequireCitations
    ));
}

#[test]
fn parse_identifier_with_colon_as_literal() {
    let raw = retrieve(
        r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN b
WHERE space = project:investments;"#,
    );
    let Condition::Predicate { literal, .. } = raw.where_clause.unwrap().node else {
        panic!("expected predicate");
    };
    let Literal::Identifier(value) = literal.node else {
        panic!("expected identifier literal");
    };
    assert_eq!(value.value, "project:investments");
}

#[test]
fn where_precedence_not_and_or() {
    let raw = retrieve(
        r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN b
WHERE NOT status = "ready" AND space = "s" OR memory_type = "decision";"#,
    );
    let Condition::Or(lhs, _) = raw.where_clause.unwrap().node else {
        panic!("expected OR at root");
    };
    let Condition::And(lhs, _) = lhs.node else {
        panic!("expected AND below OR");
    };
    assert!(matches!(lhs.node, Condition::Not(_)));
}

#[test]
fn parse_in_list_literal() {
    let raw = retrieve(
        r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN b
WHERE status IN ["ready", verified];"#,
    );
    let Condition::Predicate {
        comparator,
        literal,
        ..
    } = raw.where_clause.unwrap().node
    else {
        panic!("expected predicate");
    };
    assert!(matches!(comparator.node, cortex_aql::Comparator::In));
    let Literal::List(values) = literal.node else {
        panic!("expected list");
    };
    assert_eq!(values.len(), 2);
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
fn parse_explain_retrieve_context() {
    let statement = parse_aql(r#"EXPLAIN RETRIEVE CONTEXT FOR TASK "x" IN BRAIN b;"#).unwrap();
    let AqlStatement::Explain(inner) = statement else {
        panic!("expected explain");
    };
    assert!(matches!(*inner, AqlStatement::RetrieveContext(_)));
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
fn multiline_error_reports_line_and_column() {
    let error = parse_aql_diagnostic(
        r#"RETRIEVE CONTEXT
FOR TASK "x"
IN BRAIN b
USING MODE turbo
BUDGET 1 TOKENS;"#,
    )
    .unwrap_err();
    assert_eq!(error.kind, AqlParseErrorKind::InvalidMode);
    assert_eq!(error.span.line, 4);
    assert_eq!(error.span.column, 12);
    assert!(error.message.contains("line 4, column 12"));
}

#[test]
fn expected_keyword_is_reported() {
    let error = parse_aql(r#"VERIFY FACT "x" ON BRAIN b;"#).unwrap_err();
    assert_eq!(error.kind, AqlParseErrorKind::ExpectedKeyword);
}

#[test]
fn very_deep_where_fails_with_depth_error() {
    let mut query = String::from(r#"RETRIEVE CONTEXT FOR TASK "x" IN BRAIN b WHERE "#);
    for _ in 0..34 {
        query.push_str("NOT ");
    }
    query.push_str(r#"status = "ready";"#);
    let error = parse_aql(&query).unwrap_err();
    assert_eq!(error.kind, AqlParseErrorKind::WhereDepthExceeded);
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
