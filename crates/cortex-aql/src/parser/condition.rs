use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{char, digit1, multispace0};
use nom::combinator::{map, recognize};
use nom::sequence::{delimited, terminated, tuple};
use nom::Err;

use crate::ast::{Comparator, Condition, DecimalLiteral, Literal, Spanned};
use crate::errors::AqlParseErrorKind;

use super::string::parse_quoted_string;
use super::{
    kw, parse_identifier, parse_integer, source_span, spanned, ws1, PResult, ParseFailure, Span,
};

const MAX_WHERE_DEPTH: usize = 32;

pub(super) fn parse_condition(input: Span<'_>) -> PResult<'_, Spanned<Condition<'_>>> {
    parse_condition_or(input, 0)
}

fn parse_condition_or(input: Span<'_>, depth: usize) -> PResult<'_, Spanned<Condition<'_>>> {
    guard_depth(input, depth)?;
    let (mut input, mut lhs) = parse_condition_and(input, depth)?;
    loop {
        let Ok((next, _)) = delimited(multispace0, kw("OR"), multispace0)(input) else {
            break;
        };
        let (next, rhs) = parse_condition_and(next, depth)?;
        let span = lhs.span;
        lhs = Spanned::new(Condition::Or(Box::new(lhs), Box::new(rhs)), span);
        input = next;
    }
    Ok((input, lhs))
}

fn parse_condition_and(input: Span<'_>, depth: usize) -> PResult<'_, Spanned<Condition<'_>>> {
    guard_depth(input, depth)?;
    let (mut input, mut lhs) = parse_condition_not(input, depth)?;
    loop {
        let Ok((next, _)) = delimited(multispace0, kw("AND"), multispace0)(input) else {
            break;
        };
        let (next, rhs) = parse_condition_not(next, depth)?;
        let span = lhs.span;
        lhs = Spanned::new(Condition::And(Box::new(lhs), Box::new(rhs)), span);
        input = next;
    }
    Ok((input, lhs))
}

fn parse_condition_not(input: Span<'_>, depth: usize) -> PResult<'_, Spanned<Condition<'_>>> {
    guard_depth(input, depth)?;
    if let Ok((next, _)) = terminated(kw("NOT"), ws1)(input) {
        let (next, child) = parse_condition_not(next, depth + 1)?;
        return Ok((
            next,
            Spanned::new(Condition::Not(Box::new(child)), source_span(input)),
        ));
    }
    parse_condition_primary(input, depth)
}

fn parse_condition_primary(input: Span<'_>, depth: usize) -> PResult<'_, Spanned<Condition<'_>>> {
    guard_depth(input, depth)?;
    alt((
        delimited(
            delimited(multispace0, char('('), multispace0),
            |next| parse_condition_or(next, depth + 1),
            delimited(multispace0, char(')'), multispace0),
        ),
        parse_predicate,
    ))(input)
}

fn parse_predicate(input: Span<'_>) -> PResult<'_, Spanned<Condition<'_>>> {
    let start = input;
    let (input, field) = parse_identifier(input)?;
    let (input, _) = multispace0(input)?;
    let (input, comparator) = parse_comparator(input)?;
    let (input, _) = multispace0(input)?;
    let (input, literal) = parse_literal(input)?;
    Ok((
        input,
        Spanned::new(
            Condition::Predicate {
                field,
                comparator,
                literal,
            },
            source_span(start),
        ),
    ))
}

fn parse_comparator(input: Span<'_>) -> PResult<'_, Spanned<Comparator>> {
    spanned(alt((
        map(tag("!="), |_| Comparator::NotEq),
        map(tag(">="), |_| Comparator::Gte),
        map(tag("<="), |_| Comparator::Lte),
        map(tag("="), |_| Comparator::Eq),
        map(tag(">"), |_| Comparator::Gt),
        map(tag("<"), |_| Comparator::Lt),
    )))(input)
}

fn parse_literal(input: Span<'_>) -> PResult<'_, Spanned<Literal<'_>>> {
    alt((
        map(parse_quoted_string, |s| {
            Spanned::new(Literal::String(s.node), s.span)
        }),
        spanned(map(parse_decimal, Literal::Decimal)),
        spanned(map(parse_integer, Literal::Integer)),
        spanned(map(alt((kw("true"), kw("false"))), |value: Span<'_>| {
            Literal::Boolean(value.fragment().eq_ignore_ascii_case("true"))
        })),
    ))(input)
}

fn parse_decimal(input: Span<'_>) -> PResult<'_, DecimalLiteral<'_>> {
    map(
        recognize(tuple((digit1, char('.'), digit1))),
        |span: Span<'_>| DecimalLiteral::borrowed(span.fragment()),
    )(input)
}

fn guard_depth(input: Span<'_>, depth: usize) -> PResult<'_, ()> {
    if depth > MAX_WHERE_DEPTH {
        Err(Err::Failure(ParseFailure::new(
            input,
            AqlParseErrorKind::WhereDepthExceeded,
        )))
    } else {
        Ok((input, ()))
    }
}
