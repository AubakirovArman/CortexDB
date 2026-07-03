use std::future::Future;

use super::transport::decode_value;
use super::AsyncCortexDbClient;
use crate::http::path;
use crate::{
    AgentHandoffRequestBody, AgentHandoffResponse, AgentTransactionRequestBody,
    AgentTransactionResponse, AqlResponse, ConsolidateCommitRequestBody, ConsolidateCommitResponse,
    ConsolidatePlanRequestBody, ConsolidatePlanResponse, ContextPackResponse,
    GroundedAnswerRequest, GroundedAnswerResponse, RememberResponse, SdkResult,
    VerificationReportResponse, VerifyRequest,
};

impl AsyncCortexDbClient {
    /// Plan memory consolidation — the stale episodic groups to summarize
    /// (F04-B4.4 / B4.2). Read-only.
    pub async fn consolidate_plan(
        &self,
        request: &ConsolidatePlanRequestBody,
    ) -> SdkResult<ConsolidatePlanResponse> {
        let body = serde_json::to_string(request)?;
        decode_value(
            self.post(&path("/v1/memory/consolidate/plan", &[]), &body)
                .await?,
        )
    }

    /// Commit an externally-generated consolidation summary (F04-B4.4 / B4.3).
    pub async fn consolidate_commit(
        &self,
        request: &ConsolidateCommitRequestBody,
    ) -> SdkResult<ConsolidateCommitResponse> {
        let body = serde_json::to_string(request)?;
        decode_value(
            self.post(&path("/v1/memory/consolidate/commit", &[]), &body)
                .await?,
        )
    }

    pub async fn aql(&self, scope: &str, statement: &str) -> SdkResult<serde_json::Value> {
        self.post(&path("/v1/aql", &[("scope", scope)]), statement)
            .await
    }

    /// Commit an optimistic-concurrency agent transaction (F04-B6.3). A conflict is
    /// a normal response with `outcome == "conflict"`, not an HTTP error — read
    /// `outcome` rather than relying on the status code.
    pub async fn agent_transaction(
        &self,
        request: &AgentTransactionRequestBody,
    ) -> SdkResult<AgentTransactionResponse> {
        let body = serde_json::to_string(request)?;
        decode_value(self.post(&path("/v1/transactions", &[]), &body).await?)
    }

    /// Commit a durable SharedSequenced agent handoff (F04-B6.3 / F08-B6.1).
    pub async fn agent_handoff(
        &self,
        request: &AgentHandoffRequestBody,
    ) -> SdkResult<AgentHandoffResponse> {
        let body = serde_json::to_string(request)?;
        decode_value(self.post(&path("/v1/handoff", &[]), &body).await?)
    }

    pub async fn aql_response(&self, scope: &str, statement: &str) -> SdkResult<AqlResponse> {
        decode_value(self.aql(scope, statement).await?)
    }

    pub async fn context(&self, scope: &str, statement: &str) -> SdkResult<serde_json::Value> {
        self.post(&path("/v1/context", &[("scope", scope)]), statement)
            .await
    }

    pub async fn context_response(
        &self,
        scope: &str,
        statement: &str,
    ) -> SdkResult<ContextPackResponse> {
        decode_value(self.context(scope, statement).await?)
    }

    pub async fn context_prompt(&self, scope: &str, statement: &str) -> SdkResult<String> {
        self.post_text(
            &path("/v1/context", &[("scope", scope), ("format", "prompt")]),
            statement,
        )
        .await
    }

    pub async fn context_markdown(&self, scope: &str, statement: &str) -> SdkResult<String> {
        self.post_text(
            &path("/v1/context", &[("scope", scope), ("format", "markdown")]),
            statement,
        )
        .await
    }

    pub async fn verify(&self, scope: &str, statement: &str) -> SdkResult<serde_json::Value> {
        self.post(&path("/v1/verify", &[("scope", scope)]), statement)
            .await
    }

    pub async fn verify_response(
        &self,
        scope: &str,
        statement: &str,
    ) -> SdkResult<VerificationReportResponse> {
        decode_value(self.verify(scope, statement).await?)
    }

    pub async fn verify_request_response(
        &self,
        request: &VerifyRequest,
    ) -> SdkResult<VerificationReportResponse> {
        let request = request.clone().json();
        decode_value(self.post(&request.path(), request.statement()).await?)
    }

    pub async fn verify_request_export(&self, request: &VerifyRequest) -> SdkResult<String> {
        self.post_text(&request.path(), request.statement()).await
    }

    pub async fn answer_with_grounded_context<F, Fut>(
        &self,
        request: GroundedAnswerRequest,
        answerer: F,
    ) -> SdkResult<GroundedAnswerResponse>
    where
        F: FnOnce(&ContextPackResponse) -> Fut,
        Fut: Future<Output = SdkResult<String>>,
    {
        let retrieve_statement = request.retrieve_statement()?;
        let context = self
            .context_response(request.scope(), &retrieve_statement)
            .await?;
        let answer = answerer(&context).await?;
        let verify_statement = request.verify_statement(&answer)?;
        let verification = match verify_statement.as_deref() {
            Some(statement) => Some(self.verify_response(request.scope(), statement).await?),
            None => None,
        };
        Ok(GroundedAnswerResponse::from_context_answer(
            &request,
            retrieve_statement,
            verify_statement,
            context,
            answer,
            verification,
        ))
    }

    pub async fn remember(&self, scope: &str, statement: &str) -> SdkResult<serde_json::Value> {
        self.post(&path("/v1/remember", &[("scope", scope)]), statement)
            .await
    }

    pub async fn remember_response(
        &self,
        scope: &str,
        statement: &str,
    ) -> SdkResult<RememberResponse> {
        decode_value(self.remember(scope, statement).await?)
    }
}
