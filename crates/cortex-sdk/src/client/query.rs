use super::transport::decode_value;
use super::CortexDbClient;
use crate::http::path;
use crate::{
    AqlResponse, ContextPackResponse, GroundedAnswerRequest, GroundedAnswerResponse,
    RememberResponse, SdkResult, VerificationReportResponse, VerifyRequest,
};

impl CortexDbClient {
    pub fn aql(&self, scope: &str, statement: &str) -> SdkResult<serde_json::Value> {
        self.post(&path("/v1/aql", &[("scope", scope)]), statement)
    }

    pub fn aql_response(&self, scope: &str, statement: &str) -> SdkResult<AqlResponse> {
        decode_value(self.aql(scope, statement)?)
    }

    pub fn context(&self, scope: &str, statement: &str) -> SdkResult<serde_json::Value> {
        self.post(&path("/v1/context", &[("scope", scope)]), statement)
    }

    pub fn context_response(&self, scope: &str, statement: &str) -> SdkResult<ContextPackResponse> {
        decode_value(self.context(scope, statement)?)
    }

    pub fn context_prompt(&self, scope: &str, statement: &str) -> SdkResult<String> {
        self.post_text(
            &path("/v1/context", &[("scope", scope), ("format", "prompt")]),
            statement,
        )
    }

    pub fn context_markdown(&self, scope: &str, statement: &str) -> SdkResult<String> {
        self.post_text(
            &path("/v1/context", &[("scope", scope), ("format", "markdown")]),
            statement,
        )
    }

    pub fn verify(&self, scope: &str, statement: &str) -> SdkResult<serde_json::Value> {
        self.post(&path("/v1/verify", &[("scope", scope)]), statement)
    }

    pub fn verify_response(
        &self,
        scope: &str,
        statement: &str,
    ) -> SdkResult<VerificationReportResponse> {
        decode_value(self.verify(scope, statement)?)
    }

    pub fn verify_request_response(
        &self,
        request: &VerifyRequest,
    ) -> SdkResult<VerificationReportResponse> {
        let request = request.clone().json();
        decode_value(self.post(&request.path(), request.statement())?)
    }

    pub fn verify_request_export(&self, request: &VerifyRequest) -> SdkResult<String> {
        self.post_text(&request.path(), request.statement())
    }

    pub fn answer_with_grounded_context(
        &self,
        request: GroundedAnswerRequest,
        answerer: impl FnOnce(&ContextPackResponse) -> SdkResult<String>,
    ) -> SdkResult<GroundedAnswerResponse> {
        let retrieve_statement = request.retrieve_statement()?;
        let context = self.context_response(request.scope(), &retrieve_statement)?;
        let answer = answerer(&context)?;
        let verify_statement = request.verify_statement(&answer)?;
        let verification = match verify_statement.as_deref() {
            Some(statement) => Some(self.verify_response(request.scope(), statement)?),
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

    pub fn remember(&self, scope: &str, statement: &str) -> SdkResult<serde_json::Value> {
        self.post(&path("/v1/remember", &[("scope", scope)]), statement)
    }

    pub fn remember_response(&self, scope: &str, statement: &str) -> SdkResult<RememberResponse> {
        decode_value(self.remember(scope, statement)?)
    }
}
