use std::collections::{BTreeMap, BTreeSet};

use cortex_core::CellId;
use serde_json::{json, Value};

use super::args::Args;
use super::reporting::doc_id_from_payload;

pub(crate) fn document_total(args: &Args, corpus_len: usize) -> usize {
    args.max_documents
        .map(|max_documents| max_documents.min(corpus_len))
        .unwrap_or(corpus_len)
}

pub(crate) fn throughput_per_sec(units: usize, duration_ms: Option<f64>) -> f64 {
    let Some(duration_ms) = duration_ms else {
        return 0.0;
    };
    if duration_ms <= 0.0 {
        return 0.0;
    }
    round_ms(units as f64 / (duration_ms / 1000.0))
}

pub(crate) fn round_ms(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

pub(crate) fn round_pct(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

pub(crate) fn doc_id_to_cell_id(
    uuid_index: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, CellId>, String> {
    uuid_index
        .keys()
        .enumerate()
        .map(|(index, doc_id)| {
            let id = u64::try_from(index + 1).map_err(|_| "cell id overflow".to_owned())?;
            Ok((doc_id.clone(), CellId(id)))
        })
        .collect()
}

pub(crate) fn retrieval_row(
    qid: &str,
    query: &str,
    payloads: impl IntoIterator<Item = Vec<u8>>,
) -> Value {
    let mut seen = BTreeSet::new();
    let document_ids = payloads
        .into_iter()
        .filter_map(|payload| doc_id_from_payload(&payload))
        .filter(|doc_id| seen.insert(doc_id.clone()))
        .map(Value::String)
        .collect::<Vec<_>>();
    json!({
        "question_id": qid,
        "question": query,
        "answer": "",
        "document_ids": document_ids,
    })
}
