use cortex_sdk::{Aql, AqlRetrievalMode, CortexDbClient};

use crate::config::McpConfig;
use crate::tools::{
    RememberArgs, RetrieveContextArgs, ToolCallResult, ToolExecutor, VerifyFactArgs,
};

#[derive(Clone, Debug)]
pub struct SdkToolExecutor {
    client: CortexDbClient,
    default_scope: String,
    default_brain: String,
}

impl SdkToolExecutor {
    pub fn from_config(config: McpConfig) -> Self {
        let mut client = CortexDbClient::new(config.base_url).with_tenant(config.tenant);
        if let Some(token) = config.auth_token {
            client = client.with_token(token);
        }
        Self {
            client,
            default_scope: config.default_scope,
            default_brain: config.default_brain,
        }
    }

    fn scope(&self, override_scope: Option<String>) -> String {
        override_scope.unwrap_or_else(|| self.default_scope.clone())
    }

    fn brain(&self, override_brain: Option<String>) -> String {
        override_brain.unwrap_or_else(|| self.default_brain.clone())
    }
}

impl ToolExecutor for SdkToolExecutor {
    fn retrieve_context(&self, args: RetrieveContextArgs) -> Result<ToolCallResult, String> {
        let scope = self.scope(args.scope);
        let mut builder = Aql::retrieve_context(args.task, self.brain(args.brain));
        if let Some(mode) = args.mode.as_deref() {
            builder = builder.mode(parse_mode(mode)?);
        }
        if let Some(budget) = args.budget_tokens {
            builder = builder.budget_tokens(budget);
        }
        if let Some(limit) = args.candidate_limit {
            builder = builder.limit_candidates(limit);
        }
        if let Some(where_clause) = args.where_clause {
            builder = builder.where_clause(where_clause);
        }
        if args.require_citations.unwrap_or(false) {
            builder = builder.require_citations();
        }
        let statement = builder.build().map_err(|error| error.to_string())?;
        match args.format.as_deref().unwrap_or("json") {
            "json" => self
                .client
                .context(&scope, &statement)
                .and_then(|response| serde_json::to_string_pretty(&response).map_err(Into::into))
                .map(ToolCallResult::text)
                .map_err(|error| error.to_string()),
            "prompt" => self
                .client
                .context_prompt(&scope, &statement)
                .map(ToolCallResult::text)
                .map_err(|error| error.to_string()),
            "markdown" => self
                .client
                .context_markdown(&scope, &statement)
                .map(ToolCallResult::text)
                .map_err(|error| error.to_string()),
            other => Err(format!("unsupported context format: {other}")),
        }
    }

    fn verify_fact(&self, args: VerifyFactArgs) -> Result<ToolCallResult, String> {
        let scope = self.scope(args.scope);
        let statement = Aql::verify_fact(args.fact, self.brain(args.brain))
            .build()
            .map_err(|error| error.to_string())?;
        self.client
            .verify(&scope, &statement)
            .and_then(|response| serde_json::to_string_pretty(&response).map_err(Into::into))
            .map(ToolCallResult::text)
            .map_err(|error| error.to_string())
    }

    fn remember(&self, args: RememberArgs) -> Result<ToolCallResult, String> {
        let scope = self.scope(args.scope.clone());
        let memory_scope = args.scope.unwrap_or_else(|| self.default_scope.clone());
        let memory_type = args.memory_type.unwrap_or_else(|| "observation".to_owned());
        let mut builder = Aql::remember(args.content, memory_scope, memory_type);
        if let Some(ttl_seconds) = args.ttl_seconds {
            builder = builder.ttl_seconds(ttl_seconds);
        }
        let statement = builder.build().map_err(|error| error.to_string())?;
        self.client
            .remember(&scope, &statement)
            .and_then(|response| serde_json::to_string_pretty(&response).map_err(Into::into))
            .map(ToolCallResult::text)
            .map_err(|error| error.to_string())
    }
}

fn parse_mode(value: &str) -> Result<AqlRetrievalMode, String> {
    match value {
        "fast" => Ok(AqlRetrievalMode::Fast),
        "balanced" => Ok(AqlRetrievalMode::Balanced),
        "semantic" => Ok(AqlRetrievalMode::Semantic),
        "audit" => Ok(AqlRetrievalMode::Audit),
        _ => Err(format!("unsupported retrieval mode: {value}")),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_mode;

    #[test]
    fn rejects_unknown_mode() {
        assert!(parse_mode("oracle").is_err());
    }
}
