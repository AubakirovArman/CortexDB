use cortex_aql::AgentView;
use cortex_engine::{
    ContextPackExportFormat, ContextPackOptions, ContextPipelineStageTrace, ContextPipelineTrace,
    Database,
};
use std::time::Instant;

use crate::auth::AuthRouteContext;
use crate::authz;
use crate::embedding;
use crate::memory;
use crate::receipt::ReceiptEmissionContext;
use crate::responses::{ContextTraceRequest, ContextTraceResponse, RouterError};

use crate::router::{query_param_decoded, query_param_opt_decoded};

mod request;
mod response;
mod view;

use request::context_request;
pub(crate) use response::map_context_pack;
pub(crate) use view::view_for_scope;

pub fn handle_context_shared(
    db: &Database,
    query: &str,
    body: &[u8],
    authenticated_view: Option<&AgentView>,
    auth_context: Option<&AuthRouteContext>,
    receipt_context: Option<&ReceiptEmissionContext>,
) -> Result<String, RouterError> {
    let scope = query_param_decoded(query, "scope").map_err(RouterError::BadRequest)?;
    let mut request = context_request(query, body)?;
    if (request.embed_query || embedding::semantic_aql_needs_query_vector(&request.retrieve_aql))
        && !embedding::aql_task_has_query_vector(&request.retrieve_aql)
    {
        let query_text = request
            .query_text
            .take()
            .or_else(|| embedding::retrieve_task_text(&request.retrieve_aql))
            .ok_or_else(|| {
                RouterError::BadRequest(
                    "embed_query requires RETRIEVE CONTEXT FOR TASK AQL".to_owned(),
                )
            })?;
        let vector = embedding::embed_query_from_env(&query_text)?;
        request.retrieve_aql = embedding::inject_query_vector(&request.retrieve_aql, &vector)?;
    }
    let view = authz::read_view_for_scope(&scope, authenticated_view)?;
    let format = context_output_format(query);

    match format.as_str() {
        "json" => {
            if let Some(receipt_context) = receipt_context {
                let evidence = db.context_pack_with_receipt_evidence_from_aql(
                    &request.retrieve_aql,
                    &view,
                    ContextPackOptions::default(),
                )?;
                let receipt = receipt_context.sign(&evidence, None)?;
                Ok(serde_json::to_string(&map_context_pack(
                    &evidence.pack,
                    auth_context,
                    Some(receipt),
                ))?)
            } else {
                let pack = db.context_pack_from_aql(
                    &request.retrieve_aql,
                    &view,
                    ContextPackOptions::default(),
                )?;
                Ok(serde_json::to_string(&map_context_pack(
                    &pack,
                    auth_context,
                    None,
                ))?)
            }
        }
        "prompt" => {
            let pack = db.context_pack_from_aql(
                &request.retrieve_aql,
                &view,
                ContextPackOptions::default(),
            )?;
            Ok(pack.export(ContextPackExportFormat::Prompt))
        }
        "markdown" => {
            let pack = db.context_pack_from_aql(
                &request.retrieve_aql,
                &view,
                ContextPackOptions::default(),
            )?;
            Ok(pack.export(ContextPackExportFormat::Markdown))
        }
        other => Err(RouterError::BadRequest(format!(
            "unsupported context format '{other}' (expected json, prompt, or markdown)"
        ))),
    }
}

pub fn handle_context_trace_shared(
    db: &Database,
    query: &str,
    body: &[u8],
    authenticated_view: Option<&AgentView>,
    auth_context: Option<&AuthRouteContext>,
    receipt_context: Option<&ReceiptEmissionContext>,
) -> Result<String, RouterError> {
    let scope = query_param_decoded(query, "scope").map_err(RouterError::BadRequest)?;
    let request = context_trace_request(body)?;
    let read_view = authz::read_view_for_scope(&scope, authenticated_view)?;
    let total_started = Instant::now();

    let context_started = Instant::now();
    let evidence = db.context_pack_with_receipt_evidence_from_aql(
        &request.retrieve_aql,
        &read_view,
        ContextPackOptions::default(),
    )?;
    let pack = &evidence.pack;
    let context_duration_ms = elapsed_ms(context_started);

    let verification = if let Some(verify_aql) = request.verify_aql.as_deref() {
        let verify_view = authz::verify_view_for_scope(&scope, authenticated_view)?;
        let started = Instant::now();
        let report = db.verify_fact_aql(verify_aql, &verify_view)?;
        Some((report, elapsed_ms(started)))
    } else {
        None
    };

    let mut stages = vec![
        ContextPipelineStageTrace::new(
            "retrieve",
            None,
            0,
            (pack.cells.len() + pack.anomalies.len()) as u64,
            vec![
                "compiled RETRIEVE CONTEXT AQL and evaluated bitmap candidates".to_owned(),
                "exact candidate count is not exposed by this trace version".to_owned(),
            ],
        ),
        ContextPipelineStageTrace::new(
            "pack",
            Some(context_duration_ms),
            (pack.cells.len() + pack.anomalies.len()) as u64,
            pack.cells.len() as u64,
            vec![
                "packed candidate cells into ContextPack with token budget and citation policy"
                    .to_owned(),
            ],
        ),
    ];

    let verification_report = verification.as_ref().map(|(report, _)| report);
    if let Some((report, duration_ms)) = verification.as_ref() {
        stages.push(ContextPipelineStageTrace::new(
            "verify",
            Some(*duration_ms),
            pack.cells.len() as u64,
            (report.evidence.len() + report.contradicting_evidence.len()) as u64,
            vec!["executed VERIFY FACT against readable database evidence".to_owned()],
        ));
    }

    let trace = ContextPipelineTrace::from_pack(
        pack,
        verification_report,
        stages,
        Some(elapsed_ms(total_started)),
    );
    let context_receipt = receipt_context
        .map(|ctx| ctx.sign(&evidence, verification_report))
        .transpose()?;
    let response = ContextTraceResponse {
        schema_version: "context_trace.v1",
        context: map_context_pack(pack, auth_context, context_receipt),
        verification: verification
            .as_ref()
            .map(|(report, _)| memory::map_verification_report(report, db, None)),
        trace,
    };
    Ok(serde_json::to_string(&response)?)
}

fn context_trace_request(body: &[u8]) -> Result<ContextTraceRequest, RouterError> {
    let raw = String::from_utf8_lossy(body);
    let trimmed = raw.trim();
    if trimmed.starts_with('{') {
        let request: ContextTraceRequest = serde_json::from_str(trimmed)
            .map_err(|err| RouterError::BadRequest(err.to_string()))?;
        if request.retrieve_aql.trim().is_empty() {
            return Err(RouterError::BadRequest(
                "retrieve_aql must not be empty".to_owned(),
            ));
        }
        if request
            .verify_aql
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(RouterError::BadRequest(
                "verify_aql must not be empty when provided".to_owned(),
            ));
        }
        Ok(request)
    } else if trimmed.is_empty() {
        Err(RouterError::BadRequest(
            "context trace request body must contain retrieve AQL".to_owned(),
        ))
    } else {
        Ok(ContextTraceRequest {
            retrieve_aql: trimmed.to_owned(),
            verify_aql: None,
        })
    }
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}

fn context_output_format(query: &str) -> String {
    query_param_opt_decoded(query, "format")
        .unwrap_or_else(|| "json".to_owned())
        .trim()
        .to_ascii_lowercase()
}
