use std::borrow::Cow;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SourceSpan {
    pub offset: usize,
    pub line: u32,
    pub column: usize,
    pub len: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spanned<T> {
    pub node: T,
    pub span: SourceSpan,
}

impl<T> Spanned<T> {
    pub fn new(node: T, span: SourceSpan) -> Self {
        Self { node, span }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AqlString<'a> {
    pub value: Cow<'a, str>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identifier<'a> {
    pub value: Cow<'a, str>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecimalLiteral<'a> {
    pub raw: Cow<'a, str>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Literal<'a> {
    String(AqlString<'a>),
    Identifier(Identifier<'a>),
    List(Vec<Spanned<Literal<'a>>>),
    Integer(u64),
    Decimal(DecimalLiteral<'a>),
    Boolean(bool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Comparator {
    Eq,
    NotEq,
    Gt,
    Gte,
    Lt,
    Lte,
    In,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Condition<'a> {
    Predicate {
        field: Spanned<Identifier<'a>>,
        comparator: Spanned<Comparator>,
        literal: Spanned<Literal<'a>>,
    },
    Not(Box<Spanned<Condition<'a>>>),
    And(Box<Spanned<Condition<'a>>>, Box<Spanned<Condition<'a>>>),
    Or(Box<Spanned<Condition<'a>>>, Box<Spanned<Condition<'a>>>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Requirement<'a> {
    RequireCitations,
    MinConfidence(DecimalLiteral<'a>),
    SourceTrust(DecimalLiteral<'a>),
    Freshness(TtlValue),
    ValidAt(AqlString<'a>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Strategy {
    Fastest,
    HighestRecall,
    HighestPrecision,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TtlValue {
    Seconds(u64),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawRetrieveContext<'a> {
    pub task: Spanned<AqlString<'a>>,
    pub brain: Spanned<Identifier<'a>>,
    pub mode: Option<Spanned<crate::types::RetrievalMode>>,
    pub budget_tokens: Option<Spanned<u64>>,
    pub candidate_limit: Option<Spanned<u32>>,
    /// A5/A7.3: optional per-query MMR diversification lambda in Q16 `[0, 65535]`
    /// (`USING DIVERSITY <n>`). `None` leaves diversification to the database
    /// default (off), so an absent clause is byte-identical to before.
    pub diversity_lambda_q16: Option<Spanned<u64>>,
    /// A7.2: optional per-query two-stage dense rerank weight in Q16
    /// (`USING RERANK <n>`). `None` defers to the database default (off).
    pub rerank_weight_q16: Option<Spanned<u64>>,
    pub where_clause: Option<Spanned<Condition<'a>>>,
    pub requirements: Vec<Spanned<Requirement<'a>>>,
    pub strategy: Option<Spanned<Strategy>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawVerifyFact<'a> {
    pub fact: Spanned<AqlString<'a>>,
    pub brain: Spanned<Identifier<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawRemember<'a> {
    pub content: Spanned<AqlString<'a>>,
    pub scope: Spanned<Identifier<'a>>,
    pub memory_type: Spanned<Identifier<'a>>,
    pub ttl: Option<Spanned<TtlValue>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AqlStatement<'a> {
    RetrieveContext(Box<RawRetrieveContext<'a>>),
    VerifyFact(Box<RawVerifyFact<'a>>),
    Remember(Box<RawRemember<'a>>),
    Explain(Box<AqlStatement<'a>>),
    ExplainAnalyze(Box<AqlStatement<'a>>),
}

impl<'a> AqlString<'a> {
    pub fn borrowed(value: &'a str) -> Self {
        Self {
            value: Cow::Borrowed(value),
        }
    }

    pub fn owned(value: String) -> Self {
        Self {
            value: Cow::Owned(value),
        }
    }
}

impl<'a> Identifier<'a> {
    pub fn borrowed(value: &'a str) -> Self {
        Self {
            value: Cow::Borrowed(value),
        }
    }
}

impl<'a> DecimalLiteral<'a> {
    pub fn borrowed(raw: &'a str) -> Self {
        Self {
            raw: Cow::Borrowed(raw),
        }
    }
}
