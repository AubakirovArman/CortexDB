use std::collections::{BTreeMap, BTreeSet};

use cortex_aql::{default_weights, RetrievalMode};
use cortex_engine::{Database, RetrievedCell};
use serde_json::{json, Value};

use super::args::Args;
use super::helpers::{doc_id_to_cell_id, retrieval_row};
use super::logger::RunLogger;
use super::reporting::{benchmark_task, required_str, source_types};
use super::retrieval::{BenchmarkMetadataIndex, BenchmarkRetrievalIndex};

pub(crate) fn retrieve_cached_questions(
    retrieval_index: &BenchmarkRetrievalIndex,
    questions: &[Value],
    args: &Args,
    logger: &RunLogger,
) -> Result<Vec<Value>, String> {
    let mut rows = Vec::with_capacity(questions.len());
    for (index, question) in questions.iter().enumerate() {
        let qid = required_str(question, "question_id", index)?;
        let query = required_str(question, "question", index)?;
        let source_types = if args.official_clean {
            Vec::new()
        } else {
            source_types(question)
        };
        let mut seen = BTreeSet::<String>::new();
        let mut doc_ids = Vec::<Value>::new();
        for doc_id in retrieval_index.search_doc_ids(query, &source_types, args.top_k) {
            if seen.insert(doc_id.clone()) {
                doc_ids.push(Value::String(doc_id));
            }
        }
        let mut row = json!({
            "question_id": qid,
            "question": query,
            "answer": "",
            "document_ids": doc_ids,
        });
        if !args.official_clean {
            row["question_type"] = Value::String(
                question
                    .get("question_type")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_owned(),
            );
        }
        rows.push(row);
        if args.progress_every > 0
            && ((index + 1).is_multiple_of(args.progress_every)
                || questions.len() <= args.progress_every)
        {
            logger.log(&format!("retrieved {}/{}", index + 1, questions.len()));
            logger.status(
                "retrieve_questions",
                "running",
                "retrieve cached question rows",
                Some(index + 1),
                Some(questions.len()),
                &[("last_question_id", json!(qid))],
            );
        }
    }
    Ok(rows)
}

pub(crate) fn retrieve_metadata_only_questions(
    metadata_index: &BenchmarkMetadataIndex,
    questions: &[Value],
    args: &Args,
    logger: &RunLogger,
) -> Result<Option<Vec<Value>>, String> {
    if !args.official_clean || questions.is_empty() {
        return Ok(None);
    }
    let mut rows = Vec::with_capacity(questions.len());
    for (index, question) in questions.iter().enumerate() {
        let qid = required_str(question, "question_id", index)?;
        let query = required_str(question, "question", index)?;
        let doc_ids = metadata_index.search_doc_ids(query, args.top_k);
        if doc_ids.is_empty() {
            return Ok(None);
        }
        rows.push(json!({
            "question_id": qid,
            "question": query,
            "answer": "",
            "document_ids": doc_ids.into_iter().map(Value::String).collect::<Vec<_>>(),
        }));
    }
    logger.log("metadata-only retrieval satisfied all clean questions");
    logger.status(
        "retrieve_questions",
        "running",
        "metadata-only retrieval satisfied all clean questions",
        Some(rows.len()),
        Some(questions.len()),
        &[],
    );
    Ok(Some(rows))
}

pub(crate) fn retrieve_aql_questions(
    db: &Database,
    retrieval_index: &BenchmarkRetrievalIndex,
    uuid_index: &BTreeMap<String, String>,
    questions: &[Value],
    query_vectors: &BTreeMap<String, Vec<i16>>,
    args: &Args,
    logger: &RunLogger,
) -> Result<Vec<Value>, String> {
    let mut rows = Vec::with_capacity(questions.len());
    let doc_to_cell = doc_id_to_cell_id(uuid_index)?;
    for (index, question) in questions.iter().enumerate() {
        let qid = required_str(question, "question_id", index)?;
        let query = required_str(question, "question", index)?;
        let vector = query_vectors.get(qid);
        let cells = retrieval_index
            .search_doc_ids(query, &[], args.top_k.max(64))
            .iter()
            .filter_map(|doc_id| doc_to_cell.get(doc_id).copied())
            .filter_map(|cell_id| {
                db.get_latest_cell_with_descriptor(cell_id)
                    .map(|(payload, descriptor)| RetrievedCell {
                        cell_id,
                        payload,
                        descriptor,
                        captured_access_decision: None,
                    })
            })
            .collect::<Vec<_>>();
        let mode = if vector.is_some() {
            RetrievalMode::Semantic
        } else {
            RetrievalMode::Balanced
        };
        let task = benchmark_task(query, vector);
        let results = db
            .rerank_retrieved_cells_for_task(cells, &task, &default_weights(mode))
            .into_iter()
            .take(args.top_k)
            .collect::<Vec<_>>();
        rows.push(retrieval_row(
            qid,
            query,
            results.into_iter().map(|result| result.payload),
        ));
        if args.progress_every > 0
            && ((index + 1).is_multiple_of(args.progress_every)
                || questions.len() <= args.progress_every)
        {
            logger.log(&format!("retrieved {}/{}", index + 1, questions.len()));
            logger.status(
                "retrieve_questions",
                "running",
                "retrieve AQL question rows",
                Some(index + 1),
                Some(questions.len()),
                &[("last_question_id", json!(qid))],
            );
        }
    }
    Ok(rows)
}
