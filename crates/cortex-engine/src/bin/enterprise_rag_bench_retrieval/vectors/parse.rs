use std::collections::BTreeMap;

use cortex_engine::search::parse_vector_literal;
use serde_json::Value;

use super::DocumentVectorLookup;

pub(crate) fn vector_dot_score(query: &[i16], candidate: &[i16]) -> u64 {
    query
        .iter()
        .zip(candidate)
        .fold(0i128, |score, (left, right)| {
            score + i128::from(*left) * i128::from(*right)
        })
        .max(0)
        .min(i128::from(u64::MAX)) as u64
}

pub(crate) fn load_query_vectors(
    path: Option<&std::path::PathBuf>,
) -> Result<BTreeMap<String, Vec<i16>>, String> {
    load_id_vectors(path, "question_id")
}

pub(crate) fn load_document_vectors(
    path: Option<&std::path::PathBuf>,
) -> Result<DocumentVectorLookup, String> {
    let Some(path) = path else {
        return Ok(DocumentVectorLookup::empty());
    };
    DocumentVectorLookup::open(path)
}

pub(crate) fn load_id_vectors(
    path: Option<&std::path::PathBuf>,
    id_field: &str,
) -> Result<BTreeMap<String, Vec<i16>>, String> {
    let Some(path) = path else {
        return Ok(BTreeMap::new());
    };
    let mut vectors = BTreeMap::new();
    for (index, row) in super::super::read_jsonl(path)?.into_iter().enumerate() {
        let id = super::super::required_str(&row, id_field, index)?.to_owned();
        let vector = parse_query_vector(&row)
            .ok_or_else(|| format!("{id_field} vector row {} missing vector", index + 1))?;
        if vector.is_empty() {
            return Err(format!(
                "{} row {} has empty or invalid vector",
                path.display(),
                index + 1
            ));
        }
        vectors.insert(id, vector);
    }
    Ok(vectors)
}

pub(crate) fn parse_query_vector(row: &Value) -> Option<Vec<i16>> {
    let value = row.get("vector")?;
    if let Some(text) = value.as_str() {
        return parse_vector_literal(text).ok();
    }
    value.as_array().and_then(|items| {
        let values = items
            .iter()
            .map(json_vector_number_to_i16)
            .collect::<Option<Vec<_>>>()?;
        (!values.is_empty()).then_some(values)
    })
}

fn json_vector_number_to_i16(value: &Value) -> Option<i16> {
    if let Some(value) = value.as_i64() {
        return i16::try_from(value).ok();
    }
    let value = value.as_f64()?;
    if !value.is_finite() {
        return None;
    }
    let scaled = (value * f64::from(i16::MAX)).round();
    Some(scaled.clamp(f64::from(i16::MIN), f64::from(i16::MAX)) as i16)
}

pub(crate) fn payload_has_vector(payload: &[u8]) -> bool {
    String::from_utf8_lossy(payload).lines().any(|line| {
        line.strip_prefix("vector=")
            .is_some_and(|value| !value.trim().is_empty())
    })
}

pub(crate) fn body_vector_from_payload(payload: &[u8]) -> Option<Vec<i16>> {
    String::from_utf8_lossy(payload)
        .lines()
        .find_map(|line| parse_vector_literal(line.trim().strip_prefix("vector=")?).ok())
}
