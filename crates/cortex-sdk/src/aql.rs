use crate::aql_support::{
    quote_string, validate_identifier, validate_optional_decimal, validate_optional_fragment,
};
pub use crate::aql_support::{AqlBuildError, AqlRetrievalMode};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetrieveContextBuilder {
    task: String,
    brain: String,
    mode: Option<AqlRetrievalMode>,
    budget_tokens: Option<u64>,
    candidate_limit: Option<u32>,
    where_clause: Option<String>,
    require_citations: bool,
    min_confidence: Option<String>,
    source_trust: Option<String>,
    freshness_seconds: Option<u64>,
    explain: bool,
}

impl RetrieveContextBuilder {
    pub fn new(task: impl Into<String>, brain: impl Into<String>) -> Self {
        Self {
            task: task.into(),
            brain: brain.into(),
            mode: None,
            budget_tokens: None,
            candidate_limit: None,
            where_clause: None,
            require_citations: false,
            min_confidence: None,
            source_trust: None,
            freshness_seconds: None,
            explain: false,
        }
    }

    pub fn mode(mut self, mode: AqlRetrievalMode) -> Self {
        self.mode = Some(mode);
        self
    }

    pub fn budget_tokens(mut self, budget_tokens: u64) -> Self {
        self.budget_tokens = Some(budget_tokens);
        self
    }

    pub fn limit_candidates(mut self, candidate_limit: u32) -> Self {
        self.candidate_limit = Some(candidate_limit);
        self
    }

    pub fn where_clause(mut self, where_clause: impl Into<String>) -> Self {
        self.where_clause = Some(where_clause.into());
        self
    }

    pub fn require_citations(mut self) -> Self {
        self.require_citations = true;
        self
    }

    pub fn require_confidence_at_least(mut self, decimal: impl Into<String>) -> Self {
        self.min_confidence = Some(decimal.into());
        self
    }

    pub fn require_source_trust_at_least(mut self, decimal: impl Into<String>) -> Self {
        self.source_trust = Some(decimal.into());
        self
    }

    pub fn require_freshness_seconds(mut self, seconds: u64) -> Self {
        self.freshness_seconds = Some(seconds);
        self
    }

    pub fn explain(mut self) -> Self {
        self.explain = true;
        self
    }

    pub fn build(&self) -> Result<String, AqlBuildError> {
        validate_identifier("brain", &self.brain)?;
        validate_optional_fragment("where_clause", self.where_clause.as_deref())?;
        validate_optional_decimal("min_confidence", self.min_confidence.as_deref())?;
        validate_optional_decimal("source_trust", self.source_trust.as_deref())?;

        let mut statement = String::new();
        if self.explain {
            statement.push_str("EXPLAIN ");
        }
        statement.push_str("RETRIEVE CONTEXT FOR TASK ");
        statement.push_str(&quote_string(&self.task));
        statement.push_str(" IN BRAIN ");
        statement.push_str(&self.brain);
        if let Some(mode) = self.mode {
            statement.push_str(" USING MODE ");
            statement.push_str(mode.as_str());
        }
        if let Some(budget) = self.budget_tokens {
            statement.push_str(" BUDGET ");
            statement.push_str(&budget.to_string());
            statement.push_str(" TOKENS");
        }
        if let Some(limit) = self.candidate_limit {
            statement.push_str(" LIMIT ");
            statement.push_str(&limit.to_string());
            statement.push_str(" CANDIDATES");
        }
        if let Some(where_clause) = self.where_clause.as_deref() {
            statement.push_str(" WHERE ");
            statement.push_str(where_clause.trim());
        }
        if self.require_citations {
            statement.push_str(" REQUIRE citations");
        }
        if let Some(decimal) = self.min_confidence.as_deref() {
            statement.push_str(" REQUIRE confidence >= ");
            statement.push_str(decimal);
        }
        if let Some(decimal) = self.source_trust.as_deref() {
            statement.push_str(" REQUIRE source_trust >= ");
            statement.push_str(decimal);
        }
        if let Some(seconds) = self.freshness_seconds {
            statement.push_str(" REQUIRE freshness <= ");
            statement.push_str(&seconds.to_string());
            statement.push_str(" SECONDS");
        }
        statement.push(';');
        Ok(statement)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifyFactBuilder {
    fact: String,
    brain: String,
}

impl VerifyFactBuilder {
    pub fn new(fact: impl Into<String>, brain: impl Into<String>) -> Self {
        Self {
            fact: fact.into(),
            brain: brain.into(),
        }
    }

    pub fn build(&self) -> Result<String, AqlBuildError> {
        validate_identifier("brain", &self.brain)?;
        Ok(format!(
            "VERIFY FACT {} IN BRAIN {};",
            quote_string(&self.fact),
            self.brain
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RememberBuilder {
    content: String,
    scope: String,
    memory_type: String,
    ttl_seconds: Option<u64>,
}

impl RememberBuilder {
    pub fn new(
        content: impl Into<String>,
        scope: impl Into<String>,
        memory_type: impl Into<String>,
    ) -> Self {
        Self {
            content: content.into(),
            scope: scope.into(),
            memory_type: memory_type.into(),
            ttl_seconds: None,
        }
    }

    pub fn ttl_seconds(mut self, seconds: u64) -> Self {
        self.ttl_seconds = Some(seconds);
        self
    }

    pub fn build(&self) -> Result<String, AqlBuildError> {
        validate_identifier("scope", &self.scope)?;
        validate_identifier("memory_type", &self.memory_type)?;
        let mut statement = format!(
            "REMEMBER {} IN SCOPE {} AS TYPE {}",
            quote_string(&self.content),
            self.scope,
            self.memory_type
        );
        if let Some(seconds) = self.ttl_seconds {
            statement.push_str(" TTL ");
            statement.push_str(&seconds.to_string());
            statement.push_str(" SECONDS");
        }
        statement.push(';');
        Ok(statement)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Aql;

impl Aql {
    pub fn retrieve_context(
        task: impl Into<String>,
        brain: impl Into<String>,
    ) -> RetrieveContextBuilder {
        RetrieveContextBuilder::new(task, brain)
    }

    pub fn verify_fact(fact: impl Into<String>, brain: impl Into<String>) -> VerifyFactBuilder {
        VerifyFactBuilder::new(fact, brain)
    }

    pub fn remember(
        content: impl Into<String>,
        scope: impl Into<String>,
        memory_type: impl Into<String>,
    ) -> RememberBuilder {
        RememberBuilder::new(content, scope, memory_type)
    }
}
