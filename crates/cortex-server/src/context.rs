use cortex_aql::AgentView;
use cortex_engine::{
    ContextPack, ContextPackExportFormat, ContextPackOptions, ContextPipelineStageTrace,
    ContextPipelineTrace, Database,
};
use std::time::Instant;

use crate::auth::AuthRouteContext;
use crate::authz;
use crate::embedding;
use crate::memory;
use crate::responses::{
    ContextAccessDecisionResponse, ContextPackAnomalyResponse, ContextPackCellResponse,
    ContextPackResponse, ContextSpanProvenanceResponse, ContextTraceRequest, ContextTraceResponse,
    ExplainResponse, RouterError, ScoreComponentResponse, SourceRefResponse,
};

use crate::router::{query_param_decoded, query_param_opt_decoded};

mod request;
mod view;

use request::context_request;
pub(crate) use view::view_for_scope;

pub fn handle_context_shared(
    db: &Database,
    query: &str,
    body: &[u8],
    authenticated_view: Option<&AgentView>,
    auth_context: Option<&AuthRouteContext>,
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
    let pack =
        db.context_pack_from_aql(&request.retrieve_aql, &view, ContextPackOptions::default())?;

    match context_output_format(query).as_str() {
        "json" => Ok(serde_json::to_string(&map_context_pack(
            &pack,
            auth_context,
        ))?),
        "prompt" => Ok(pack.export(ContextPackExportFormat::Prompt)),
        "markdown" => Ok(pack.export(ContextPackExportFormat::Markdown)),
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
) -> Result<String, RouterError> {
    let scope = query_param_decoded(query, "scope").map_err(RouterError::BadRequest)?;
    let request = context_trace_request(body)?;
    let read_view = authz::read_view_for_scope(&scope, authenticated_view)?;
    let total_started = Instant::now();

    let context_started = Instant::now();
    let pack = db.context_pack_from_aql(
        &request.retrieve_aql,
        &read_view,
        ContextPackOptions::default(),
    )?;
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
        &pack,
        verification_report,
        stages,
        Some(elapsed_ms(total_started)),
    );
    let response = ContextTraceResponse {
        schema_version: "context_trace.v1",
        context: map_context_pack(&pack, auth_context),
        verification: verification
            .as_ref()
            .map(|(report, _)| memory::map_verification_report(report, db)),
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

pub(crate) fn map_context_pack(
    pack: &ContextPack,
    auth_context: Option<&AuthRouteContext>,
) -> ContextPackResponse {
    let cells = pack
        .cells
        .iter()
        .map(|cell| {
            let source_ref = cell.metadata.source_ref.as_ref().map(map_source_ref);
            let provenance =
                cell.provenance
                    .as_ref()
                    .map(|provenance| ContextSpanProvenanceResponse {
                        source_cell_id: provenance.source_cell_id.0,
                        source_byte_start: provenance.source_byte_start,
                        source_byte_end: provenance.source_byte_end,
                        source_line_start: provenance.source_line_start,
                        source_line_end: provenance.source_line_end,
                        source_ref: provenance.source_ref.as_ref().map(map_source_ref),
                    });

            let explain = cell.explain.as_ref().map(|exp| ExplainResponse {
                score: exp.score,
                matched_terms: exp.matched_terms.clone(),
                why_selected: exp.why_selected.clone(),
                score_components: exp
                    .score_components
                    .iter()
                    .map(|component| ScoreComponentResponse {
                        name: component.name.clone(),
                        value: component.value,
                        contribution: component.contribution,
                        reason: component.reason.clone(),
                    })
                    .collect(),
                base_bm25: exp.base_bm25,
                source_trust_q16: exp.source_trust_q16,
                source_trust_category: exp.source_trust_category.as_str().to_owned(),
                source_trust_bonus: exp.source_trust_bonus,
                source_freshness_q16: exp.source_freshness_q16,
                source_freshness_category: exp.source_freshness_category.as_str().to_owned(),
                source_freshness_bonus: exp.source_freshness_bonus,
                redundancy_penalty: exp.redundancy_penalty,
            });

            ContextPackCellResponse {
                cell_id: cell.cell_id.0,
                estimated_tokens: cell.estimated_tokens,
                citation: cell.citation.clone(),
                payload_text: String::from_utf8_lossy(&cell.payload).into_owned(),
                explain,
                source_ref,
                provenance,
                access_decision: cell.access_decision.as_ref().map(|decision| {
                    ContextAccessDecisionResponse {
                        cell_id: decision.cell_id.0,
                        decision: decision.decision.as_str().to_owned(),
                        policy: decision.policy.clone(),
                        reason: decision.reason.clone(),
                        scope: decision.scope.clone(),
                        scope_id: decision.scope_id,
                        agent_id: decision.agent_id,
                        principal_id: auth_context.and_then(|ctx| ctx.principal_id.clone()),
                        auth_role: auth_context
                            .and_then(|ctx| ctx.role.map(|role| role.as_str().to_owned())),
                    }
                }),
            }
        })
        .collect();

    let anomalies = pack
        .anomalies
        .iter()
        .map(|anom| ContextPackAnomalyResponse {
            cell_id: anom.cell_id.map(|cid| cid.0),
            code: anom.code.as_str().to_owned(),
            message: anom.message.clone(),
            why_excluded: anom.why_excluded.clone(),
        })
        .collect();

    ContextPackResponse {
        schema_version: "context_pack.v1",
        token_budget_tokens: pack.token_budget_tokens,
        estimated_tokens: pack.estimated_tokens,
        truncated: pack.truncated,
        citations_required: pack.citations_required,
        answerability_q16: pack.answerability_q16,
        conflict_visibility_q16: pack.conflict_visibility_q16,
        visible_conflict_count: pack.visible_conflict_count,
        cells,
        anomalies,
    }
}

fn map_source_ref(sr: &cortex_engine::SourceRef) -> SourceRefResponse {
    SourceRefResponse {
        source_id: sr.source_id.clone(),
        source_url: sr.source_url.clone(),
        document_id: sr.document_id.clone(),
        page: sr.page,
        row: sr.row,
        cell_range: sr.cell_range.clone(),
        json_path: sr.json_path.clone(),
        confidence_q16: sr.confidence_q16,
    }
}
