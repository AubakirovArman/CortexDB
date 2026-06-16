use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::Value;

use crate::constants::{CLEAN_PREFILTER_RETRIEVAL_FIELDS, ORACLE_FIELDS};
use crate::io::read_jsonl;
use crate::prefilter::ExternalPrefilterRetrieval;

pub(crate) fn required_str<'a>(
    row: &'a Value,
    field: &str,
    index: usize,
) -> Result<&'a str, String> {
    row.get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("row {} missing non-empty {field}", index + 1))
}

pub(crate) fn reject_oracle_fields(rows: &[Value]) -> Result<(), String> {
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

pub(crate) fn load_external_prefilter_retrieval(
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
