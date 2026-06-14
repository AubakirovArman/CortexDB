use nom::bytes::complete::tag;
use nom::character::complete::{char, multispace0};
use nom::combinator::opt;
use nom::sequence::{delimited, tuple};
use nom::Err;

use crate::ast::{Requirement, Spanned, TtlValue};
use crate::errors::AqlParseErrorKind;

use super::super::string::parse_quoted_string;
use super::super::{
    kw, parse_decimal, parse_spanned_integer, span_between, starts_with_keyword, ws1, PResult,
    ParseFailure, Span,
};

pub(super) fn parse_require(input: Span<'_>) -> PResult<'_, Vec<Spanned<Requirement<'_>>>> {
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
    } else if starts_with_keyword(input, "valid") {
        parse_valid_at(input)
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

fn parse_valid_at(input: Span<'_>) -> PResult<'_, Spanned<Requirement<'_>>> {
    let start = input;
    let (input, _) = kw("valid")(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("at")(input)?;
    let (input, _) = ws1(input)?;
    let (input, date) = parse_quoted_string(input)?;
    Ok((
        input,
        Spanned::new(Requirement::ValidAt(date.node), span_between(start, input)),
    ))
}
