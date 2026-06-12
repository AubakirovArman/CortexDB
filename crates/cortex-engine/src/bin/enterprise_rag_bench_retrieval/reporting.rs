use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use cortex_aql::{AgentId, AgentView, BrainId, MemoryType, RetrievalMode, Q16_ZERO};
use cortex_engine::{scope_id, Database};
use serde_json::{json, Value};

use super::args::{Args, BenchmarkRetrievalMode};
use super::io::read_jsonl;
use super::metrics::{RunReportMetrics, VectorReadinessReport};
use super::prefilter::ExternalPrefilterRetrieval;
use super::vectors::payload_has_vector;
use super::{
    doc_id_to_cell_id, round_pct, throughput_per_sec, CLEAN_PREFILTER_RETRIEVAL_FIELDS,
    ORACLE_FIELDS,
};

#[cfg(test)]
pub(super) fn build_benchmark_aql(
    query: &str,
    query_vector: Option<&Vec<i16>>,
    limit: usize,
) -> String {
    let task = benchmark_task(query, query_vector);
    let mode = if let Some(vector) = query_vector {
        let _ = vector;
        "semantic"
    } else {
        "balanced"
    };
    format!(
        "RETRIEVE CONTEXT FOR TASK {} IN BRAIN enterprise_rag USING MODE {mode} WHERE space = bench:enterprise_rag AND status = \"ready\" AND type = \"document_block\" LIMIT {} CANDIDATES;",
        quote_aql_string(&task),
        limit.max(1)
    )
}

pub(super) fn benchmark_task(query: &str, query_vector: Option<&Vec<i16>>) -> String {
    let mut task = query.to_owned();
    if let Some(vector) = query_vector {
        task.push('\n');
        task.push_str("query_vector=");
        task.push_str(&vector_literal(vector));
    }
    task
}

fn vector_literal(vector: &[i16]) -> String {
    vector
        .iter()
        .map(i16::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
pub(super) fn quote_aql_string(value: &str) -> String {
    let mut quoted = String::with_capacity(value.len() + 2);
    quoted.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => quoted.push_str("\\\\"),
            '"' => quoted.push_str("\\\""),
            '\n' => quoted.push_str("\\n"),
            '\r' => quoted.push_str("\\r"),
            '\t' => quoted.push_str("\\t"),
            ch => quoted.push(ch),
        }
    }
    quoted.push('"');
    quoted
}

pub(super) fn report_payload(
    questions: &[Value],
    uuid_index: &BTreeMap<String, String>,
    args: &Args,
    metrics: &RunReportMetrics,
) -> Value {
    let mut by_type = BTreeMap::<String, usize>::new();
    if !args.official_clean {
        for question in questions {
            let question_type = question
                .get("question_type")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_owned();
            *by_type.entry(question_type).or_default() += 1;
        }
    }
    json!({
        "schema_version": "cortexdb.enterprise_rag_bench.retrieval_report.v1",
        "questions": questions.len(),
        "documents_indexed": args.max_documents.unwrap_or(uuid_index.len()),
        "top_k": args.top_k,
        "batch_size": args.batch_size,
        "official_clean": args.official_clean,
        "retrieval_mode": args.retrieval_mode.as_str(),
        "rerank_mode": args.rerank_mode.as_str(),
        "skip_ingest": args.skip_ingest,
        "skip_checkpoint": args.skip_checkpoint,
        "query_vectors": args.query_vectors.as_ref().map(|path| path.display().to_string()),
        "document_vectors": args.document_vectors.as_ref().map(|path| path.display().to_string()),
        "prefilter_retrieval": args.prefilter_retrieval.as_ref().map(|path| path.display().to_string()),
        "disable_search_prefilter": args.disable_search_prefilter,
        "source_payload_prefilter": metrics.source_payload_prefilter,
        "vector_readiness": metrics.vector_readiness.as_ref().map(|readiness| {
            json!({
                "required_for_mode": readiness.required_for_mode,
                "ready": readiness.ready,
                "query_vector_rows": readiness.query_vector_rows,
                "document_vector_rows_loaded": readiness.document_vector_rows_loaded,
                "external_document_vectors_available": readiness.external_document_vectors_available,
                "sampled_documents": readiness.sampled_documents,
                "sampled_documents_with_vectors": readiness.sampled_documents_with_vectors,
                "document_vector_sample_coverage_pct": readiness.document_vector_sample_coverage_pct,
                "warnings": &readiness.warnings,
            })
        }),
        "diversity": metrics.diversity.to_json(),
        "source_type_filter": !args.official_clean && args.retrieval_mode == BenchmarkRetrievalMode::CachedLexical,
        "performance": {
            "total_duration_ms": metrics.total_duration_ms,
            "ingest": {
                "documents_indexed": metrics.documents_indexed,
                "duration_ms": metrics.ingest_duration_ms,
                "throughput_docs_per_sec": throughput_per_sec(metrics.documents_indexed, metrics.ingest_duration_ms),
            },
            "checkpoint": {
                "duration_ms": metrics.checkpoint_duration_ms,
            },
            "retrieval": {
                "questions": metrics.questions_retrieved,
                "duration_ms": metrics.retrieval_duration_ms,
                "throughput_questions_per_sec": throughput_per_sec(metrics.questions_retrieved, metrics.retrieval_duration_ms),
            },
            "resource_usage": process_memory_report(),
        },
        "by_question_type": by_type,
        "output": args.output,
        "db_root": args.db_root,
        "runner": "cortex-engine-official-clean-retrieval",
    })
}

pub(super) fn required_str<'a>(
    row: &'a Value,
    field: &str,
    index: usize,
) -> Result<&'a str, String> {
    row.get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("row {} missing non-empty {field}", index + 1))
}

pub(super) fn reject_oracle_fields(rows: &[Value]) -> Result<(), String> {
    for (index, row) in rows.iter().enumerate() {
        let Some(object) = row.as_object() else {
            continue;
        };
        let forbidden = ORACLE_FIELDS
            .iter()
            .copied()
            .filter(|field| object.contains_key(*field))
            .collect::<Vec<_>>();
        if !forbidden.is_empty() {
            return Err(format!(
                "official-clean question row {} has forbidden oracle fields: {}",
                index + 1,
                forbidden.join(", ")
            ));
        }
    }
    Ok(())
}

pub(super) fn assess_vector_readiness(
    db: &Database,
    uuid_index: &BTreeMap<String, String>,
    retrieval_mode: BenchmarkRetrievalMode,
    question_count: usize,
    query_vector_rows: usize,
    document_vector_rows_loaded: usize,
    source_payload_prefilter: bool,
) -> Result<VectorReadinessReport, String> {
    let required_for_mode = matches!(
        retrieval_mode,
        BenchmarkRetrievalMode::EngineHybrid | BenchmarkRetrievalMode::EngineHybridRerank
    );
    let (sampled_documents, sampled_documents_with_vectors) = if source_payload_prefilter {
        (0, 0)
    } else {
        sample_document_vector_coverage(db, uuid_index, 2048)?
    };
    let document_vector_sample_coverage_pct = if sampled_documents > 0 {
        round_pct(sampled_documents_with_vectors as f64 / sampled_documents as f64 * 100.0)
    } else {
        0.0
    };
    let mut warnings = Vec::new();
    if required_for_mode && query_vector_rows < question_count {
        warnings.push(format!(
            "hybrid retrieval needs query vectors for all questions; loaded {query_vector_rows}/{question_count}"
        ));
    }
    if required_for_mode && source_payload_prefilter && document_vector_rows_loaded == 0 {
        warnings.push(
            "source-payload prefilter is active without document vectors; dense document scoring is unavailable".to_owned(),
        );
    } else if required_for_mode && sampled_documents == 0 {
        warnings.push(
            "hybrid retrieval could not sample indexed documents for vector coverage".to_owned(),
        );
    }
    if required_for_mode
        && sampled_documents > 0
        && sampled_documents_with_vectors == 0
        && document_vector_rows_loaded == 0
    {
        warnings.push(
            "hybrid retrieval selected but no document vectors were found in the sampled DB payloads"
                .to_owned(),
        );
    }
    if required_for_mode
        && sampled_documents > 0
        && sampled_documents_with_vectors > 0
        && sampled_documents_with_vectors < sampled_documents
    {
        warnings.push(format!(
            "hybrid retrieval has partial document vector sample coverage: {sampled_documents_with_vectors}/{sampled_documents}"
        ));
    }
    let ready = if required_for_mode {
        query_vector_rows >= question_count
            && (document_vector_rows_loaded > 0
                || (sampled_documents > 0 && sampled_documents_with_vectors == sampled_documents))
    } else {
        true
    };
    Ok(VectorReadinessReport {
        required_for_mode,
        ready,
        query_vector_rows,
        document_vector_rows_loaded,
        external_document_vectors_available: document_vector_rows_loaded > 0,
        sampled_documents,
        sampled_documents_with_vectors,
        document_vector_sample_coverage_pct,
        warnings,
    })
}

fn sample_document_vector_coverage(
    db: &Database,
    uuid_index: &BTreeMap<String, String>,
    limit: usize,
) -> Result<(usize, usize), String> {
    let cell_ids = doc_id_to_cell_id(uuid_index)?;
    let mut sampled = 0usize;
    let mut with_vectors = 0usize;
    for doc_id in uuid_index.keys().take(limit) {
        sampled += 1;
        let Some(cell_id) = cell_ids.get(doc_id) else {
            continue;
        };
        if db
            .get_latest_cell(*cell_id)
            .as_deref()
            .is_some_and(payload_has_vector)
        {
            with_vectors += 1;
        }
    }
    Ok((sampled, with_vectors))
}

pub(super) fn load_external_prefilter_retrieval(
    path: Option<&Path>,
) -> Result<Option<ExternalPrefilterRetrieval>, String> {
    let Some(path) = path else {
        return Ok(None);
    };
    let rows = read_jsonl(path)?;
    let mut by_question_id = BTreeMap::<String, Vec<String>>::new();
    for (index, row) in rows.iter().enumerate() {
        let row_no = index + 1;
        let object = row
            .as_object()
            .ok_or_else(|| format!("prefilter retrieval row {row_no} must be a JSON object"))?;
        reject_prefilter_oracle_fields(object, row_no)?;
        reject_unknown_prefilter_fields(object, row_no)?;
        validate_optional_prefilter_string(object, "question", row_no, false)?;
        validate_optional_prefilter_string(object, "answer", row_no, true)?;
        let question_id = object
            .get("question_id")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                format!("prefilter retrieval row {row_no} missing non-empty question_id")
            })?
            .to_owned();
        let document_ids = clean_prefilter_document_ids(object.get("document_ids"), row_no)?;
        if !document_ids.is_empty() {
            let existing = by_question_id.entry(question_id).or_default();
            let mut seen = existing.iter().cloned().collect::<BTreeSet<_>>();
            for doc_id in document_ids {
                if seen.insert(doc_id.clone()) {
                    existing.push(doc_id);
                }
            }
        }
    }
    Ok(Some(ExternalPrefilterRetrieval {
        by_question_id,
        rows: rows.len(),
    }))
}

fn reject_prefilter_oracle_fields(
    object: &serde_json::Map<String, Value>,
    row_no: usize,
) -> Result<(), String> {
    let forbidden = ORACLE_FIELDS
        .iter()
        .copied()
        .filter(|field| object.contains_key(*field))
        .collect::<Vec<_>>();
    if forbidden.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "prefilter retrieval row {row_no} has forbidden oracle fields: {}",
            forbidden.join(", ")
        ))
    }
}

fn reject_unknown_prefilter_fields(
    object: &serde_json::Map<String, Value>,
    row_no: usize,
) -> Result<(), String> {
    let unknown = object
        .keys()
        .filter(|field| !CLEAN_PREFILTER_RETRIEVAL_FIELDS.contains(&field.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if unknown.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "prefilter retrieval row {row_no} has unsupported fields: {}",
            unknown.join(", ")
        ))
    }
}

fn validate_optional_prefilter_string(
    object: &serde_json::Map<String, Value>,
    field: &str,
    row_no: usize,
    allow_empty: bool,
) -> Result<(), String> {
    let Some(value) = object.get(field) else {
        return Ok(());
    };
    let Some(text) = value.as_str() else {
        return Err(format!(
            "prefilter retrieval row {row_no} field {field} must be a string"
        ));
    };
    if !allow_empty && text.trim().is_empty() {
        return Err(format!(
            "prefilter retrieval row {row_no} field {field} must be non-empty"
        ));
    }
    Ok(())
}

fn clean_prefilter_document_ids(
    value: Option<&Value>,
    row_no: usize,
) -> Result<Vec<String>, String> {
    let value =
        value.ok_or_else(|| format!("prefilter retrieval row {row_no} missing document_ids"))?;
    let items = value.as_array().ok_or_else(|| {
        format!("prefilter retrieval row {row_no} field document_ids must be an array")
    })?;
    let mut seen = BTreeSet::new();
    let mut document_ids = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let doc_id = item
            .as_str()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                format!(
                    "prefilter retrieval row {row_no} document_ids[{}] must be a non-empty string",
                    index
                )
            })?;
        if seen.insert(doc_id.to_owned()) {
            document_ids.push(doc_id.to_owned());
        }
    }
    Ok(document_ids)
}

pub(super) fn doc_id_from_payload(payload: &[u8]) -> Option<String> {
    String::from_utf8_lossy(payload)
        .lines()
        .find_map(|line| line.strip_prefix("doc_id=").map(str::to_owned))
}

fn process_memory_report() -> Value {
    let (rss_bytes, peak_rss_bytes) = linux_proc_status_memory_bytes().unwrap_or((0, 0));
    json!({
        "rss_bytes": rss_bytes,
        "peak_rss_bytes": peak_rss_bytes,
    })
}

fn linux_proc_status_memory_bytes() -> Option<(u64, u64)> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    let mut rss_bytes = 0;
    let mut peak_rss_bytes = 0;
    for line in status.lines() {
        if let Some(value) = line.strip_prefix("VmRSS:") {
            rss_bytes = parse_status_kib(value).unwrap_or(0);
        } else if let Some(value) = line.strip_prefix("VmHWM:") {
            peak_rss_bytes = parse_status_kib(value).unwrap_or(0);
        }
    }
    Some((rss_bytes, peak_rss_bytes.max(rss_bytes)))
}

pub(super) fn parse_status_kib(value: &str) -> Option<u64> {
    let kib = value.split_whitespace().next()?.parse::<u64>().ok()?;
    kib.checked_mul(1024)
}

pub(super) fn bench_view() -> AgentView {
    AgentView {
        agent_id: AgentId(1),
        label: None,
        readable_brains: BTreeSet::from([BrainId(1)]),
        readable_scopes: BTreeSet::from([scope_id("bench:enterprise_rag")]),
        writable_scopes: BTreeSet::new(),
        allowed_modes: BTreeSet::from([RetrievalMode::Balanced, RetrievalMode::Semantic]),
        allowed_memory_types: BTreeSet::from([MemoryType::Decision]),
        max_context_budget_tokens: 1_000,
        default_context_budget_tokens: 400,
        max_candidate_limit: 100,
        default_candidate_limit: 20,
        min_required_confidence_q16: Q16_ZERO,
        max_ttl_seconds: Some(3_600),
        allow_remember: false,
        allow_verify_fact: false,
        allow_audit_mode: false,
        require_citations_by_default: false,
        private_scope: None,
    }
}

pub(super) fn source_types(row: &Value) -> Vec<String> {
    row.get("source_types")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}
