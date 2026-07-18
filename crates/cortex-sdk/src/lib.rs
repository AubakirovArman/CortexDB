//! Blocking Rust HTTP client for CortexDB.
//!
//! `cortexdb-sdk` provides a synchronous, ergonomic client for the CortexDB
//! beta HTTP API. All methods return strongly-typed responses or
//! `serde_json::Value` for raw access.
//!
//! # Quickstart
//!
//! ```no_run
//! use cortex_sdk::CortexDbClient;
//!
//! let client = CortexDbClient::new("http://127.0.0.1:8181");
//! let health = client.health_response().unwrap();
//! println!("Server version: {}", health.server_version);
//! ```

mod aql;
mod aql_support;
#[cfg(feature = "async")]
mod async_client;
mod client;
mod context_pack;
mod generated;
mod grounded_answer;
mod http;
mod types;
mod verification;

pub use aql::{
    Aql, AqlBuildError, AqlRetrievalMode, RememberBuilder, RetrieveContextBuilder,
    VerifyFactBuilder,
};
#[cfg(feature = "async")]
pub use async_client::AsyncCortexDbClient;
pub use client::{CortexDbClient, SdkError, SdkResult};
pub use context_pack::{
    AnswerGroundingOptionsV1, AnswerGroundingReportV1, AnswerGroundingSpanV1,
    ContextPackAccessDecisionV1, ContextPackAnomalyV1, ContextPackCellV1, ContextPackExplainV1,
    ContextPackProvenanceV1, ContextPackSourceRefV1, ContextPackV1,
    CONTEXT_PACK_V1_REQUIRED_FIELDS, CONTEXT_PACK_V1_SCHEMA_VERSION,
};
pub use grounded_answer::{GroundedAnswerRequest, GroundedAnswerResponse};
pub use types::{
    transaction_outcome_status, AgentHandoffRequestBody, AgentHandoffResponse,
    AgentTransactionConflictResponse, AgentTransactionRequestBody, AgentTransactionResponse,
    AnnEvaluationResponse, AnnNoFallbackDecision, AnnSearchReport, AnswerGroundingOptionsResponse,
    AnswerGroundingReportResponse, AnswerGroundingSpanResponse, AqlCandidateCounts,
    AqlCellResponse, AqlExecutionOperator, AqlExecutionTrace, AqlExplainFilter, AqlExplainResponse,
    AqlLogicalPlan, AqlLogicalPlanNode, AqlResponse, CellLookupResponse, CellResponse,
    CheckpointResponse, ConsolidateCandidate, ConsolidateCommitRequestBody,
    ConsolidateCommitResponse, ConsolidateGroup, ConsolidatePlanRequestBody,
    ConsolidatePlanResponse, ConsolidateSourceRef, ContextPackAnomalyResponse,
    ContextPackCellResponse, ContextPackResponse, ContextSpanProvenanceResponse, DeleteJobResponse,
    ErrorCode, ErrorResponse, EvidenceResponse, ExplainResponse, GuardResponse, HealthResponse,
    HnswNoFallbackProfileResponse, IngestResponse, IngestionJobResponse, IngestionJobStatus,
    IngestionSkippedItem, IngestionSourceRefReport, IngestionValidationIssue,
    IngestionValidationReport, NumericConflictResponse, PutCellResponse, RememberResponse,
    ScoreComponentResponse, SearchAnswerGroundingReportResponse, SearchAnswerGroundingSpanResponse,
    SearchExplainItem, SearchExplainResponse, SearchExplainTermContribution, SearchResponse,
    SearchResult, SearchRoutingDecision, SourceRefResponse, StatsResponse, ValidationResponse,
    VectorAlgorithm, VerificationReportResponse, WriteBatchOperationRequest, WriteBatchRequest,
    WriteBatchResponse, WriteOpRequest,
};
pub use verification::{
    VerificationReportExt, VerifyConflict, VerifyEvidenceConflict, VerifyNumericConflict,
    VerifyOutputFormat, VerifyRequest, VerifyResult,
};

#[cfg(test)]
mod context_pack_tests;
#[cfg(test)]
mod grounded_answer_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod verification_tests;
