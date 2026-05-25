use std::str::FromStr;

use nom::bytes::complete::tag;
use nom::character::complete::{char, multispace0};
use nom::combinator::opt;
use nom::sequence::{delimited, preceded, tuple};
use nom::Err;

use crate::ast::{RawRetrieveContext, Requirement, Spanned, TtlValue};
use crate::errors::AqlParseErrorKind;
use crate::types::RetrievalMode;

use super::condition::parse_condition;
use super::string::parse_quoted_string;
use super::{
    kw, parse_decimal, parse_identifier, parse_spanned_integer, parse_spanned_u32, span_between,
    starts_with_keyword, ws1, PResult, ParseFailure, Span,
};

pub(super) fn parse_retrieve_context(input: Span<'_>) -> PResult<'_, RawRetrieveContext<'_>> {
    let (input, _) = kw("RETRIEVE")(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("CONTEXT")(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("FOR")(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("TASK")(input)?;
    let (input, _) = ws1(input)?;
    let (input, task) = parse_quoted_string(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("IN")(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("BRAIN")(input)?;
    let (mut input, _) = ws1(input)?;
    let (next, brain) = parse_identifier(input)?;
    input = next;

    let mut clauses = RetrieveClauses::default();
    loop {
        let (next, _) = multispace0(input)?;
        if next.fragment().starts_with(';') {
            input = next;
            break;
        }
        input = parse_next_clause(next, &mut clauses)?;
    }
    let (input, _) = delimited(multispace0, char(';'), multispace0)(input)?;
    Ok((
        input,
        RawRetrieveContext {
            task,
            brain,
            mode: clauses.mode,
            budget_tokens: clauses.budget_tokens,
            candidate_limit: clauses.candidate_limit,
            where_clause: clauses.where_clause,
            requirements: clauses.requirements,
            strategy: None,
        },
    ))
}

#[derive(Default)]
struct RetrieveClauses<'a> {
    mode: Option<Spanned<RetrievalMode>>,
    budget_tokens: Option<Spanned<u64>>,
    candidate_limit: Option<Spanned<u32>>,
    where_clause: Option<Spanned<crate::ast::Condition<'a>>>,
    requirements: Vec<Spanned<Requirement<'a>>>,
}

fn parse_next_clause<'a>(
    input: Span<'a>,
    clauses: &mut RetrieveClauses<'a>,
) -> Result<Span<'a>, nom::Err<ParseFailure<'a>>> {
    if starts_with_keyword(input, "USING") {
        let (input, mode) = parse_using_mode(input)?;
        reject_duplicate(clauses.mode.replace(mode), input)?;
        Ok(input)
    } else if starts_with_keyword(input, "BUDGET") {
        let (input, budget) = parse_budget(input)?;
        reject_duplicate(clauses.budget_tokens.replace(budget), input)?;
        Ok(input)
    } else if starts_with_keyword(input, "LIMIT") {
        let (input, limit) = parse_limit(input)?;
        reject_duplicate(clauses.candidate_limit.replace(limit), input)?;
        Ok(input)
    } else if starts_with_keyword(input, "WHERE") {
        let (input, where_clause) = preceded(tuple((kw("WHERE"), ws1)), parse_condition)(input)?;
        reject_duplicate(clauses.where_clause.replace(where_clause), input)?;
        Ok(input)
    } else if starts_with_keyword(input, "REQUIRE") {
        let (input, mut requirements) = parse_require(input)?;
        clauses.requirements.append(&mut requirements);
        Ok(input)
    } else {
        Err(Err::Error(ParseFailure::new(
            input,
            AqlParseErrorKind::Unexpected,
        )))
    }
}

fn parse_using_mode(input: Span<'_>) -> PResult<'_, Spanned<RetrievalMode>> {
    let (input, _) = kw("USING")(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("MODE")(input)?;
    let (input, _) = ws1(input)?;
    let start = input;
    let (input, identifier) = parse_identifier(input)?;
    let mode = RetrievalMode::from_str(identifier.node.value.as_ref())
        .map_err(|_| Err::Failure(ParseFailure::new(start, AqlParseErrorKind::InvalidMode)))?;
    Ok((input, Spanned::new(mode, span_between(start, input))))
}

fn parse_budget(input: Span<'_>) -> PResult<'_, Spanned<u64>> {
    let (input, _) = kw("BUDGET")(input)?;
    let (input, _) = ws1(input)?;
    let (input, budget) = parse_spanned_integer(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("TOKENS")(input)?;
    Ok((input, budget))
}

fn parse_limit(input: Span<'_>) -> PResult<'_, Spanned<u32>> {
    let (input, _) = kw("LIMIT")(input)?;
    let (input, _) = ws1(input)?;
    let (input, limit) = parse_spanned_u32(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("CANDIDATES")(input)?;
    Ok((input, limit))
}

fn parse_require(input: Span<'_>) -> PResult<'_, Vec<Spanned<Requirement<'_>>>> {
    let (mut input, _) = kw("REQUIRE")(input)?;
    let (next, _) = ws1(input)?;
    input = next;
    let mut requirements = Vec::new();
    loop {
        let (next, requirement) = parse_requirement(input)?;
        requirements.push(requirement);
        let (next, comma) = opt(delimited(multispace0, char(','), multispace0))(next)?;
        if comma.is_none() {
            return Ok((next, requirements));
        }
        input = next;
    }
}

fn parse_requirement(input: Span<'_>) -> PResult<'_, Spanned<Requirement<'_>>> {
    if starts_with_keyword(input, "citations") {
        parse_citations(input)
    } else if starts_with_keyword(input, "confidence") {
        parse_decimal_requirement(input, "confidence", Requirement::MinConfidence)
    } else if starts_with_keyword(input, "source_trust") {
        parse_decimal_requirement(input, "source_trust", Requirement::SourceTrust)
    } else if starts_with_keyword(input, "freshness") {
        parse_freshness(input)
    } else {
        Err(Err::Error(ParseFailure::new(
            input,
            AqlParseErrorKind::Unexpected,
        )))
    }
}

fn parse_citations(input: Span<'_>) -> PResult<'_, Spanned<Requirement<'_>>> {
    let start = input;
    let (input, _) = kw("citations")(input)?;
    let (input, _) = opt(tuple((multispace0, char('='), multispace0, kw("true"))))(input)?;
    Ok((
        input,
        Spanned::new(Requirement::RequireCitations, span_between(start, input)),
    ))
}

fn parse_decimal_requirement<'a>(
    input: Span<'a>,
    name: &'static str,
    build: fn(crate::ast::DecimalLiteral<'a>) -> Requirement<'a>,
) -> PResult<'a, Spanned<Requirement<'a>>> {
    let start = input;
    let (input, _) = kw(name)(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = tag(">=")(input)?;
    let (input, _) = multispace0(input)?;
    let (input, decimal) = parse_decimal(input)?;
    Ok((
        input,
        Spanned::new(build(decimal), span_between(start, input)),
    ))
}

fn parse_freshness(input: Span<'_>) -> PResult<'_, Spanned<Requirement<'_>>> {
    let start = input;
    let (input, _) = kw("freshness")(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("<=")(input)?;
    let (input, _) = multispace0(input)?;
    let (input, seconds) = parse_spanned_integer(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("SECONDS")(input)?;
    Ok((
        input,
        Spanned::new(
            Requirement::Freshness(TtlValue::Seconds(seconds.node)),
            span_between(start, input),
        ),
    ))
}

fn reject_duplicate<T>(existing: Option<T>, input: Span<'_>) -> PResult<'_, ()> {
    if existing.is_some() {
        Err(Err::Failure(ParseFailure::new(
            input,
            AqlParseErrorKind::Unexpected,
        )))
    } else {
        Ok((input, ()))
    }
}
