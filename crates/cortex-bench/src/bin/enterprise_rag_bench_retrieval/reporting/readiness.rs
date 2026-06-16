use std::collections::BTreeMap;

use cortex_engine::Database;

use crate::args::BenchmarkRetrievalMode;
use crate::helpers::{doc_id_to_cell_id, round_pct};
use crate::metrics::VectorReadinessReport;
use crate::vectors::payload_has_vector;

pub(crate) fn assess_vector_readiness(
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
