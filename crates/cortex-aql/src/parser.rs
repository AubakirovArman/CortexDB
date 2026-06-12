use std::num::ParseIntError;

use nom::bytes::complete::{tag_no_case, take_while, take_while1};
use nom::character::complete::{char, digit1, multispace0};
use nom::combinator::{all_consuming, map, map_res, opt, recognize};
use nom::error::{ErrorKind, FromExternalError, ParseError};
use nom::sequence::{delimited, pair, preceded, terminated, tuple};
use nom::{Err, IResult};
use nom_locate::LocatedSpan;

use crate::ast::{
    AqlStatement, DecimalLiteral, Identifier, RawRemember, RawVerifyFact, SourceSpan, Spanned,
    TtlValue,
};
use crate::errors::{AqlParseError, AqlParseErrorKind};

mod condition;
mod retrieve;
mod string;

use retrieve::parse_retrieve_context;
use string::parse_quoted_string;

pub(super) type Span<'a> = LocatedSpan<&'a str>;
pub(super) type PResult<'a, O> = IResult<Span<'a>, O, ParseFailure<'a>>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ParseFailure<'a> {
    span: Span<'a>,
    kind: AqlParseErrorKind,
}

pub fn parse_aql(input: &str) -> Result<AqlStatement<'_>, AqlParseError> {
    parse_aql_diagnostic(input)
}

pub fn parse_aql_diagnostic(input: &str) -> Result<AqlStatement<'_>, AqlParseError> {
    match all_consuming(delimited(multispace0, parse_statement, multispace0))(Span::new(input)) {
        Ok((_, statement)) => Ok(statement),
        Err(Err::Error(error)) | Err(Err::Failure(error)) => {
            Err(AqlParseError::new(error.kind, source_span(error.span)))
        }
        Err(Err::Incomplete(_)) => Err(AqlParseError::new(
            AqlParseErrorKind::Incomplete,
            SourceSpan::default(),
        )),
    }
}

fn parse_statement(input: Span<'_>) -> PResult<'_, AqlStatement<'_>> {
    if starts_with_keyword(input, "EXPLAIN") {
        parse_explain(input)
    } else if starts_with_keyword(input, "RETRIEVE") {
        map(parse_retrieve_context, |raw| {
            AqlStatement::RetrieveContext(Box::new(raw))
        })(input)
    } else if starts_with_keyword(input, "VERIFY") {
        map(parse_verify_fact, |raw| {
            AqlStatement::VerifyFact(Box::new(raw))
        })(input)
    } else if starts_with_keyword(input, "REMEMBER") {
        map(parse_remember, |raw| AqlStatement::Remember(Box::new(raw)))(input)
    } else {
        Err(Err::Error(ParseFailure::new(
            input,
            AqlParseErrorKind::Unexpected,
        )))
    }
}

fn parse_explain(input: Span<'_>) -> PResult<'_, AqlStatement<'_>> {
    let (input, _) = kw("EXPLAIN")(input)?;
    let (input, _) = ws1(input)?;
    if starts_with_keyword(input, "ANALYZE") {
        let (input, _) = kw("ANALYZE")(input)?;
        let (input, _) = ws1(input)?;
        return map(parse_statement, |statement| {
            AqlStatement::ExplainAnalyze(Box::new(statement))
        })(input);
    }
    map(parse_statement, |statement| {
        AqlStatement::Explain(Box::new(statement))
    })(input)
}

fn parse_verify_fact(input: Span<'_>) -> PResult<'_, RawVerifyFact<'_>> {
    let (input, _) = kw("VERIFY")(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("FACT")(input)?;
    let (input, _) = ws1(input)?;
    let (input, fact) = parse_quoted_string(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("IN")(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("BRAIN")(input)?;
    let (input, _) = ws1(input)?;
    let (input, brain) = parse_identifier(input)?;
    let (input, _) = delimited(multispace0, char(';'), multispace0)(input)?;
    Ok((input, RawVerifyFact { fact, brain }))
}

fn parse_remember(input: Span<'_>) -> PResult<'_, RawRemember<'_>> {
    let (input, _) = kw("REMEMBER")(input)?;
    let (input, _) = ws1(input)?;
    let (input, content) = parse_quoted_string(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("IN")(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("SCOPE")(input)?;
    let (input, _) = ws1(input)?;
    let (input, scope) = parse_identifier(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("AS")(input)?;
    let (input, _) = ws1(input)?;
    let (input, _) = kw("TYPE")(input)?;
    let (input, _) = ws1(input)?;
    let (input, memory_type) = parse_identifier(input)?;
    let (input, ttl) = opt(preceded(
        tuple((ws1, kw("TTL"), ws1)),
        map(
            terminated(parse_spanned_integer, tuple((ws1, kw("SECONDS")))),
            |ttl| Spanned::new(TtlValue::Seconds(ttl.node), ttl.span),
        ),
    ))(input)?;
    let (input, _) = delimited(multispace0, char(';'), multispace0)(input)?;
    Ok((
        input,
        RawRemember {
            content,
            scope,
            memory_type,
            ttl,
        },
    ))
}

pub(super) fn parse_identifier(input: Span<'_>) -> PResult<'_, Spanned<Identifier<'_>>> {
    spanned(map(
        recognize(pair(take_while1(is_ident_start), take_while(is_ident_rest))),
        |span: Span<'_>| Identifier::borrowed(span.fragment()),
    ))(input)
}

pub(super) fn parse_spanned_integer(input: Span<'_>) -> PResult<'_, Spanned<u64>> {
    spanned(parse_integer)(input)
}

pub(super) fn parse_integer(input: Span<'_>) -> PResult<'_, u64> {
    map_res(digit1, |span: Span<'_>| span.fragment().parse::<u64>())(input)
}

pub(super) fn parse_spanned_u32(input: Span<'_>) -> PResult<'_, Spanned<u32>> {
    spanned(parse_u32)(input)
}

pub(super) fn parse_u32(input: Span<'_>) -> PResult<'_, u32> {
    map_res(digit1, |span: Span<'_>| span.fragment().parse::<u32>())(input)
}

pub(super) fn parse_decimal(input: Span<'_>) -> PResult<'_, DecimalLiteral<'_>> {
    map(
        recognize(tuple((digit1, char('.'), digit1))),
        |span: Span<'_>| DecimalLiteral::borrowed(span.fragment()),
    )(input)
}

pub(super) fn spanned<'a, O, F>(mut parser: F) -> impl FnMut(Span<'a>) -> PResult<'a, Spanned<O>>
where
    F: FnMut(Span<'a>) -> PResult<'a, O>,
{
    move |input| {
        let start = input;
        let (input, node) = parser(input)?;
        Ok((input, Spanned::new(node, span_between(start, input))))
    }
}

pub(super) fn kw<'a>(keyword: &'static str) -> impl FnMut(Span<'a>) -> PResult<'a, Span<'a>> {
    move |input| match tag_no_case::<_, _, ParseFailure<'a>>(keyword)(input) {
        Ok(parsed) => Ok(parsed),
        Err(Err::Error(_)) => Err(Err::Error(ParseFailure::new(
            input,
            AqlParseErrorKind::ExpectedKeyword,
        ))),
        Err(Err::Failure(failure)) => Err(Err::Failure(failure)),
        Err(Err::Incomplete(needed)) => Err(Err::Incomplete(needed)),
    }
}

pub(super) fn ws1(input: Span<'_>) -> PResult<'_, Span<'_>> {
    take_while1(|value: char| value.is_whitespace())(input)
}

fn is_ident_start(value: char) -> bool {
    value == '_' || value.is_ascii_alphabetic()
}

fn is_ident_rest(value: char) -> bool {
    value == '_' || value == '-' || value == ':' || value.is_ascii_alphanumeric()
}

pub(super) fn starts_with_keyword(input: Span<'_>, keyword: &str) -> bool {
    input
        .fragment()
        .get(..keyword.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(keyword))
}

pub(super) fn source_span(input: Span<'_>) -> SourceSpan {
    SourceSpan {
        offset: input.location_offset(),
        line: input.location_line(),
        column: input.get_column(),
        len: 0,
    }
}

pub(super) fn span_between(start: Span<'_>, end: Span<'_>) -> SourceSpan {
    SourceSpan {
        offset: start.location_offset(),
        line: start.location_line(),
        column: start.get_column(),
        len: end
            .location_offset()
            .saturating_sub(start.location_offset()),
    }
}

impl<'a> ParseFailure<'a> {
    pub(super) fn new(span: Span<'a>, kind: AqlParseErrorKind) -> Self {
        Self { span, kind }
    }
}

impl<'a> ParseError<Span<'a>> for ParseFailure<'a> {
    fn from_error_kind(input: Span<'a>, _kind: ErrorKind) -> Self {
        Self::new(input, AqlParseErrorKind::Unexpected)
    }

    fn append(_input: Span<'a>, _kind: ErrorKind, other: Self) -> Self {
        other
    }
}

impl<'a> FromExternalError<Span<'a>, ParseIntError> for ParseFailure<'a> {
    fn from_external_error(input: Span<'a>, _kind: ErrorKind, _error: ParseIntError) -> Self {
        Self::new(input, AqlParseErrorKind::InvalidInteger)
    }
}
