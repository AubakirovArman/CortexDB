use std::collections::BTreeMap;

use serde_json::{json, Value};

use super::support::process_memory_report;
use crate::args::{Args, BenchmarkRetrievalMode};
use crate::helpers::throughput_per_sec;
use crate::metrics::RunReportMetrics;

pub(crate) fn report_payload(
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
        "candidate_pool": args.candidate_limit,
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
